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

use libc::{uint8_t, c_int};
use std::result::Result;
use std::error;
use std::io;
use std::fmt;
use decode::decode_rust_internal;

/// The error type for all bcn decoding operations.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

/// A list specifing the category of the error.
///
/// It is used with the [`bcndecode::Error`] type.
///
/// [`bcndecode::Error`]: struct.Error.html
#[derive(Debug)]
pub enum ErrorKind {
    /// Decoding failed due to incorrect source data.
    ImageDecodingError,
    /// Size of the image is invalid
    InvalidImageSize,
    NotImplemented,
}

impl ErrorKind {
    fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::ImageDecodingError => "Failed to decode image",
            ErrorKind::InvalidImageSize => "Size of the image is invalid",
            ErrorKind::NotImplemented => "Feature is not implemented",
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.kind.as_str()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.kind.as_str())
    }
}

impl From<Error> for io::Error {
    fn from(error: Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, error)
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Error {
        Error { kind: kind }
    }
}

extern "C" {
    fn BcnDecode(
        dst: *mut uint8_t,
        dst_size: c_int,
        src: *const uint8_t,
        src_size: c_int,
        width: c_int,
        height: c_int,
        N: c_int,
        dst_format: c_int,
        flip: c_int,
    ) -> c_int;
}

/// Specifies the pixel format of the output data
#[derive(Copy, Clone)]
pub enum BcnDecoderFormat {
    RGBA = 1,
    BGRA = 2,
    ARGB = 3,
    ABGR = 4,
}

/// Encoding type of the source data.
#[derive(Copy, Clone)]
pub enum BcnEncoding {
    /// Raw data (will not be decoded)
    Raw = 0, // TODO: consider removing it
    /// BC1: 565 color, 1-bit alpha (dxt1)
    Bc1 = 1,
    /// BC2: 565 color, 4-bit alpha (dxt3)
    Bc2 = 2, 
    /// BC3: 565 color, 2-endpoint 8-bit interpolated alpha (dxt5)
    Bc3 = 3,
    /// BC4: 1-channel 8-bit via 1 BC3 alpha block
    Bc4 = 4,
    /// BC5: 2-channel 8-bit via 2 BC3 alpha blocks
    Bc5 = 5,
    /// BC6: 3-channel 16-bit float (not implemented)
    Bc6 = 6,
    /// BC7: 4-channel 8-bit via everything (not implemented)
    Bc7 = 7,
}

/// Decodes the given BCN encoded image data. On success, the decoded data as a byte vector is returned.
///
/// # Arguments
///
/// * `source`    - A byte slice that holds the data of the compressed image
/// * `width`     - Width of the encoded image in pixels
/// * `height`    - Width of the encoded image in pixels
/// * `encoding`  - Encoding type of the image.
/// * `format`    - Image format.
///
/// # Errors
///
/// This function will return an error if the data cannot be decoded with the given parameters.
///
/// # Examples
///
/// ```
/// use bcndecode::bcndecode;
/// use std::fs::File;
/// use std::io::Read;
///    
/// # use std::io; 
/// # fn foo() -> io::Result<()> {
/// let mut compressed_file = File::open("testdata/images/copyright_2048_compressed.dat")?;
/// let mut compressed_data = Vec::new();
/// 
/// compressed_file.read_to_end(&mut compressed_data)?;
/// 
/// let decompressed_data = bcndecode::decode(
///     &compressed_data,
///     2048,
///     2048,
///     bcndecode::BcnEncoding::Bc3,
///     bcndecode::BcnDecoderFormat::RGBA,
/// )?; 
///
/// # Ok(())
/// # }
/// ```
pub fn decode(
    source: &[u8],
    width: usize,
    height: usize,
    encoding: BcnEncoding,
    format: BcnDecoderFormat,
) -> Result<Vec<u8>, Error> {
    let mut dst_size = (4 * width * height) as usize;

    match encoding {
        BcnEncoding::Bc4 => {
            dst_size >>= 2;
        }
        BcnEncoding::Bc6 => {
            dst_size <<= 2;
        }
        _ => {}
    };

    let mut dst: Vec<u8> = vec![0; dst_size];

    let mut flip: c_int = 0;

    if ((width & 3) | (height & 3)) != 0 {
        flip = 1;
    }

    unsafe {
        let data_read = BcnDecode(
            dst.as_mut_ptr(),
            dst.len() as c_int,
            source.as_ptr(),
            source.len() as c_int,
            width as c_int,
            height as c_int,
            encoding as c_int,
            format as c_int,
            flip,
        );
        if data_read < 0 {
            return Err(Error::new(ErrorKind::ImageDecodingError));
        }
    }

    Ok(dst)
}

pub fn decode_rust(
    source: &[u8],
    width: usize,
    height: usize,
    encoding: BcnEncoding,
    format: BcnDecoderFormat,
) -> Result<Vec<u8>, Error> {
    decode_rust_internal(source, width, height, encoding, format)
}
