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

use bcndecode::{BcnDecoderFormat, BcnEncoding, Error, ErrorKind};
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
    //  sign: bool,
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
    l: u8,
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

// TODO_rename
pub fn decode_rust_internal(
    source: &[u8],
    width: usize,
    height: usize,
    encoding: BcnEncoding,
    format: BcnDecoderFormat,
) -> Result<Vec<u8>, Error> {

    // check input data validity
    if width == 0 || height == 0 {
        return Err(Error::new(ErrorKind::InvalidImageSize));
    }

    match encoding {
        BcnEncoding::Raw | BcnEncoding::Bc7 => {
            return Err(Error::new(ErrorKind::NotImplemented));
        }
        _ => {}
    };

    // create target buffer
    let mut dst_size = 4 * width * height;

    match encoding {
        BcnEncoding::Bc4 => {
            dst_size >>= 2;
        }
        BcnEncoding::Bc6 => {
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
    $source:expr, $state:expr, $flip:expr) => {
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
        BcnEncoding::Bc6 => {
            // TODO: bc6
        }
        BcnEncoding::Bc7 => {
            //decode_loop!(decode_bc7_block, 16, RGBA, source, state, flip);
        }
        _ => {
            // TODO: RAW
        }
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
