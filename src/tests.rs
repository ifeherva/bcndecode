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

use libc::{c_int, uint8_t};
use std::fs::File;
use std::io::Read;
use std::error;
use super::decode::*;

use BcnDecoderFormat;
use BcnEncoding;
use super::Error;

static TEST_DATA_PATH: &'static str = "testdata/images/";

static COMPRESSED_BC1: &'static str = "copyright_2048_compressed_bc1.dat";
static COMPRESSED_BC3: &'static str = "copyright_2048_compressed_bc3.dat";
static COMPRESSED_BC4: &'static str = "copyright_2048_compressed_bc4.dat";
static COMPRESSED_BC5: &'static str = "copyright_2048_compressed_bc5.dat";
static COMPRESSED_BC6H: &'static str = "copyright_2048_compressed_bc6h.dat";

static DECOMPRESSED_BC1: &'static str = "copyright_2048_decompressed_bc1.dat";
static DECOMPRESSED_BC3: &'static str = "copyright_2048_decompressed_bc3.dat";
static DECOMPRESSED_BC4: &'static str = "copyright_2048_decompressed_bc4.dat";
static DECOMPRESSED_BC5: &'static str = "copyright_2048_decompressed_bc5.dat";

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

fn decode_c(
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
        BcnEncoding::Bc6H => {
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
            return Err(Error::ImageDecodingError);
        }
    }

    Ok(dst)
}

/// Compares the decoding output of the C and rust implementation
fn compare_decode(
    compressed_file_path: &str,
    width: usize,
    height: usize,
    encoding: BcnEncoding,
    format: BcnDecoderFormat,
) {
    let file_path = format!("{}{}", TEST_DATA_PATH, compressed_file_path);
    let mut compressed_file = match File::open(&file_path) {
        Ok(f) => f,
        Err(err) => panic!(
            "Failed to open test data file at {}: {}",
            file_path,
            error::Error::description(&err)
        ),
    };

    let mut compressed_data = Vec::new();
    match compressed_file.read_to_end(&mut compressed_data) {
        Ok(_) => {}
        Err(err) => panic!(
            "Failed to read test data at {}: {}",
            file_path,
            error::Error::description(&err)
        ),
    };

    let decompressed_data_c = match decode_c(&compressed_data, width, height, encoding, format) {
        Ok(result) => result,
        Err(err) => {
            panic!(
                "Failed to decompress test data with c decoder at {}: {}",
                file_path,
                error::Error::description(&err)
            );
        }
    };

    let decompressed_data_rust =
        match decode_rust(&compressed_data, width, height, encoding, format) {
            Ok(result) => result,
            Err(err) => {
                panic!(
                    "Failed to decompress test data with rust decoder at {}: {}",
                    file_path,
                    error::Error::description(&err)
                );
            }
        };

    assert_eq!(decompressed_data_c.len(), decompressed_data_rust.len());
    assert_eq!(decompressed_data_c, decompressed_data_rust);
}

fn test_decode_rust(
    compressed_path: &str,
    compressed_len: usize,
    decompressed_path: &str,
    width: usize,
    height: usize,
    encoding: BcnEncoding,
    format: BcnDecoderFormat,
) {
    let compressed_file_path = format!("{}{}", TEST_DATA_PATH, compressed_path);
    let mut compressed_file = match File::open(&compressed_file_path) {
        Ok(f) => f,
        Err(err) => panic!(
            "Failed to open test data file at {}: {}",
            compressed_file_path,
            error::Error::description(&err)
        ),
    };

    let mut compressed_data = Vec::new();
    match compressed_file.read_to_end(&mut compressed_data) {
        Ok(_) => {
            assert_eq!(compressed_data.len(), compressed_len);
        }
        Err(err) => panic!(
            "Failed to read test data at {}: {}",
            compressed_file_path,
            error::Error::description(&err)
        ),
    };

    let decompressed_file_path = format!("{}{}", TEST_DATA_PATH, decompressed_path);
    let mut decompressed_file = match File::open(&decompressed_file_path) {
        Ok(f) => f,
        Err(err) => panic!(
            "Failed to open test data file at {}: {}",
            decompressed_file_path,
            error::Error::description(&err)
        ),
    };

    let mut correct_decompressed_data = Vec::new();
    match decompressed_file.read_to_end(&mut correct_decompressed_data) {
        Ok(_) => {}
        Err(err) => panic!(
            "Failed to read test data at {}: {}",
            decompressed_file_path,
            error::Error::description(&err)
        ),
    };

    let decompressed_data = match decode_rust(&compressed_data, width, height, encoding, format) {
        Ok(result) => result,
        Err(err) => {
            panic!(
                "Failed to decompress test data at {}: {}",
                compressed_file_path,
                error::Error::description(&err)
            );
        }
    };

    assert_eq!(decompressed_data.len(), correct_decompressed_data.len());
    assert_eq!(decompressed_data, correct_decompressed_data);
}

#[test]
fn decode_rust_bc1() {
    test_decode_rust(
        COMPRESSED_BC1,
        2796216,
        DECOMPRESSED_BC1,
        2048,
        2048,
        BcnEncoding::Bc1,
        BcnDecoderFormat::RGBA,
    );
}

#[test]
fn decode_rust_bc3() {
    test_decode_rust(
        COMPRESSED_BC3,
        5592432,
        DECOMPRESSED_BC3,
        2048,
        2048,
        BcnEncoding::Bc3,
        BcnDecoderFormat::RGBA,
    );
}

#[test]
fn decode_rust_bc4() {
    test_decode_rust(
        COMPRESSED_BC4,
        2796216,
        DECOMPRESSED_BC4,
        2048,
        2048,
        BcnEncoding::Bc4,
        BcnDecoderFormat::LUM,
    );
}

#[test]
fn decode_rust_bc5() {
    test_decode_rust(
        COMPRESSED_BC5,
        5592432,
        DECOMPRESSED_BC5,
        2048,
        2048,
        BcnEncoding::Bc5,
        BcnDecoderFormat::RGBA,
    );
}

#[test]
fn compare_decode_bc5() {
    compare_decode(
        COMPRESSED_BC5,
        2048,
        2048,
        BcnEncoding::Bc5,
        BcnDecoderFormat::RGBA,
    );
}

#[test]
fn compare_decode_bc6h() {
    compare_decode(
        COMPRESSED_BC6H,
        2048,
        2048,
        BcnEncoding::Bc6H,
        BcnDecoderFormat::RGBA,
    );
}
