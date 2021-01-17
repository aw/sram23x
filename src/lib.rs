/*!
This is a platform agnostic Rust driver for the [23x series serial SRAM/NVSRAM SPI memory chips](https://www.microchip.com/en-us/products/memory/serial-sram-and-serial-nvsram),
based on the [`embedded-hal`](https://github.com/rust-embedded/embedded-hal) traits.

See the [Intro post](https://blog.a1w.ca/p/rust-embedded-driver-microchip-23x-sram).

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
sram23x = "0.2.2"
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

*/
#![deny(unsafe_code)]
#![no_std]

extern crate bit_field;
extern crate embedded_hal as hal;

mod sram23x;

/// Microchip SRAM 23x driver
#[derive(Debug, Default)]
pub struct Sram23x<SPI, CS, HOLD, DT> {
    /// The concrete SPI device implementation
    spi: SPI,
    /// The SPI chip select pin
    cs: CS,
    /// The SPI device hold pin
    hold: HOLD,
    /// The SRAM device type
    dt: DT,
    /// The operating mode of the device
    pub mode: u8,
}

/// All possible instructions
#[repr(u8)]
pub enum Instruction {
    /// Read data from memory
    Read = 0x03,
    /// Write data to memory
    Write = 0x02,
    /// Enter Dual I/O access
    EnterDualIo = 0x3B,
    /// Enter Quad I/O access
    EnterQuadIo = 0x38,
    /// Reset Dual/Quad I/O access
    ResetIo = 0xFF,
    /// Read the 8-bit mode/status register
    ReadMode = 0x05,
    /// Write the 8-bit mode/status register
    WriteMode = 0x01,
}

/// Modes of operation
#[repr(u8)]
pub enum OperatingMode {
    /// In this mode, the read/write operations are limited to only one byte
    Byte = 0b00_000000,
    /// In this mode, the read and write operations are limited to within the addressed page
    Page = 0b10_000000,
    /// In this mode, the entire array can be written to and read from
    Sequential = 0b01_000000,
    /// Reserved (do not use this mode)
    Reserved = 0b11_000000,
}

/// All possible errors in this crate
#[derive(Debug)]
#[repr(u8)]
pub enum Error<S, P> {
    /// SPI bus error
    SpiError(S),
    /// Pin error
    PinError(P),
    /// Too much data received/passed for a read or write
    TooMuchData,
    /// Memory address is out of range
    InvalidAddress,
    /// Address size is invalid
    InvalidAddressSize,
    /// Operating mode is invalid, use `set_mode()` to change it
    InvalidOperatingMode,
    /// Operating mode is unknown
    UnknownOperatingMode,
}

/// Types of devices supported by this crate
pub mod device_type {
    /// Microchip 23A640/23K640, 8KB (64Kbit) SRAM
    pub struct M23x640;
    /// Microchip 23A256/23K256, 32KB (256Kbit) SRAM
    pub struct M23x256;
    /// Microchip 23A512/23LC512, 64KB (512Kbit) SRAM
    pub struct M23x512;
    /// Microchip 23LCV512, 64KB (512Kbit) NVSRAM (VBat)
    pub struct M23xv512;
    /// Microchip 23A1024/23LC1024, 128KB (1Mbit) SRAM
    pub struct M23x1024;
    /// Microchip 23LCV1024, 128KB (1Mbit) NVSRAM (VBat)
    pub struct M23xv1024;
}

type SpiRes<S, P> = Result<(), Error<S, P>>;
