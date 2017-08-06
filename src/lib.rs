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

//! # Decoder for DXTn-compressed data
//!
//! This crate provides methods to decompress DXTn-compressed image data.
//!
//! The following formats are currently supported:
//!
//! * Bc1: 565 color, 1-bit alpha (dxt1)
//! * Bc2: 565 color, 4-bit alpha (dxt3)
//! * Bc3: 565 color, 2-endpoint 8-bit interpolated alpha (dxt5)
//! * Bc4: 1-channel 8-bit via 1 BC3 alpha block
//! * Bc5: 2-channel 8-bit via 2 BC3 alpha blocks
//! * Bc6: 3-channel 16-bit float
//!
//! The following formats are not implemented:
//!
//! * Bc7: 4-channel 8-bit via everything
//!
//! Format documentation for BC1-BC5
//! http://oss.sgi.com/projects/ogl-sample/registry/EXT/texture_compression_s3tc.txt
//!
//! BC6 and BC7 are described here:
//! https://www.opengl.org/registry/specs/ARB/texture_compression_bptc.txt
//!
//! The decompression code was based on the original C code used in the
//! [Python Pillow Imaging package](https://python-pillow.org/)

#[cfg(test)]
extern crate libc;

use std::error;
use std::fmt;
use std::io;

mod decode;

#[cfg(test)]
mod tests;

/// The error type for all bcn decoding operations.
#[derive(Debug)]
pub enum Error {
    /// Decoding failed due to incorrect source data.
    ImageDecodingError,
    /// Size of the image is invalid.
    InvalidImageSize,
    /// Requested feature is not implemented.
    FeatureNotImplemented,
    /// Pixel format is invalid for the given decoding
    InvalidPixelFormat,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            &Error::ImageDecodingError => "Failed to decode image",
            &Error::InvalidImageSize => "Size of the image is invalid",
            &Error::FeatureNotImplemented => "Feature is not implemented",
            &Error::InvalidPixelFormat => "Pixel format is invalid for the given decoding",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", error::Error::description(self))
    }
}

impl From<Error> for io::Error {
    fn from(error: Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, error)
    }
}

/// Encoding type of the source data.
#[derive(Copy, Clone)]
pub enum BcnEncoding {
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
    /// BC6: Three color channels (16 bits:16 bits:16 bits) in "half" floating point
    /// (16 bit value that consists of an optional sign bit, a 5 bit biased exponent,
    /// and a 10 or 11 bit mantissa.)
    Bc6H = 6,
    // BC7: Three color channels (4 to 7 bits per channel) with 0 to 8 bits of alpha
    // (not implemented)
    //Bc7 = 7,
}

/// Specifies the pixel format of the output data
#[derive(Copy, Clone)]
pub enum BcnDecoderFormat {
    RGBA = 1,
    BGRA = 2,
    ARGB = 3,
    ABGR = 4,
    /// Format only used for BC4 decompression
    LUM = 5,
}

/// Decodes the given BCN encoded image data.
/// On success, the decoded data as a byte vector is returned.
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
/// use bcndecode;
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
    decode::decode_rust(source, width, height, encoding, format)
}
