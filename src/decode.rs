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

#[derive(Default)]
struct BcnDecoderState {
    // Destination buffer, a bitmap.
    // For N=1, 2, 3, 5, 7: 4 bytes-per-pixel
    // For N=4, 1 byte-per-pixel
    // For N=6, 16 bytes-per-pixel (32-bit float)
    buffer: Vec<u8>,
    // Destination region offset
    x_off: isize,
    y_off: isize,
    // Destination region size
    width: usize,
    height: usize,
    // Current pixel to be written
    x: isize,
    y: isize,
    // If < 0, the image will be flipped on the y-axis
    y_step: i8,
    // For bc6, data is signed numbers if true.
    sign: bool,
    // Swizzle components as necessary to match the bitmap format
    // 2 bits per component; least-significant two are index of red channel,
    // then green, blue, alpha
    swizzle: u8,
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

    let mut flip = false;

    if ((width & 3) | (height & 3)) != 0 {
        state.y_step = -1;
        decode_bcn(&mut state, source, encoding, true)?;
    } else {
        state.y_step = 1;
        decode_bcn(&mut state, source, encoding, false)?;
    }

    Ok(state.buffer)
}

fn decode_bcn(
    state: &mut BcnDecoderState,
    source: &[u8],
    encoding: BcnEncoding,
    flip: bool,
) -> Result<(), Error> {

    let y_max: isize = state.height as isize + state.y_off;
    //const uint8_t *ptr = src;

    match encoding {
        BcnEncoding::Bc1 => decode_loop<RGBA>(8),
        BcnEncoding::Bc2 => decode_loop<RGBA>(16),
        BcnEncoding::Bc3 => decode_loop<RGBA>(16),
        BcnEncoding::Bc4 => decode_loop<LUM>(8),
        BcnEncoding::Bc5 => decode_loop<RGBA>(16),
        BcnEncoding::Bc6 => {

        },
        _ => {
            // TODO: error
        },
    };

    Ok(())
}

fn decode_loop<T>(b: i32) {

}

struct RGBA {

}

struct LUM {

}
