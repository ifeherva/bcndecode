# Decoder for DXTn-compressed data
[![Build Status](https://travis-ci.org/ifeherva/bcndecode.svg?branch=master)](https://travis-ci.org/ifeherva/bcndecode) [![Crates.io](https://img.shields.io/crates/v/bcndecode.svg)](https://crates.io/crates/bcndecode)

This crate provides methods to decompress DXTn-compressed image data. The decompression code was based on the original C code used in the [Python Pillow Imaging package](https://python-pillow.org/).

[Documentation](https://docs.rs/bcndecode/0.2.0/)

The following formats are currently supported:

* Bc1: 565 color, 1-bit alpha (dxt1)
* Bc2: 565 color, 4-bit alpha (dxt3)
* Bc3: 565 color, 2-endpoint 8-bit interpolated alpha (dxt5)
* Bc4: 1-channel 8-bit via 1 BC3 alpha block
* Bc5: 2-channel 8-bit via 2 BC3 alpha blocks
* Bc6: 3-channel 16-bit float

The following formats are not implemented:

* Bc7: 4-channel 8-bit

Format documentation:
http://oss.sgi.com/projects/ogl-sample/registry/EXT/texture_compression_s3tc.txt

License: MIT, copyright Istvan Fehervari, Robert Nix
