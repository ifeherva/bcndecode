# Decoder for DXTn-compressed data

This crate provides methods to decompress DXTn-compressed image data via a wrapper around
the original C code used in the [Python Pillow Imaging package](https://python-pillow.org/).

The following formats are currently supported:

* Bc1: 565 color, 1-bit alpha (dxt1)
* Bc2: 565 color, 4-bit alpha (dxt3)
* Bc3: 565 color, 2-endpoint 8-bit interpolated alpha (dxt5)
* Bc4: 1-channel 8-bit via 1 BC3 alpha block
* Bc5: 2-channel 8-bit via 2 BC3 alpha blocks

The following formats are not implemented:

* Bc6: 3-channel 16-bit float
* Bc7: 4-channel 8-bit via everything

Format documentation:
http://oss.sgi.com/projects/ogl-sample/registry/EXT/texture_compression_s3tc.txt

License: MIT, copyright Robert Nix, Istvan Fehervari
