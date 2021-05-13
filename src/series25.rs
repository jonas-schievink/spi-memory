//! Driver for 25-series SPI Flash and EEPROM chips.

use crate::{utils::HexSlice, BlockDevice, Error, Read};
use bitflags::bitflags;
use core::convert::TryInto;
use core::fmt;
use embedded_hal::blocking::{delay::DelayUs, spi::Transfer};
use embedded_hal::digital::v2::OutputPin;

/// 3-Byte JEDEC manufacturer and device identification.
pub struct Identification {
    /// Data collected
    /// - First byte is the manufacturer's ID code from eg JEDEC Publication No. 106AJ
    /// - The trailing bytes are a manufacturer-specific device ID.
    bytes: [u8; 3],

    /// The number of continuations that precede the main manufacturer ID
    continuations: u8,
}

impl Identification {
    /// Build an Identification from JEDEC ID bytes.
    pub fn from_jedec_id(buf: &[u8]) -> Identification {
        // Example response for Cypress part FM25V02A:
        // 7F 7F 7F 7F 7F 7F C2 22 08  (9 bytes)
        // 0x7F is a "continuation code", not part of the core manufacturer ID
        // 0xC2 is the company identifier for Cypress (Ramtron)

        // Find the end of the continuation bytes (0x7F)
        let mut start_idx = 0;
        for i in 0..(buf.len() - 2) {
            if buf[i] != 0x7F {
                start_idx = i;
                break;
            }
        }

        Self {
            bytes: [buf[start_idx], buf[start_idx + 1], buf[start_idx + 2]],
            continuations: start_idx as u8,
        }
    }

    /// The JEDEC manufacturer code for this chip.
    pub fn mfr_code(&self) -> u8 {
        self.bytes[0]
    }

    /// The manufacturer-specific device ID for this chip.
    pub fn device_id(&self) -> &[u8] {
        self.bytes[1..].as_ref()
    }

    /// Number of continuation codes in this chip ID.
    ///
    /// For example the ARM Ltd identifier is `7F 7F 7F 7F 3B` (5 bytes), so
    /// the continuation count is 4.
    pub fn continuation_count(&self) -> u8 {
        self.continuations
    }
}

impl fmt::Debug for Identification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Identification")
            .field(&HexSlice(self.bytes))
            .finish()
    }
}

#[repr(u8)]
#[allow(unused)] // TODO support more features
enum Opcode {
    /// Read the 8-bit legacy device ID.
    ReadDeviceId = 0xAB,
    /// Read the 8-bit manufacturer and device IDs.
    ReadMfDId = 0x90,
    /// Read 16-bit manufacturer ID and 8-bit device ID.
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
    SectorErase = 0x20,
    BlockErase = 0xD8,
    ChipErase = 0xC7,
    PowerDown = 0xB9,
}

bitflags! {
    /// Status register bits.
    pub struct Status: u8 {
        /// Erase or write in progress.
        const BUSY = 1 << 0;
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
//pub struct Flash<SPI: Transfer<u8>, CS: OutputPin> {
pub struct Flash<CS: OutputPin> {
    //    spi: &mut SPI,
    cs: CS,
}

impl<CS: OutputPin> Flash<CS> {
    /// Creates a new 25-series flash driver.
    ///
    /// # Parameters
    ///
    /// * **`spi`**: An SPI master. Must be configured to operate in the correct
    ///   mode for the device.
    /// * **`cs`**: The **C**hip-**S**elect Pin connected to the `\CS`/`\CE` pin
    ///   of the flash chip. Will be driven low when accessing the device.
    pub fn init<SPI: Transfer<u8>>(spi: &mut SPI, cs: CS) -> Result<Self, Error<SPI, CS>> {
        let mut this = Self { cs };
        let status = this.read_status(spi)?;
        info!("Flash::init: status = {:?}", status);

        // Here we don't expect any writes to be in progress, and the latch must
        // also be deasserted.
        if !(status & (Status::BUSY | Status::WEL)).is_empty() {
            return Err(Error::UnexpectedStatus);
        }

        Ok(this)
    }

    fn command<SPI: Transfer<u8>>(
        &mut self,
        spi: &mut SPI,
        bytes: &mut [u8],
    ) -> Result<(), Error<SPI, CS>> {
        // If the SPI transfer fails, make sure to disable CS anyways
        self.cs.set_low().map_err(Error::Gpio)?;
        let spi_result = spi.transfer(bytes).map_err(Error::Spi);
        self.cs.set_high().map_err(Error::Gpio)?;
        spi_result?;
        Ok(())
    }

    /// Reads the JEDEC manufacturer/device identification.
    pub fn read_jedec_id<SPI: Transfer<u8>>(
        &mut self,
        spi: &mut SPI,
    ) -> Result<Identification, Error<SPI, CS>> {
        // Optimistically read 12 bytes, even though some identifiers will be shorter
        let mut buf: [u8; 12] = [0; 12];
        buf[0] = Opcode::ReadJedecId as u8;
        self.command(spi, &mut buf)?;

        // Skip buf[0] (SPI read response byte)
        Ok(Identification::from_jedec_id(&buf[1..]))
    }

    /// Reads the status register.
    pub fn read_status<SPI: Transfer<u8>>(
        &mut self,
        spi: &mut SPI,
    ) -> Result<Status, Error<SPI, CS>> {
        let mut buf = [Opcode::ReadStatus as u8, 0];
        self.command(spi, &mut buf)?;

        Ok(Status::from_bits_truncate(buf[1]))
    }

    fn write_enable<SPI: Transfer<u8>>(&mut self, spi: &mut SPI) -> Result<(), Error<SPI, CS>> {
        let mut cmd_buf = [Opcode::WriteEnable as u8];
        self.command(spi, &mut cmd_buf)?;
        Ok(())
    }

    fn wait_done<SPI: Transfer<u8>>(&mut self, spi: &mut SPI) -> Result<(), Error<SPI, CS>> {
        // TODO: Consider changing this to a delay based pattern
        while self.read_status(spi)?.contains(Status::BUSY) {}
        Ok(())
    }

    /// Enters power down mode.
    /// Datasheet, 8.2.35: Power-down:
    /// Although  the  standby  current  during  normal  operation  is  relatively  low,  standby  current  can  be  further
    /// reduced  with  the  Power-down  instruction.  The  lower  power  consumption  makes  the  Power-down
    /// instruction especially useful for battery powered applications (See ICC1 and ICC2 in AC Characteristics).
    /// The instruction is initiated by driving the /CS pin low and shifting the instruction code “B9h” as shown in
    /// Figure 44.  
    ///  
    /// The /CS pin must be driven high after the eighth bit has been latched. If this is not done the Power-down
    /// instruction will not be executed. After /CS is driven high, the power-down state will entered within the time
    /// duration of tDP (See AC Characteristics). While in the power-down state only the Release Power-down /
    /// Device ID (ABh) instruction, which restores the device to normal operation, will be recognized. All other
    /// instructions  are  ignored.  This  includes  the  Read  Status  Register  instruction,  which  is  always  available
    /// during normal operation. Ignoring all but one instruction makes the Power Down state a useful condition
    /// for  securing maximum  write protection. The  device  always  powers-up  in the  normal  operation with  the
    /// standby current of ICC1.   
    pub fn power_down<SPI: Transfer<u8>>(&mut self, spi: &mut SPI) -> Result<(), Error<SPI, CS>> {
        let mut buf = [Opcode::PowerDown as u8];
        self.command(spi, &mut buf)?;

        Ok(())
    }

    /// Exits Power Down Mode
    /// Datasheet, 8.2.36: Release Power-down:
    /// The Release from Power-down /  Device ID instruction is  a multi-purpose instruction. It can be used to
    /// release the device from the power-down state, or obtain the devices electronic identification (ID) number.   
    /// To  release the device  from  the  power-down state,  the instruction  is  issued by driving the  /CS  pin low,
    /// shifting the instruction code “ABh” and driving /CS high as shown in Figure 45. Release from power-down
    /// will  take  the  time  duration  of  tRES1  (See  AC  Characteristics)  before  the  device  will  resume  normal
    /// operation  and  other  instructions  are  accepted.  The  /CS  pin  must  remain  high  during  the  tRES1  time
    /// duration.
    ///
    /// Note: must manually delay after running this, IOC
    pub fn release_power_down<SPI: Transfer<u8>, D: DelayUs<u8>>(
        &mut self,
        spi: &mut SPI,
        delay: &mut D,
    ) -> Result<(), Error<SPI, CS>> {
        // Same command as reading ID.. Wakes instead of reading ID if not followed by 3 dummy bytes.
        let mut buf = [Opcode::ReadDeviceId as u8];
        self.command(spi, &mut buf)?;

        delay.delay_us(6); // Table 9.7: AC Electrical Characteristics: tRES1 = max 3us.

        Ok(())
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> Read<u32, SPI, CS> for Flash<CS> {
    /// Reads flash contents into `buf`, starting at `addr`.
    ///
    /// Note that `addr` is not fully decoded: Flash chips will typically only
    /// look at the lowest `N` bits needed to encode their size, which means
    /// that the contents are "mirrored" to addresses that are a multiple of the
    /// flash size. Only 24 bits of `addr` are transferred to the device in any
    /// case, limiting the maximum size of 25-series SPI flash chips to 16 MiB.
    ///
    /// # Parameters
    ///
    /// * `addr`: 24-bit address to start reading at.
    /// * `buf`: Destination buffer to fill.
    fn read(&mut self, spi: &mut SPI, addr: u32, buf: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        // TODO what happens if `buf` is empty?

        let mut cmd_buf = [
            Opcode::Read as u8,
            (addr >> 16) as u8,
            (addr >> 8) as u8,
            addr as u8,
        ];

        self.cs.set_low().map_err(Error::Gpio)?;
        let mut spi_result = spi.transfer(&mut cmd_buf);
        if spi_result.is_ok() {
            spi_result = spi.transfer(buf);
        }
        self.cs.set_high().map_err(Error::Gpio)?;
        spi_result.map(|_| ()).map_err(Error::Spi)
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> BlockDevice<u32, SPI, CS> for Flash<CS> {
    fn erase_sectors(
        &mut self,
        spi: &mut SPI,
        addr: u32,
        amount: usize,
    ) -> Result<(), Error<SPI, CS>> {
        for c in 0..amount {
            self.write_enable(spi)?;

            let current_addr: u32 = (addr as usize + c * 256).try_into().unwrap();
            let mut cmd_buf = [
                Opcode::SectorErase as u8,
                (current_addr >> 16) as u8,
                (current_addr >> 8) as u8,
                current_addr as u8,
            ];
            self.command(spi, &mut cmd_buf)?;
            self.wait_done(spi)?;
        }

        Ok(())
    }

    fn write_bytes(
        &mut self,
        spi: &mut SPI,
        addr: u32,
        data: &mut [u8],
    ) -> Result<(), Error<SPI, CS>> {
        for (c, chunk) in data.chunks_mut(256).enumerate() {
            self.write_enable(spi)?;

            let current_addr: u32 = (addr as usize + c * 256).try_into().unwrap();
            let mut cmd_buf = [
                Opcode::PageProg as u8,
                (current_addr >> 16) as u8,
                (current_addr >> 8) as u8,
                current_addr as u8,
            ];

            self.cs.set_low().map_err(Error::Gpio)?;
            let mut spi_result = spi.transfer(&mut cmd_buf);
            if spi_result.is_ok() {
                spi_result = spi.transfer(chunk);
            }
            self.cs.set_high().map_err(Error::Gpio)?;
            spi_result.map(|_| ()).map_err(Error::Spi)?;
            self.wait_done(spi)?;
        }
        Ok(())
    }

    fn erase_all(&mut self, spi: &mut SPI) -> Result<(), Error<SPI, CS>> {
        self.write_enable(spi)?;
        let mut cmd_buf = [Opcode::ChipErase as u8];
        self.command(spi, &mut cmd_buf)?;
        self.wait_done(spi)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_jedec_id() {
        let cypress_id_bytes = [0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0xC2, 0x22, 0x08];
        let ident = Identification::from_jedec_id(&cypress_id_bytes);
        assert_eq!(0xC2, ident.mfr_code());
        assert_eq!(6, ident.continuation_count());
        let device_id = ident.device_id();
        assert_eq!(device_id[0], 0x22);
        assert_eq!(device_id[1], 0x08);
    }
}
