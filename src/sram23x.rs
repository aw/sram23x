use super::*;
use bit_field::BitField;
use core::convert::TryFrom;
use hal::blocking::spi::{Transfer, Write};
use hal::digital::v2::OutputPin;

pub trait DeviceType {
    const ADDRESS_BYTES: usize;
    const HOLD_PIN: bool; // the device has a HOLD pin
    const HOLD_STATUS: bool; // the device has a HOLD status register bit
    const MAX: u32;

    fn fill_address(address: &mut u32, instruction: Instruction);
}

// Macros
macro_rules! impl_device_type {
    ($devicetype:ident, $a:expr, $b:expr, $c:expr, $d:expr) => {
        impl DeviceType for device_type::$devicetype {
            const ADDRESS_BYTES: usize = $a;
            const HOLD_PIN: bool = $b;
            const HOLD_STATUS: bool = $c;
            const MAX: u32 = $d;

            fn fill_address(address: &mut u32, instruction: Instruction) {
                address.set_bits(24..31, instruction as u32);
            }
        }
    };
}

impl_device_type!(M23x640, 3, true, true, 0x1FFF_u32);
impl_device_type!(M23x256, 3, true, true, 0x7FFF_u32);
impl_device_type!(M23x512, 3, true, false, 0xFFFF_u32);
impl_device_type!(M23xv512, 3, false, false, 0xFFFF_u32);
impl_device_type!(M23x1024, 4, true, false, 0x1FFFF_u32);
impl_device_type!(M23xv1024, 4, false, false, 0x1FFFF_u32);

impl<SPI, S, P, CS, HOLD, DT> Sram23x<SPI, CS, HOLD, DT>
where
    SPI: Transfer<u8, Error = S> + Write<u8, Error = S>,
    CS: OutputPin<Error = P>,
    HOLD: OutputPin<Error = P>,
    DT: DeviceType,
{
    /// Initialize the SRAM device, disable the pin's hold feature, and obtain the operating mode
    pub fn new(spi: SPI, cs: CS, hold: HOLD, dt: DT) -> Result<Self, Error<S, P>> {
        let mut sram = Sram23x {
            spi,
            cs,
            hold,
            dt,
            mode: 0,
        };
        sram.cs.set_high().map_err(Error::PinError)?;
        sram.set_hold(false)?;
        sram.get_mode()?;
        Ok(sram)
    }

    /// Transfer data over the SPI bus
    pub fn transfer(&mut self, bytes: &mut [u8]) -> SpiRes<S, P> {
        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.transfer(bytes).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;
        Ok(())
    }

    /// Return the operating mode/status of the device
    pub fn get_mode(&mut self) -> Result<u8, Error<S, P>> {
        let mut buf: [u8; 2] = [Instruction::ReadMode as u8, 0];
        self.transfer(&mut buf)?;
        self.get_mode_bits(buf[1])?;
        self.mode = buf[1];
        Ok(self.mode)
    }

    /// Return true if the bit pattern for the operating mode is valid
    fn get_mode_bits(&mut self, bits: u8) -> Result<bool, Error<S, P>> {
        match bits.get_bits(6..8) {
            0b00 | 0b10 | 0b01 | 0b11 => return Ok(true),
            _ => return Err(Error::UnknownOperatingMode),
        }
    }

    /// Sets the operating mode/status of the device
    pub fn set_mode(&mut self, mode: u8) -> SpiRes<S, P> {
        let mut buf: [u8; 2] = [Instruction::WriteMode as u8, mode];
        self.transfer(&mut buf)?;
        self.mode = mode;
        Ok(())
    }

    /// Enable the hold pin (bring it low), which prevents data transmission
    pub fn set_hold(&mut self, enabled: bool) -> SpiRes<S, P> {
        if DT::HOLD_PIN {
            if enabled {
                self.enable_hold_feature()?;
                self.hold.set_low().map_err(Error::PinError)
            } else {
                self.hold.set_high().map_err(Error::PinError)
            }
        } else {
            Ok(())
        }
    }

    /// Enable the HOLD feature (status register bit 0) on 23x640 and 23x256 devices
    pub fn enable_hold_feature(&mut self) -> SpiRes<S, P> {
        if DT::HOLD_PIN && DT::HOLD_STATUS {
            self.mode.set_bit(0, false); // setting to false enables the HOLD functionality
            self.set_mode(self.mode)?;
        }
        Ok(())
    }

    /// Disable the HOLD feature (status register bit 0) on 23x640 and 23x256 devices
    pub fn disable_hold_feature(&mut self) -> SpiRes<S, P> {
        if DT::HOLD_PIN && DT::HOLD_STATUS {
            self.mode.set_bit(0, true); // setting to true disables the HOLD functionality
            self.set_mode(self.mode)?;
        }
        Ok(())
    }

    /// Read a single byte from an address
    pub fn read_byte(&mut self, address: u32) -> Result<u8, Error<S, P>> {
        if address > DT::MAX {
            Err(Error::InvalidAddress)
        } else {
            let mut addr = address;
            DT::fill_address(&mut addr, Instruction::Read);
            let data = addr.to_be_bytes();
            let mut buf: [u8; 5] = self.get_address_array(data, 0)?;
            self.transfer(&mut buf[..=DT::ADDRESS_BYTES])?;
            Ok(buf[DT::ADDRESS_BYTES])
        }
    }

    /// Write a single byte to an address
    pub fn write_byte(&mut self, address: u32, byte: u8) -> SpiRes<S, P> {
        if address > DT::MAX {
            Err(Error::InvalidAddress)
        } else {
            let mut addr = address;
            DT::fill_address(&mut addr, Instruction::Write);
            let data = addr.to_be_bytes();
            let mut buf: [u8; 5] = self.get_address_array(data, byte)?;
            self.transfer(&mut buf[..=DT::ADDRESS_BYTES])?;
            Ok(())
        }
    }

    /// Return a 5 element array with the address and filled data byte(s)
    fn get_address_array(&mut self, data: [u8; 4], byte: u8) -> Result<[u8; 5], Error<S, P>> {
        match DT::ADDRESS_BYTES {
            3 => return Ok([data[0], data[2], data[3], byte, 0]),
            4 => return Ok([data[0], data[1], data[2], data[3], byte]),
            _ => return Err(Error::InvalidAddressSize),
        };
    }

    /// Read a 32-byte page starting from an address
    pub fn read_page(&mut self, address: u32) -> Result<[u8; 32], Error<S, P>> {
        if address > DT::MAX {
            Err(Error::InvalidAddress)
        } else {
            let mut addr = address;
            DT::fill_address(&mut addr, Instruction::Read);
            let data = addr.to_be_bytes();
            let mut buf: [u8; 36] = [0; 36];
            let size: usize = DT::ADDRESS_BYTES + 32;
            let res: [u8; 32] = match DT::ADDRESS_BYTES {
                3 => {
                    buf[0] = data[0];
                    buf[1] = data[2];
                    buf[2] = data[3];
                    self.transfer(&mut buf[..size])?;
                    TryFrom::try_from(&buf[3..35]).unwrap()
                }
                4 => {
                    buf[..4].clone_from_slice(&data[..]);
                    self.transfer(&mut buf[..size])?;
                    TryFrom::try_from(&buf[4..]).unwrap()
                }
                _ => return Err(Error::InvalidAddressSize),
            };
            Ok(res)
        }
    }

    /// Write a 32-byte page starting from an address
    pub fn write_page(&mut self, address: u32, bytes: &[u8]) -> SpiRes<S, P> {
        if address > DT::MAX {
            Err(Error::InvalidAddress)
        } else if bytes.len() > 32 {
            Err(Error::TooMuchData)
        } else {
            let mut addr = address;
            DT::fill_address(&mut addr, Instruction::Write);
            let data = addr.to_be_bytes();
            let mut buf: [u8; 36] = [0; 36];
            match DT::ADDRESS_BYTES {
                3 => {
                    buf[0] = data[0];
                    buf[1] = data[2];
                    buf[2] = data[3];
                    buf[3..35].clone_from_slice(bytes);
                }
                4 => {
                    buf[..4].clone_from_slice(&data[..]);
                    buf[4..].clone_from_slice(bytes);
                }
                _ => return Err(Error::InvalidAddressSize),
            };
            let size: usize = DT::ADDRESS_BYTES + 32;
            self.transfer(&mut buf[..size])?;
            Ok(())
        }
    }

    /// Read N-bytes from an array sequentially, starting from an address
    pub fn read_sequential(&mut self, address: u32, bytes: &mut [u8]) -> SpiRes<S, P> {
        self.sequential(address, bytes, Instruction::Read)
    }

    /// Write N-bytes to an array sequentially, starting from an address
    pub fn write_sequential(&mut self, address: u32, bytes: &mut [u8]) -> SpiRes<S, P> {
        self.sequential(address, bytes, Instruction::Write)
    }

    fn sequential(
        &mut self,
        address: u32,
        bytes: &mut [u8],
        instruction: Instruction,
    ) -> SpiRes<S, P> {
        if address > DT::MAX {
            Err(Error::InvalidAddress)
        } else {
            let mut addr = address;
            DT::fill_address(&mut addr, instruction);
            let data = addr.to_be_bytes();
            let mut buf: [u8; 4] = match DT::ADDRESS_BYTES {
                3 => [data[0], data[2], data[3], 0],
                4 => data,
                _ => return Err(Error::InvalidAddressSize),
            };
            self.cs.set_low().map_err(Error::PinError)?;
            self.spi
                .transfer(&mut buf[..DT::ADDRESS_BYTES])
                .map_err(Error::SpiError)?;
            self.spi.transfer(&mut bytes[..]).map_err(Error::SpiError)?;
            self.cs.set_high().map_err(Error::PinError)?;
            Ok(())
        }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
}
