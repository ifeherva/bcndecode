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
mod tests;
