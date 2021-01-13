# Microchip 23x SRAM/NVSRAM embedded-hal SPI driver

[![crates.io](https://img.shields.io/crates/v/sram23x.svg)](https://crates.io/crates/sram23x)
[![Docs](https://docs.rs/sram23x/badge.svg)](https://docs.rs/sram23x)
![Maintenance Intention](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)

This is a platform agnostic Rust driver for the [23x series serial SRAM/NVSRAM SPI memory chips](https://www.microchip.com/en-us/products/memory/serial-sram-and-serial-nvsram),
based on the [`embedded-hal`](https://github.com/rust-embedded/embedded-hal) traits.

This driver allows you to:

- Read a single byte from a memory address. See: `read_byte()`.
- Read a 32-byte page starting on a memory address. See: `read_page()`.
- Read an N-byte array starting on a memory address. See: `read_sequential()`.
- Write a single byte to a memory address. See: `write_byte()`.
- Write a 32-byte page starting on a memory address. See: `write_page()`.
- Write an N-byte array starting on a memory address. See: `write_sequential()`.
- Enable and disable transmission by managing the _HOLD_ pin.
- Get/Set the operating mode/status register.

Read the [API Documentation](https://docs.rs/sram23x) for more information.

# Supported devices

| Device | Memory bytes | Memory bits | HOLD pin | Datasheet |
|-------:|------------:|------------:|----------:|:-----------|
|   M23x640 |   8 KB |   64 Kbit | yes |    [23A640/23K640] |
|   M23x256  |  32 KB | 256 Kbit | yes |    [23A256/23K256] |
|   M23x512  |  64 KB | 512 Kbit | yes |   [23A512/23LC512] |
|   M23x512  |  64 KB | 512 Kbit |  no |         [23LCV512] |
|  M23x1024  | 128 KB |   1 Mbit | yes | [23A1024/23LC1024] |
| M23xv1024  | 128 KB |   1 Mbit |  no |        [23LCV1024] |

[23A640/23K640]: http://ww1.microchip.com/downloads/en/DeviceDoc/22126E.pdf
[23A256/23K256]: http://ww1.microchip.com/downloads/en/DeviceDoc/22100F.pdf
[23A512/23LC512]: https://ww1.microchip.com/downloads/en/DeviceDoc/20005155B.pdf
[23LCV512]: https://ww1.microchip.com/downloads/en/DeviceDoc/25157A.pdf
[23A1024/23LC1024]: https://ww1.microchip.com/downloads/en/DeviceDoc/20005142C.pdf
[23LCV1024]: https://ww1.microchip.com/downloads/en/DeviceDoc/25156A.pdf

# Usage

Include [library](https://crates.io/crates/sram23x) as a dependency in your Cargo.toml

```toml
[dependencies]
sram23x = "0.2.1"
```

Some example usage:

```rust
extern crate sram23x;
use sram23x::*;

fn main() {
    // 1. Ensure spi, cs, and hold pins are defined. hold pin is required (any unused output pin will do)
    // (device specific)

    // 2. Instantiate memory device 23LCV1024
    let mut sram = Sram23x::new(spi, cs, hold, device_type::M23xv1024).unwrap();

    // 3. Check the operating mode register
    println!("Operating mode register: {:?}", sram.mode);

    // 4. Change the operating mode to sequential
    sram.set_mode(OperatingMode::Sequential as u8).unwrap();
    assert_eq!(sram.mode, 0b01);

    // 5. Write 4 bytes of data starting at address 0x00 from a buffer
    let mut data: [u8; 4] = ['t' as u8, 'e' as u8, 's' as u8, 't' as u8];
    sram.write_sequential(0x00_u32, &mut data).unwrap();

    // 6. Read 4 bytes of data starting at address 0x00 into a buffer
    sram.read_sequential(0x00_u32, &mut data).unwrap();
    println!("Read data: {:?}", data);
    assert_eq!(data[0], 't' as u8);
    assert_eq!(data[1], 'e' as u8);
    assert_eq!(data[2], 's' as u8);
    assert_eq!(data[3], 't' as u8);

    // 7. Write and read 1 byte to/from address 0x04
    sram.set_mode(OperatingMode::Byte as u8).unwrap();
    assert_eq!(sram.mode, 0b00);
    sram.write_byte(0x04_u32, 'a' as u8).unwrap();
    let byte = sram.read_byte(0x04_u32).unwrap();
    println!("Read 1 byte: {:?}", byte);
    assert_eq!(byte, 'a' as u8);

    // 8. Write and read a 32-byte page starting at address 0x00
    sram.set_mode(OperatingMode::Page as u8).unwrap();
    assert_eq!(sram.mode, 0b10);
    let mut data = "Microchip\n1Mbit serial\nsram test".as_bytes();
    sram.write_page(0x00_u32, data).unwrap();
    let page = sram.read_page(0x00_u32).unwrap();
    println!("Read a 32-byte page: {:?}", page);
    assert_eq!(page[0], 'M' as u8);
    assert_eq!(page[31], 't' as u8);
}
```

# Todo

- [ ] Separate I/O into their own private functions
- [ ] Add tests
- [ ] Document other missing minor details

# Contributing

If you find any bugs or issues, please [create an issue](https://github.com/aw/sram23x/issues/new).

# License

[MIT License](LICENSE)

Copyright (c) 2021 Alexander Williams, On-Prem <license@on-premises.com>
