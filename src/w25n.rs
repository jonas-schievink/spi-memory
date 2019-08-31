use crate::{BlockDevice, Error, Read};
use crate::w25m::Stackable;
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;
use bitflags::bitflags;
use core::fmt::Debug;
use core::convert::TryInto;

enum Opcode {
    // Read one of the 3 8 bit status registers
    ReadStatus = 0x05,
    // Set the write enable latch.
    WriteEnable = 0x06,
    // Write to one of the three status registers
    WriteStatus = 0x1F,
    // Erase a 128 kb block
    BlockErase = 0xD8,
    // Read one page of data into the buffer
    PageDataRead = 0x13,
    // Read data from the buffer
    ReadData = 0x03,
    // Write a page of data from the buffer into a memory region
    ProgramExecute = 0x10,
    // Write a page of data into the buffer
    RandomLoadProgramData = 0x84,
}

bitflags! {
    /// Status register bits.
    pub struct Status3: u8 {
        /// Erase or write in progress.
        const BUSY = 1 << 0;
        /// Status of the **W**rite **E**nable **L**atch.
        const WEL = 1 << 1;
    }
}

/// Driver for W25N series SPI Flash chips.
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
        let status = this.read_status_3()?;
        info!("Flash::init: status = {:?}", status);
        // Here we don't expect any writes to be in progress, and the latch must
        // also be deasserted.
        if !(status & (Status3::BUSY | Status3::WEL)).is_empty() {
            return Err(Error::UnexpectedStatus);
        }

        // write to config register 2, set BUF=0 (continious mode) and everything else on reset
        this.command(&mut [Opcode::WriteStatus as u8, 0xA0, 0b00000010])?;
        this.command(&mut [Opcode::WriteStatus as u8, 0xB0, 0b00010000])?;
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

    /// Reads status register 3
    pub fn read_status_3(&mut self) -> Result<Status3, Error<SPI, CS>> {
        let mut buf = [Opcode::ReadStatus as u8, 0xC0, 0];
        self.command(&mut buf)?;
        Ok(Status3::from_bits_truncate(buf[2]))
    }

    fn write_enable(&mut self) -> Result<(), Error<SPI, CS>> {
        let mut cmd_buf = [Opcode::WriteEnable as u8];
        self.command(&mut cmd_buf)?;
        Ok(())
    }

    fn wait_done(&mut self) -> Result<(), Error<SPI, CS>> {
        // TODO: Consider changing this to a delay based pattern
        while self.read_status_3()?.contains(Status3::BUSY) {}
        Ok(())
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> Stackable<SPI,CS> for Flash<SPI,CS> 
where
    SPI::Error: Debug,
    CS::Error: Debug,
{
    fn new(spi: SPI, cs: CS) -> Result<Self, Error<SPI, CS>> {
        Flash::init(spi, cs)
    }

    fn free(self) -> (SPI, CS) {
        (self.spi, self.cs)
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> Read<SPI, CS> for Flash<SPI, CS> {
    fn read(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        let start_addr: u16 = (addr / 2048).try_into().unwrap(); // page address = addr / 2048 byte
        let mut cmd_buf = [
            Opcode::PageDataRead as u8,
            0, // dummy cycles
            (start_addr >> 8) as u8,
            start_addr as u8
        ];

        self.command(&mut cmd_buf)?;
        self.wait_done()?;

        let mut cmd_buf = [
            Opcode::ReadData as u8,
            0, // 24 dummy cycles
            0,
            0,
        ];

        self.cs.set_low().map_err(Error::Gpio)?;
        let mut spi_result = self.spi.transfer(&mut cmd_buf);
        if spi_result.is_ok() {
            spi_result = self.spi.transfer(buf);
        }
        self.cs.set_high().map_err(Error::Gpio)?;
        self.wait_done()?;
        spi_result.map(|_| ()).map_err(Error::Spi)
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> BlockDevice<SPI, CS> for Flash<SPI, CS> {
    fn erase(&mut self, addr: u32, amount: usize) -> Result<(), Error<SPI, CS>> {
        let start_addr: u16 = (addr / 2048).try_into().unwrap(); // page address = addr / 2048 byte
        for c in 0..amount {
            self.write_enable()?;

            let current_addr: u16 = (start_addr as usize + c).try_into().unwrap();
            let mut cmd_buf = [
                Opcode::BlockErase as u8,
                0, // 8 dummy cycles
                (current_addr >> 8) as u8,
                current_addr as u8,
            ];
            self.command(&mut cmd_buf)?;
            self.wait_done()?;
        }

        Ok(())
    }

    fn write_bytes(&mut self, addr: u32, data: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        let start_addr: u16 = (addr / 2048).try_into().unwrap(); // page address = addr / 2048 byte
        let mut current_addr = start_addr;
        data.reverse();
        for chunk in data.chunks_mut(2048).rev() {
            chunk.reverse();
            self.write_enable()?;
            let column_addr: u16 = current_addr % 2048;
            let mut cmd_buf = [
                Opcode::RandomLoadProgramData as u8,
                (column_addr >> 8) as u8,
                column_addr as u8,
            ];

            self.cs.set_low().map_err(Error::Gpio)?;
            let mut spi_result = self.spi.transfer(&mut cmd_buf);
            if spi_result.is_ok() {
                spi_result = self.spi.transfer(chunk);
            }
            self.cs.set_high().map_err(Error::Gpio)?;
            spi_result.map(|_| ()).map_err(Error::Spi)?;

            self.wait_done()?;

            let mut cmd_buf = [
                Opcode::ProgramExecute as u8,
                0, // 8 dummy cycles
                (current_addr >> 8) as u8,
                current_addr as u8,
            ];
            self.command(&mut cmd_buf)?;
            self.wait_done()?;
            current_addr = current_addr + chunk.len() as u16;
        }
        Ok(())
    }

    fn erase_all(&mut self) -> Result<(), Error<SPI, CS>> {
        self.erase(0, 1024)
    }
}



















