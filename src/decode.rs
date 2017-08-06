// Copyright (c) Istvan Fehervari

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use super::{BcnDecoderFormat, BcnEncoding, Error};
use std::mem;
use std::slice;

#[derive(Default)]
struct BcnDecoderState {
    // Destination buffer, a bitmap.
    // For N=1, 2, 3, 5, 7: 4 bytes-per-pixel
    // For N=4, 1 byte-per-pixel
    // For N=6, 16 bytes-per-pixel (32-bit float)
    buffer: Vec<u8>,
    // Destination region size
    width: usize,
    height: usize,
    // Current pixel to be written
    x: usize,
    y: usize,
    // If < 0, the image will be flipped on the y-axis
    y_step: i8,
    // For bc6, data is signed numbers if true.
    sign: bool,
    // Swizzle components as necessary to match the bitmap format
    // 2 bits per component; least-significant two are index of red channel,
    // then green, blue, alpha
    swizzle: u8,
}

#[derive(Clone, Copy, Default)]
#[repr(packed)]
struct RGBA {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Clone, Copy, Default)]
#[repr(packed)]
struct LUM {
    #[allow(dead_code)]
    l: u8,
}

#[derive(Clone, Copy, Default)]
#[repr(packed)]
struct RGB32f {
    r: f32,
    g: f32,
    b: f32,
}

#[derive(Default)]
#[repr(packed)]
struct Bc1Color {
    c0: u16,
    c1: u16,
    lut: u32,
}

impl Bc1Color {
    fn load(&mut self, source: &[u8]) {
        self.c0 = load_16(source);
        self.c1 = load_16(&source[2..]);
        self.lut = load_32(&source[4..]);
    }
}

#[derive(Default)]
#[repr(packed)]
struct Bc3Alpha {
    a0: u8,
    a1: u8,
    lut: [u8; 6],
}

impl Bc3Alpha {
    fn load(&mut self, source: &[u8]) {
        self.a0 = source[0];
        self.a1 = source[1];
        self.lut.copy_from_slice(&source[2..8]);
    }
}

pub fn decode_rust(
    source: &[u8],
    width: usize,
    height: usize,
    encoding: BcnEncoding,
    format: BcnDecoderFormat,
) -> Result<Vec<u8>, Error> {

    // check input data validity
    if width == 0 || height == 0 {
        return Err(Error::InvalidImageSize);
    }

    // create target buffer
    let mut dst_size = 4 * width * height;

    match encoding {
        BcnEncoding::Bc4 => {
            dst_size >>= 2;
        }
        BcnEncoding::Bc6H => {
            dst_size <<= 2;
        }
        _ => {}
    };

    let mut state = BcnDecoderState::default();
    state.width = width;
    state.height = height;
    state.buffer = vec![0; dst_size];

    match format {
        BcnDecoderFormat::RGBA => state.swizzle = 0b11100100,
        BcnDecoderFormat::BGRA => state.swizzle = 0b11000110,
        BcnDecoderFormat::ARGB => state.swizzle = 0b10010011,
        BcnDecoderFormat::ABGR => state.swizzle = 0b00011011,
        BcnDecoderFormat::LUM => match encoding {
            BcnEncoding::Bc4 => {
                state.swizzle = 0;
            }
            _ => {
                return Err(Error::InvalidPixelFormat);
            }
        },
    }

    if ((width & 3) | (height & 3)) != 0 {
        state.y_step = -1;
        decode_bcn(&mut state, source, encoding, true);
    } else {
        state.y_step = 1;
        decode_bcn(&mut state, source, encoding, false);
    }

    Ok(state.buffer)
}

macro_rules! decode_loop {
    ( $decode_fn:ident, $block_size:expr, $T:ident,
    $source:expr, $state:expr, $flip:expr ) => {
        let mut bytes = $source.len();
        let mut source_ptr = 0;
        let y_max = $state.height;

        while bytes >= $block_size {
            let mut col = [$T::default(); 16];
            $decode_fn(&mut col, &$source[source_ptr..]);

            unsafe {
                put_block($state, to_byte_ptr(&col), mem::size_of::<$T>(), $flip);
            }

            source_ptr += $block_size;
            bytes -= $block_size;
            if $state.y >= y_max {
                break;
            }
        }
    };

    ( $decode_fn:ident, $block_size:expr, $T:ident,
    $source:expr, $state:expr, $flip:expr, $sign:expr ) => {
        let mut bytes = $source.len();
        let mut source_ptr = 0;
        let y_max = $state.height;

        while bytes >= $block_size {
            let mut col = [$T::default(); 16];
            $decode_fn(&mut col, &$source[source_ptr..], $sign);

            unsafe {
                put_block($state, to_byte_ptr(&col), mem::size_of::<$T>(), $flip);
            }

            source_ptr += $block_size;
            bytes -= $block_size;
            if $state.y >= y_max {
                break;
            }
        }
    }
}

fn decode_bcn(state: &mut BcnDecoderState, source: &[u8], encoding: BcnEncoding, flip: bool) {
    match encoding {
        BcnEncoding::Bc1 => {
            decode_loop!(decode_bc1_block, 8, RGBA, source, state, flip);
        }
        BcnEncoding::Bc2 => {
            decode_loop!(decode_bc2_block, 16, RGBA, source, state, flip);
        }
        BcnEncoding::Bc3 => {
            decode_loop!(decode_bc3_block, 16, RGBA, source, state, flip);
        }
        BcnEncoding::Bc4 => {
            decode_loop!(decode_bc4_block, 8, LUM, source, state, flip);
        }
        BcnEncoding::Bc5 => {
            decode_loop!(decode_bc5_block, 16, RGBA, source, state, flip);
        }
        BcnEncoding::Bc6H => {
            decode_loop!(
                decode_bc6h_block,
                16,
                RGB32f,
                source,
                state,
                flip,
                state.sign
            );
        }
        /*BcnEncoding::Bc7 => {
            //decode_loop!(decode_bc7_block, 16, RGBA, source, state, flip);
            unimplemented!();
        }
        _ => {
            // TODO: RAW
            unimplemented!();
        }*/
    };
}

unsafe fn to_byte_ptr<T>(a: &[T]) -> &[u8] {
    let p: *const u8 = (a as *const [T]) as *const u8;
    slice::from_raw_parts(p, mem::size_of::<T>() * a.len())
}

unsafe fn to_byte_ptr_mut<T>(a: &mut [T]) -> &mut [u8] {
    let p: *mut u8 = (a as *mut [T]) as *mut u8;
    slice::from_raw_parts_mut(p, mem::size_of::<T>() * a.len())
}

fn put_block(state: &mut BcnDecoderState, col: &[u8], block_size: usize, flip: bool) {
    let xmax = state.width;
    let ymax = state.height;

    for j in 0..4 {
        let mut y = state.y + j;
        if flip {
            if y >= state.height {
                continue;
            }
            if state.y_step < 0 {
                y = ymax - y - 1;
            }
            let dst_ptr = block_size * state.width * y;
            for i in 0..4 {
                let x = state.x + i;
                if x >= state.width {
                    continue;
                }
                swizzle_copy(
                    state.swizzle,
                    &mut state.buffer[dst_ptr + block_size * x..],
                    &col[block_size * (j * 4 + i)..],
                    block_size,
                );
            }
        } else {
            if state.y_step < 0 {
                y = ymax - y - 1;
            }
            let x = state.x;
            let mut dst_ptr = (block_size * state.width * y) + block_size * x;
            let mut src_ptr = block_size * (j * 4);
            for _ in 0..4 {
                swizzle_copy(
                    state.swizzle,
                    &mut state.buffer[dst_ptr..],
                    &col[src_ptr..],
                    block_size,
                );
                dst_ptr += block_size;
                src_ptr += block_size;
            }
        }
    }
    state.x += 4;
    if state.x >= xmax {
        state.y += 4;
        state.x = 0;
    }
}

fn load_16(source: &[u8]) -> u16 {
    (source[0] as u16) | (source[1] as u16) << 8
}

fn load_32(source: &[u8]) -> u32 {
    (source[0] as u32) | ((source[1] as u32) << 8) | ((source[2] as u32) << 16) |
        ((source[3] as u32) << 24)
}

fn swizzle_copy(swizzle: u8, dst: &mut [u8], src: &[u8], mut block_size: usize) {
    if swizzle == 0 || swizzle == 0xe4 {
        dst[0..block_size].copy_from_slice(&src[0..block_size]);
        return;
    }

    // bring sz down to size-per-component
    block_size >>= 2;
    let mut start_ptr = block_size * (((swizzle as usize) & 3));
    dst[start_ptr..start_ptr + block_size].copy_from_slice(&src[0..block_size]);

    start_ptr = block_size * (((swizzle as usize) & 0x0c) >> 2);
    dst[start_ptr..start_ptr + block_size].copy_from_slice(&src[block_size..2 * block_size]);

    start_ptr = block_size * (((swizzle as usize) & 0x30) >> 4);
    dst[start_ptr..start_ptr + block_size].copy_from_slice(&src[2 * block_size..3 * block_size]);

    start_ptr = block_size * (((swizzle as usize) & 0xc0) >> 6);
    dst[start_ptr..start_ptr + block_size].copy_from_slice(&src[3 * block_size..4 * block_size]);
}

fn decode_bc1_block(col: &mut [RGBA], source: &[u8]) {
    decode_bc1_color(col, source);
}

fn decode_bc2_block(col: &mut [RGBA], source: &[u8]) {
    decode_bc1_color(col, &source[8..]);
    for n in 0..16 {
        let bit_i: usize = n * 4;
        let by_i: usize = bit_i >> 3;
        let mut av = 0xf & (source[by_i] >> (bit_i & 7));
        av = (av << 4) | av;
        col[n].a = av;
    }
}

fn decode_bc3_block(col: &mut [RGBA], source: &[u8]) {
    decode_bc1_color(col, &source[8..]);
    unsafe {
        decode_bc3_alpha(to_byte_ptr_mut(col), source, mem::size_of::<RGBA>(), 3);
    }
}

fn decode_bc4_block(col: &mut [LUM], source: &[u8]) {
    unsafe {
        decode_bc3_alpha(to_byte_ptr_mut(col), source, mem::size_of::<LUM>(), 0);
    }
}

fn decode_bc5_block(col: &mut [RGBA], source: &[u8]) {
    unsafe {
        let dst = to_byte_ptr_mut(col);
        decode_bc3_alpha(dst, source, mem::size_of::<RGBA>(), 0);
        decode_bc3_alpha(dst, &source[8..], mem::size_of::<RGBA>(), 1);
    }
}

fn decode_bc6h_block(col: &mut [RGB32f], source: &[u8], sign: bool) {
    let mut bit = 5;
    let mut epbits = 75;
    let mut ib = 3;
    let mut mode: usize = source[0] as usize & 0x1f;
    if (mode & 3) == 0 || (mode & 3) == 1 {
        mode &= 3;
        bit = 2;
    } else if (mode & 3) == 2 {
        mode = 2 + (mode >> 2);
        epbits = 72;
    } else {
        mode = 10 + (mode >> 2);
        epbits = 60;
        ib = 4;
    }

    if mode >= 14 {
        // invalid block
        return;
    }

    let info = Bc6ModeInfo::new(mode);
    let cw = bc7_get_weights(ib);

    let numep = if info.ns == 2 { 12 } else { 6 };

    let mut endpoints: [u16; 12] = [0; 12];
    for i in 0..epbits {
        let mut di = BC6_BIT_PACKINGS[mode][i];
        let dw = di >> 4;
        di &= 15;
        endpoints[dw as usize] |= ((get_bit(source, bit + i) as usize) << di) as u16;
    }

    bit += epbits;
    let partition: u8 = get_bits(source, bit, info.pb as usize);
    bit += info.pb as usize;

    let mask: u16 = (((1 << info.epb) as usize) - 1) as u16;
    if sign {
        // sign-extend e0 if signed
        bc6_sign_extend(&mut endpoints[0], info.epb as isize);
        bc6_sign_extend(&mut endpoints[1], info.epb as isize);
        bc6_sign_extend(&mut endpoints[2], info.epb as isize);
    }
    if sign || info.tr > 0 {
        // sign-extend e1,2,3 if signed or deltas
        let mut i = 3;
        while i < numep {
            bc6_sign_extend(&mut endpoints[i + 0], info.rb as isize);
            bc6_sign_extend(&mut endpoints[i + 1], info.gb as isize);
            bc6_sign_extend(&mut endpoints[i + 2], info.bb as isize);
            i += 3;
        }
    }
    if info.tr > 0 {
        // apply deltas
        for i in 3..numep {
            endpoints[i] = ((endpoints[i] as usize + endpoints[0] as usize) & mask as usize) as u16;
        }
        if sign {
            let mut i = 3;
            while i < numep {
                bc6_sign_extend(&mut endpoints[i + 0], info.rb as isize);
                bc6_sign_extend(&mut endpoints[i + 1], info.gb as isize);
                bc6_sign_extend(&mut endpoints[i + 2], info.bb as isize);
                i += 3;
            }
        }
    }
    let mut ueps: [isize; 12] = [0; 12];
    for i in 0..numep {
        ueps[i] = bc6_unquantize(endpoints[i], info.epb as isize, sign);
    }
    for i in 0..16 {
        let s = bc7_get_subset(info.ns, partition as usize, i) * 6;
        let mut ib2 = ib as usize;
        if i == 0 {
            ib2 -= 1;
        } else if info.ns == 2 {
            if i == BC7_AI0[partition as usize] as usize {
                ib2 -= 1;
            }
        }
        let i0 = get_bits(source, bit, ib2) as usize;
        bit += ib2;

        bc6_lerp(&mut col[i], &ueps[s..], &ueps[(s + 3)..], cw[i0], sign);
    }
}

fn bc6_lerp(col: &mut RGB32f, e0: &[isize], e1: &[isize], s: u8, sign: bool) {
    let t: isize = 64 - s as isize;
    let r: isize = (e0[0] * t + e1[0] * s as isize) >> 6;
    let g: isize = (e0[1] * t + e1[1] * s as isize) >> 6;
    let b: isize = (e0[2] * t + e1[2] * s as isize) >> 6;
    col.r = bc6_finalize(r, sign);
    col.g = bc6_finalize(g, sign);
    col.b = bc6_finalize(b, sign);
}

fn bc6_finalize(v: isize, sign: bool) -> f32 {
    if sign {
        if v < 0 {
            let _v = ((-v) * 31) / 32;
            return half_to_float((0x8000 | _v) as u16);
        } else {
            return half_to_float(((v * 31) / 32) as u16);
        }
    } else {
        return half_to_float(((v * 31) / 64) as u16);
    }
}

#[repr(C)]
#[derive(Default)]
struct FloatUnion {
    data: u32,
}

impl FloatUnion {
    unsafe fn as_f32_mut(&mut self) -> &mut f32 {
        let p = self as *mut _ as *mut f32;
        &mut *p
    }
    unsafe fn as_f32(&self) -> &f32 {
        let p = self as *const _ as *const f32;
        &*p
    }
    unsafe fn as_u32_mut(&mut self) -> &mut u32 {
        let p = self as *mut _ as *mut u32;
        &mut *p
    }
    unsafe fn as_u32(&self) -> &u32 {
        let p = self as *const _ as *const u32;
        &*p
    }
}

fn half_to_float(h: u16) -> f32 {
    unsafe {
        // https://gist.github.com/rygorous/2144712
        let mut o = FloatUnion::default();
        let mut m = FloatUnion::default();

        *m.as_u32_mut() = 0x77800000;
        *o.as_u32_mut() = (h as u32 & 0x7fff) << 13;
        *o.as_f32_mut() = *o.as_f32() * *m.as_f32(); // o.f *= m.f;
        *m.as_u32_mut() = 0x47800000;
        if o.as_f32() >= m.as_f32() {
            *o.as_u32_mut() = *o.as_u32() | (255 << 23);
        }
        *o.as_u32_mut() = *o.as_u32() | ((h as u32 & 0x8000) << 16);
        return *o.as_f32();
    }
}

#[derive(Default)]
struct Bc6ModeInfo {
    ns: u8,  /* number of subsets (also called regions) */
    tr: u8,  /* whether endpoints are delta-compressed */
    pb: u8,  /* partition bits */
    epb: u8, /* endpoint bits */
    rb: u8,  /* red bits (delta) */
    gb: u8,  /* green bits (delta) */
    bb: u8,  /* blue bits (delta) */
}

impl Bc6ModeInfo {
    fn new(mode: usize) -> Self {
        match mode {
            0 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 10,
                rb: 5,
                gb: 5,
                bb: 5,
            },
            1 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 7,
                rb: 6,
                gb: 6,
                bb: 6,
            },
            2 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 11,
                rb: 5,
                gb: 4,
                bb: 4,
            },
            3 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 11,
                rb: 4,
                gb: 5,
                bb: 4,
            },
            4 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 11,
                rb: 4,
                gb: 4,
                bb: 5,
            },
            5 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 9,
                rb: 5,
                gb: 5,
                bb: 5,
            },
            6 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 8,
                rb: 6,
                gb: 5,
                bb: 5,
            },
            7 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 8,
                rb: 5,
                gb: 6,
                bb: 5,
            },
            8 => Bc6ModeInfo {
                ns: 2,
                tr: 1,
                pb: 5,
                epb: 8,
                rb: 5,
                gb: 5,
                bb: 6,
            },
            9 => Bc6ModeInfo {
                ns: 2,
                tr: 0,
                pb: 5,
                epb: 6,
                rb: 6,
                gb: 6,
                bb: 6,
            },
            10 => Bc6ModeInfo {
                ns: 1,
                tr: 0,
                pb: 0,
                epb: 10,
                rb: 10,
                gb: 10,
                bb: 10,
            },
            11 => Bc6ModeInfo {
                ns: 1,
                tr: 1,
                pb: 0,
                epb: 11,
                rb: 9,
                gb: 9,
                bb: 9,
            },
            12 => Bc6ModeInfo {
                ns: 1,
                tr: 1,
                pb: 0,
                epb: 12,
                rb: 8,
                gb: 8,
                bb: 8,
            },
            13 => Bc6ModeInfo {
                ns: 1,
                tr: 1,
                pb: 0,
                epb: 16,
                rb: 4,
                gb: 4,
                bb: 4,
            },
            _ => Bc6ModeInfo::default(),
        }
    }
}

static BC7_WEIGHTS2: [u8; 4] = [0, 21, 43, 64];
static BC7_WEIGHTS3: [u8; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
static BC7_WEIGHTS4: [u8; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

fn bc7_get_weights(n: u8) -> &'static [u8] {
    if n == 2 {
        return &BC7_WEIGHTS2;
    }
    if n == 3 {
        return &BC7_WEIGHTS3;
    }
    return &BC7_WEIGHTS4;
}

static BC6_BIT_PACKINGS: [[u8; 75]; 14] = [
    [
        116,
        132,
        176,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        48,
        49,
        50,
        51,
        52,
        164,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        68,
        172,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        84,
        173,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        100,
        174,
        144,
        145,
        146,
        147,
        148,
        175,
    ],
    [
        117,
        164,
        165,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        172,
        173,
        132,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        133,
        174,
        116,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        175,
        177,
        176,
        48,
        49,
        50,
        51,
        52,
        53,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        68,
        69,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        84,
        85,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        100,
        101,
        144,
        145,
        146,
        147,
        148,
        149,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        48,
        49,
        50,
        51,
        52,
        10,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        26,
        172,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        42,
        173,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        100,
        174,
        144,
        145,
        146,
        147,
        148,
        175,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        48,
        49,
        50,
        51,
        10,
        164,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        68,
        26,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        42,
        173,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        172,
        174,
        144,
        145,
        146,
        147,
        116,
        175,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        48,
        49,
        50,
        51,
        10,
        132,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        26,
        172,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        84,
        42,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        173,
        174,
        144,
        145,
        146,
        147,
        176,
        175,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        132,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        116,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        176,
        48,
        49,
        50,
        51,
        52,
        164,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        68,
        172,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        84,
        173,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        100,
        174,
        144,
        145,
        146,
        147,
        148,
        175,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        164,
        132,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        174,
        116,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        175,
        176,
        48,
        49,
        50,
        51,
        52,
        53,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        68,
        172,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        84,
        173,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        100,
        101,
        144,
        145,
        146,
        147,
        148,
        149,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        172,
        132,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        117,
        116,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        165,
        176,
        48,
        49,
        50,
        51,
        52,
        164,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        68,
        69,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        84,
        173,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        100,
        174,
        144,
        145,
        146,
        147,
        148,
        175,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        173,
        132,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        133,
        116,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        177,
        176,
        48,
        49,
        50,
        51,
        52,
        164,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        68,
        172,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        84,
        85,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        100,
        174,
        144,
        145,
        146,
        147,
        148,
        175,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        164,
        172,
        173,
        132,
        16,
        17,
        18,
        19,
        20,
        21,
        117,
        133,
        174,
        116,
        32,
        33,
        34,
        35,
        36,
        37,
        165,
        175,
        177,
        176,
        48,
        49,
        50,
        51,
        52,
        53,
        112,
        113,
        114,
        115,
        64,
        65,
        66,
        67,
        68,
        69,
        160,
        161,
        162,
        163,
        80,
        81,
        82,
        83,
        84,
        85,
        128,
        129,
        130,
        131,
        96,
        97,
        98,
        99,
        100,
        101,
        144,
        145,
        146,
        147,
        148,
        149,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        48,
        49,
        50,
        51,
        52,
        53,
        54,
        55,
        56,
        57,
        64,
        65,
        66,
        67,
        68,
        69,
        70,
        71,
        72,
        73,
        80,
        81,
        82,
        83,
        84,
        85,
        86,
        87,
        88,
        89,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        48,
        49,
        50,
        51,
        52,
        53,
        54,
        55,
        56,
        10,
        64,
        65,
        66,
        67,
        68,
        69,
        70,
        71,
        72,
        26,
        80,
        81,
        82,
        83,
        84,
        85,
        86,
        87,
        88,
        42,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        48,
        49,
        50,
        51,
        52,
        53,
        54,
        55,
        11,
        10,
        64,
        65,
        66,
        67,
        68,
        69,
        70,
        71,
        27,
        26,
        80,
        81,
        82,
        83,
        84,
        85,
        86,
        87,
        43,
        42,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ],
    [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        32,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
        48,
        49,
        50,
        51,
        15,
        14,
        13,
        12,
        11,
        10,
        64,
        65,
        66,
        67,
        31,
        30,
        29,
        28,
        27,
        26,
        80,
        81,
        82,
        83,
        47,
        46,
        45,
        44,
        43,
        42,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ],
];

/* Subset indices:
 Table.P2, 1 bit per index */
static BC7_SI2: [u16; 64] = [
    0xcccc,
    0x8888,
    0xeeee,
    0xecc8,
    0xc880,
    0xfeec,
    0xfec8,
    0xec80,
    0xc800,
    0xffec,
    0xfe80,
    0xe800,
    0xffe8,
    0xff00,
    0xfff0,
    0xf000,
    0xf710,
    0x008e,
    0x7100,
    0x08ce,
    0x008c,
    0x7310,
    0x3100,
    0x8cce,
    0x088c,
    0x3110,
    0x6666,
    0x366c,
    0x17e8,
    0x0ff0,
    0x718e,
    0x399c,
    0xaaaa,
    0xf0f0,
    0x5a5a,
    0x33cc,
    0x3c3c,
    0x55aa,
    0x9696,
    0xa55a,
    0x73ce,
    0x13c8,
    0x324c,
    0x3bdc,
    0x6996,
    0xc33c,
    0x9966,
    0x0660,
    0x0272,
    0x04e4,
    0x4e40,
    0x2720,
    0xc936,
    0x936c,
    0x39c6,
    0x639c,
    0x9336,
    0x9cc6,
    0x817e,
    0xe718,
    0xccf0,
    0x0fcc,
    0x7744,
    0xee22,
];

/* Table.P3, 2 bits per index */
static BC7_SI3: [u32; 4 * 16] = [
    0xaa685050,
    0x6a5a5040,
    0x5a5a4200,
    0x5450a0a8,
    0xa5a50000,
    0xa0a05050,
    0x5555a0a0,
    0x5a5a5050,
    0xaa550000,
    0xaa555500,
    0xaaaa5500,
    0x90909090,
    0x94949494,
    0xa4a4a4a4,
    0xa9a59450,
    0x2a0a4250,
    0xa5945040,
    0x0a425054,
    0xa5a5a500,
    0x55a0a0a0,
    0xa8a85454,
    0x6a6a4040,
    0xa4a45000,
    0x1a1a0500,
    0x0050a4a4,
    0xaaa59090,
    0x14696914,
    0x69691400,
    0xa08585a0,
    0xaa821414,
    0x50a4a450,
    0x6a5a0200,
    0xa9a58000,
    0x5090a0a8,
    0xa8a09050,
    0x24242424,
    0x00aa5500,
    0x24924924,
    0x24499224,
    0x50a50a50,
    0x500aa550,
    0xaaaa4444,
    0x66660000,
    0xa5a0a5a0,
    0x50a050a0,
    0x69286928,
    0x44aaaa44,
    0x66666600,
    0xaa444444,
    0x54a854a8,
    0x95809580,
    0x96969600,
    0xa85454a8,
    0x80959580,
    0xaa141414,
    0x96960000,
    0xaaaa1414,
    0xa05050a0,
    0xa0a5a5a0,
    0x96000000,
    0x40804080,
    0xa9a8a9a8,
    0xaaaaaa44,
    0x2a4a5254,
];

/* Anchor indices:
 Table.A2 */
static BC7_AI0: [u8; 64] = [
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    2,
    8,
    2,
    2,
    8,
    8,
    15,
    2,
    8,
    2,
    2,
    8,
    8,
    2,
    2,
    15,
    15,
    6,
    8,
    2,
    8,
    15,
    15,
    2,
    8,
    2,
    2,
    2,
    15,
    15,
    6,
    6,
    2,
    6,
    8,
    15,
    15,
    2,
    2,
    15,
    15,
    15,
    15,
    15,
    2,
    2,
    15,
];

/* Table.A3a */
static BC7_AI1: [u8; 64] = [
    3,
    3,
    15,
    15,
    8,
    3,
    15,
    15,
    8,
    8,
    6,
    6,
    6,
    5,
    3,
    3,
    3,
    3,
    8,
    15,
    3,
    3,
    6,
    10,
    5,
    8,
    8,
    6,
    8,
    5,
    15,
    15,
    8,
    15,
    3,
    5,
    6,
    10,
    8,
    15,
    15,
    3,
    15,
    5,
    15,
    15,
    15,
    15,
    3,
    15,
    5,
    5,
    5,
    8,
    5,
    10,
    5,
    10,
    8,
    13,
    15,
    12,
    3,
    3,
];

/* Table.A3b */
static BC7_AI2: [u8; 64] = [
    15,
    8,
    8,
    3,
    15,
    15,
    3,
    8,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    8,
    15,
    8,
    15,
    3,
    15,
    8,
    15,
    8,
    3,
    15,
    6,
    10,
    15,
    15,
    10,
    8,
    15,
    3,
    15,
    10,
    10,
    8,
    9,
    10,
    6,
    15,
    8,
    15,
    3,
    6,
    6,
    8,
    15,
    3,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    15,
    3,
    15,
    15,
    8,
];

fn get_bit(src: &[u8], bit: usize) -> u8 {
    let by = bit >> 3;
    let bit_m = bit & 7;
    return (src[by] >> bit_m) & 1;
}

fn get_bits(src: &[u8], bit: usize, count: usize) -> u8 {
    let v: u8;
    let by = bit >> 3;
    let _bit = bit & 7;
    if count == 0 {
        return 0;
    }
    if _bit + count <= 8 {
        v = (src[by] >> _bit) & ((1 << count) - 1);
    } else {
        let x = src[by] as usize | ((src[by + 1] as usize) << 8);
        v = ((x >> _bit) & ((1 << count) - 1)) as u8;
    }
    return v;
}

fn bc6_sign_extend(v: &mut u16, prec: isize) {
    let mut x = v.clone() as isize;
    if x & (1 << (prec - 1)) > 0 {
        x |= -1 << prec;
    }
    *v = x as u16;
}

fn bc6_unquantize(v: u16, prec: isize, sign: bool) -> isize {
    if !sign {
        let x = v as isize;
        if prec >= 15 {
            return x;
        }

        if x == 0 {
            return 0;
        }

        if x == ((1 << prec) - 1) {
            return 0xffff;
        }
        return ((x << 15) + 0x4000) >> (prec - 1);
    } else {
        let mut x = v as isize;
        if prec >= 16 {
            return x;
        }
        let mut s = 0;
        if x < 0 {
            s = 1;
            x = -x;
        }

        if x != 0 {
            if x >= ((1 << (prec - 1)) - 1) {
                x = 0x7fff;
            } else {
                x = ((x << 15) + 0x4000) >> (prec - 1);
            }
        }

        if s != 0 {
            return -x;
        }
        return x;
    }
}

fn bc7_get_subset(ns: u8, partition: usize, n: usize) -> usize {
    if ns == 2 {
        return 1 & (BC7_SI2[partition] as usize >> n);
    }
    if ns == 3 {
        return 3 & (BC7_SI3[partition] as usize >> (2 * n));
    }
    return 0;
}

fn decode_565(x: u16) -> RGBA {
    let mut r: isize = ((x & 0xf800) >> 8) as isize;
    r |= r >> 5;

    let mut g: isize = ((x & 0x7e0) >> 3) as isize;
    g |= g >> 6;

    let mut b: isize = ((x & 0x1f) << 3) as isize;
    b |= b >> 5;

    return RGBA {
        r: r as u8,
        g: g as u8,
        b: b as u8,
        a: 0xff,
    };
}

fn decode_bc1_color(dst: &mut [RGBA], source: &[u8]) {
    let mut col = Bc1Color::default();
    let mut p = [RGBA::default(); 4];

    col.load(source);

    p[0] = decode_565(col.c0);
    let r0: u16 = p[0].r as u16;
    let g0: u16 = p[0].g as u16;
    let b0: u16 = p[0].b as u16;

    p[1] = decode_565(col.c1);
    let r1: u16 = p[1].r as u16;
    let g1: u16 = p[1].g as u16;
    let b1: u16 = p[1].b as u16;

    if col.c0 > col.c1 {
        p[2].r = ((2 * r0 + 1 * r1) / 3) as u8;
        p[2].g = ((2 * g0 + 1 * g1) / 3) as u8;
        p[2].b = ((2 * b0 + 1 * b1) / 3) as u8;
        p[2].a = 0xff;
        p[3].r = ((1 * r0 + 2 * r1) / 3) as u8;
        p[3].g = ((1 * g0 + 2 * g1) / 3) as u8;
        p[3].b = ((1 * b0 + 2 * b1) / 3) as u8;
        p[3].a = 0xff;
    } else {
        p[2].r = ((r0 + r1) / 2) as u8;
        p[2].g = ((g0 + g1) / 2) as u8;
        p[2].b = ((b0 + b1) / 2) as u8;
        p[2].a = 0xff;
        p[3].r = 0;
        p[3].g = 0;
        p[3].b = 0;
        p[3].a = 0;
    }

    for n in 0..16 {
        let cw: usize = (3 & (col.lut >> (2 * n))) as usize;
        dst[n] = p[cw];
    }
}

fn decode_bc3_alpha(dst: &mut [u8], source: &[u8], stride: usize, o: usize) {
    let mut b = Bc3Alpha::default();
    b.load(source);

    let a0: u16 = b.a0 as u16;
    let a1: u16 = b.a1 as u16;
    let mut a: [u8; 8] = [0; 8];
    a[0] = a0 as u8;
    a[1] = a1 as u8;
    if a0 > a1 {
        a[2] = ((6 * a0 + 1 * a1) / 7) as u8;
        a[3] = ((5 * a0 + 2 * a1) / 7) as u8;
        a[4] = ((4 * a0 + 3 * a1) / 7) as u8;
        a[5] = ((3 * a0 + 4 * a1) / 7) as u8;
        a[6] = ((2 * a0 + 5 * a1) / 7) as u8;
        a[7] = ((1 * a0 + 6 * a1) / 7) as u8;
    } else {
        a[2] = ((4 * a0 + 1 * a1) / 5) as u8;
        a[3] = ((3 * a0 + 2 * a1) / 5) as u8;
        a[4] = ((2 * a0 + 3 * a1) / 5) as u8;
        a[5] = ((1 * a0 + 4 * a1) / 5) as u8;
        a[6] = 0;
        a[7] = 0xff;
    }
    let lut: usize = (b.lut[0] as usize) | ((b.lut[1] as usize) << 8) | ((b.lut[2] as usize) << 16);
    for n in 0..8 {
        let aw: usize = 7 & (lut >> (3 * n));
        dst[stride * n + o] = a[aw];
    }
    let lut: usize = (b.lut[3] as usize) | ((b.lut[4] as usize) << 8) | ((b.lut[5] as usize) << 16);
    for n in 0..8 {
        let aw: usize = 7 & (lut >> (3 * n));
        dst[stride * (8 + n) + o] = a[aw];
    }
}
