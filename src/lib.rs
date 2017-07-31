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
//! This crate provides methods to decompress DXTn-compressed image data via a wrapper around
//! the original C code used in the [Python Pillow Imaging package](https://python-pillow.org/).
//!
//! The following formats are currently supported:
//!
//! * Bc1: 565 color, 1-bit alpha (dxt1)
//! * Bc2: 565 color, 4-bit alpha (dxt3)
//! * Bc3: 565 color, 2-endpoint 8-bit interpolated alpha (dxt5)
//! * Bc4: 1-channel 8-bit via 1 BC3 alpha block
//! * Bc5: 2-channel 8-bit via 2 BC3 alpha blocks
//!
//! The following formats are not implemented:
//!
//! * Bc6: 3-channel 16-bit float
//! * Bc7: 4-channel 8-bit via everything
//!
//! Format documentation for BC1-BC5
//! http://oss.sgi.com/projects/ogl-sample/registry/EXT/texture_compression_s3tc.txt
//!
//! BC6 and BC7 are described here:
//! https://www.opengl.org/registry/specs/ARB/texture_compression_bptc.txt

extern crate libc;

mod decode;
pub mod bcndecode;

#[cfg(test)]
mod tests {

    use std::fs::File;
    use std::io::Read;
    use std::error::Error;
    use super::bcndecode::*;

    static FILE_PATH_COMPRESSED_BC3: &'static str = "testdata/images/copyright_2048_compressed_bc3.dat";
    static FILE_PATH_DECOMPRESSED_BC3: &'static str = "testdata/images/copyright_2048_decompressed_bc3.dat";
    static FILE_PATH_COMPRESSED_BC1: &'static str = "testdata/images/copyright_2048_compressed_bc1.dat";
    static FILE_PATH_DECOMPRESSED_BC1: &'static str = "testdata/images/copyright_2048_decompressed_bc1.dat";

    #[test]
    fn decode_bc3() {
        let mut compressed_file = match File::open(FILE_PATH_COMPRESSED_BC3) {
            Ok(f) => f,
            Err(err) => {
                panic!(
                    "Failed to open test data file at {}: {}",
                    FILE_PATH_COMPRESSED_BC3,
                    err.description()
                )
            }
        };

        let mut compressed_data = Vec::new();
        match compressed_file.read_to_end(&mut compressed_data) {
            Ok(_) => {
                assert_eq!(compressed_data.len(), 5592432);
            }
            Err(err) => {
                panic!(
                    "Failed to read test data at {}: {}",
                    FILE_PATH_COMPRESSED_BC3,
                    err.description()
                )
            }
        };

        let mut decompressed_file = match File::open(FILE_PATH_DECOMPRESSED_BC3) {
            Ok(f) => f,
            Err(err) => {
                panic!(
                    "Failed to open test data file at {}: {}",
                    FILE_PATH_COMPRESSED_BC3,
                    err.description()
                )
            }
        };

        let mut correct_decompressed_data = Vec::new();
        match decompressed_file.read_to_end(&mut correct_decompressed_data) {
            Ok(_) => {}
            Err(err) => {
                panic!(
                    "Failed to read test data at {}: {}",
                    FILE_PATH_DECOMPRESSED_BC3,
                    err.description()
                )
            }
        };

        let decompressed_data = match decode(
            &compressed_data,
            2048,
            2048,
            BcnEncoding::Bc3,
            BcnDecoderFormat::RGBA,
        ) {
            Ok(result) => result,
            Err(err) => {
                panic!(
                    "Failed to decompress test data at {}: {}",
                    FILE_PATH_COMPRESSED_BC3,
                    err.description()
                );
            }
        };

        assert_eq!(decompressed_data.len(), correct_decompressed_data.len());
        assert_eq!(decompressed_data, correct_decompressed_data);
    }

    #[test]
    fn decode_rust_bc3() {
        let mut compressed_file = match File::open(FILE_PATH_COMPRESSED_BC3) {
            Ok(f) => f,
            Err(err) => {
                panic!(
                    "Failed to open test data file at {}: {}",
                    FILE_PATH_COMPRESSED_BC3,
                    err.description()
                )
            }
        };

        let mut compressed_data = Vec::new();
        match compressed_file.read_to_end(&mut compressed_data) {
            Ok(_) => {
                assert_eq!(compressed_data.len(), 5592432);
            }
            Err(err) => {
                panic!(
                    "Failed to read test data at {}: {}",
                    FILE_PATH_COMPRESSED_BC3,
                    err.description()
                )
            }
        };

        let mut decompressed_file = match File::open(FILE_PATH_DECOMPRESSED_BC3) {
            Ok(f) => f,
            Err(err) => {
                panic!(
                    "Failed to open test data file at {}: {}",
                    FILE_PATH_COMPRESSED_BC3,
                    err.description()
                )
            }
        };

        let mut correct_decompressed_data = Vec::new();
        match decompressed_file.read_to_end(&mut correct_decompressed_data) {
            Ok(_) => {}
            Err(err) => {
                panic!(
                    "Failed to read test data at {}: {}",
                    FILE_PATH_DECOMPRESSED_BC3,
                    err.description()
                )
            }
        };

        let decompressed_data = match decode_rust(
            &compressed_data,
            2048,
            2048,
            BcnEncoding::Bc3,
            BcnDecoderFormat::RGBA,
        ) {
            Ok(result) => result,
            Err(err) => {
                panic!(
                    "Failed to decompress test data at {}: {}",
                    FILE_PATH_COMPRESSED_BC3,
                    err.description()
                );
            }
        };

        assert_eq!(decompressed_data.len(), correct_decompressed_data.len());
        assert_eq!(decompressed_data, correct_decompressed_data);
    }

    #[test]
    fn decode_rust_bc1() {
        let mut compressed_file = match File::open(FILE_PATH_COMPRESSED_BC1) {
            Ok(f) => f,
            Err(err) => {
                panic!(
                    "Failed to open test data file at {}: {}",
                    FILE_PATH_COMPRESSED_BC1,
                    err.description()
                )
            }
        };

        let mut compressed_data = Vec::new();
        match compressed_file.read_to_end(&mut compressed_data) {
            Ok(_) => {
                assert_eq!(compressed_data.len(), 2796216);
            }
            Err(err) => {
                panic!(
                    "Failed to read test data at {}: {}",
                    FILE_PATH_COMPRESSED_BC1,
                    err.description()
                )
            }
        };

        let mut decompressed_file = match File::open(FILE_PATH_DECOMPRESSED_BC1) {
            Ok(f) => f,
            Err(err) => {
                panic!(
                    "Failed to open test data file at {}: {}",
                    FILE_PATH_COMPRESSED_BC1,
                    err.description()
                )
            }
        };

        let mut correct_decompressed_data = Vec::new();
        match decompressed_file.read_to_end(&mut correct_decompressed_data) {
            Ok(_) => {}
            Err(err) => {
                panic!(
                    "Failed to read test data at {}: {}",
                    FILE_PATH_DECOMPRESSED_BC1,
                    err.description()
                )
            }
        };

        let decompressed_data = match decode_rust(
            &compressed_data,
            2048,
            2048,
            BcnEncoding::Bc1,
            BcnDecoderFormat::RGBA,
        ) {
            Ok(result) => result,
            Err(err) => {
                panic!(
                    "Failed to decompress test data at {}: {}",
                    FILE_PATH_COMPRESSED_BC1,
                    err.description()
                );
            }
        };

        assert_eq!(decompressed_data.len(), correct_decompressed_data.len());
        assert_eq!(decompressed_data, correct_decompressed_data);
    }
}
