# hacam-lib-rs

A Rust cross-platform userspace driver for interacting with the Huawei EnVizion 360° Camera (Huawei CV60).

## Platform support

Tested on Windows and macOS. Should work on Linux as well. 
This library uses the [nusb](https://github.com/kevinmehall/nusb) library.
The camera itself uses a standard LibUSB driver, so it works out of the box on macOS, but you need to select the driver manually on Windows (WinUSB).

## Examples

Examples are provided in the `examples` directory.

## Roadmap

- [x] Live view functionality
- [x] Recording video
- [x] Taking pictures
- [x] Reading/writing all settings (1:1 with the original app)   
- [ ] Firmware updating 