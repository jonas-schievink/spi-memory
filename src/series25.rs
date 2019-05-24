//! Driver for 25-series SPI Flash and EEPROM chips.

use crate::{utils::HexSlice, Error};
use bitflags::bitflags;
use core::fmt;
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;

/// 3-Byte JEDEC manufacturer and device identification.
pub struct Identification {
    /// The received bytes, in order.
    ///
    /// First 1 or 2 Bytes are the JEDEC manufacturer ID, last 1-2 Bytes are the
    /// device ID. How many bytes are used depends on manufacturer's place in
    /// the JEDEC list, I guess.
    bytes: [u8; 3],
}

impl fmt::Debug for Identification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Identification")
            .field(&HexSlice(self.bytes))
            .finish()
    }
}

#[allow(unused)] // TODO support more features
enum Opcode {
    /// Read the 8-bit legacy device ID.
    ReadDeviceId = 0xAB,
    /// Read the 8-bit manufacturer and device IDs.
    ReadMfDId = 0x90,
    /// Read the 16-bit manufacturer ID and 8-bit device ID.
    ReadJedecId = 0x9F,
    /// Set the write enable latch.
    WriteEnable = 0x06,
    /// Clear the write enable latch.
    WriteDisable = 0x04,
    /// Read the 8-bit status register.
    ReadStatus = 0x05,
    /// Write the 8-bit status register. Not all bits are writeable.
    WriteStatus = 0x01,
    Read = 0x03,
    PageProg = 0x02, // directly writes to EEPROMs too
    SectorErase = 0xD7,
    BlockErase = 0xD8,
    ChipErase = 0xC7,
}

bitflags! {
    /// Status register bits.
    pub struct Status: u8 {
        /// **W**rite **I**n **P**rogress bit.
        const WIP = 1 << 0;
        /// Status of the **W**rite **E**nable **L**atch.
        const WEL = 1 << 1;
        /// The 3 protection region bits.
        const PROT = 0b00011100;
        /// **S**tatus **R**egister **W**rite **D**isable bit.
        const SRWD = 1 << 7;
    }
}

/// Driver for 25-series SPI Flash chips.
///
/// # Type Parameters
///
/// * **`SPI`**: The SPI master to which the flash chip is attached.
/// * **`CS`**: The **C**hip-**S**elect line attached to the `\CS`/`\CE` pin of
///   the flash chip.
#[derive(Debug)]
pub struct Flash<SPI: Transfer<u8>, CS: OutputPin> {
    spi: SPI,
    cs: CS,
}

impl<SPI: Transfer<u8>, CS: OutputPin> Flash<SPI, CS> {
    /// Creates a new 25-series flash driver.
    ///
    /// # Parameters
    ///
    /// * **`spi`**: An SPI master. Must be configured to operate in the correct
    ///   mode for the device.
    /// * **`cs`**: The **C**hip-**S**elect Pin connected to the `\CS`/`\CE` pin
    ///   of the flash chip. Will be driven low when accessing the device.
    pub fn init(spi: SPI, cs: CS) -> Result<Self, Error<SPI, CS>> {
        let mut this = Self { spi, cs };
        let status = this.read_status()?;
        info!("Flash::init: status = {:?}", status);

        // Here we don't expect any writes to be in progress, and the latch must
        // also be deasserted.
        if !(status & (Status::WIP | Status::WEL)).is_empty() {
            return Err(Error::UnexpectedStatus);
        }

        Ok(this)
    }

    fn command(&mut self, bytes: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        // If the SPI transfer fails, make sure to disable CS anyways
        self.cs.set_low().map_err(Error::Gpio)?;
        let spi_result = self.spi.transfer(bytes).map_err(Error::Spi);
        self.cs.set_high().map_err(Error::Gpio)?;
        spi_result?;
        Ok(())
    }

    /// Reads the JEDEC manufacturer/device identification.
    pub fn read_jedec_id(&mut self) -> Result<Identification, Error<SPI, CS>> {
        let mut buf = [Opcode::ReadJedecId as u8, 0, 0, 0];
        self.command(&mut buf)?;

        Ok(Identification {
            bytes: [buf[1], buf[2], buf[3]],
        })
    }

    /// Reads the status register.
    pub fn read_status(&mut self) -> Result<Status, Error<SPI, CS>> {
        let mut buf = [Opcode::ReadStatus as u8, 0];
        self.command(&mut buf)?;

        Ok(Status::from_bits_truncate(buf[1]))
    }

    /// Reads flash contents into `buf`, starting at `addr`.
    ///
    /// This will always read `buf.len()` worth of bytes, filling up `buf`
    /// completely.
    ///
    /// Note that `addr` is not fully decoded: Flash chips will typically only
    /// look at the lowest `N` bits needed to encode their size, which means
    /// that the contents are "mirrored" to addresses that are a multiple of the
    /// flash size. Only 24 bits of `addr` are transferred to the device in any
    /// case, limiting the maximum size of 25-series SPI flash chips to 16 MiB.
    ///
    /// # Parameters
    ///
    /// * **`addr`**: 24-bit address to start reading at.
    /// * **`buf`**: Destination buffer to fill.
    pub fn read(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        // TODO what happens if `buf` is empty?

        let mut cmd_buf = [
            Opcode::Read as u8,
            (addr >> 16) as u8,
            (addr >> 8) as u8,
            addr as u8,
        ];

        self.cs.set_low().map_err(Error::Gpio)?;
        let mut spi_result = self.spi.transfer(&mut cmd_buf);
        if spi_result.is_ok() {
            spi_result = self.spi.transfer(buf);
        }
        self.cs.set_high().map_err(Error::Gpio)?;
        spi_result.map(|_| ()).map_err(Error::Spi)
    }
}
