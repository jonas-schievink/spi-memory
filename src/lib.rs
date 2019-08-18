//! An `embedded-hal`-based SPI-Flash chip driver.
//!
//! This crate aims to be compatible with common families of SPI flash chips.
//! Currently, reading 25-series chips is supported, and support for writing and
//! erasing as well as other chip families (eg. 24-series chips) is planned.
//! Contributions are always welcome!

#![doc(html_root_url = "https://docs.rs/spi-memory/0.1.0")]
#![warn(missing_debug_implementations, rust_2018_idioms)]
#![no_std]

#[macro_use]
mod log;
pub mod series25;
mod utils;

use core::fmt::{self, Debug};
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;

mod private {
    #[derive(Debug)]
    pub enum Private {}
}

/// The error type used by this library.
///
/// This can encapsulate an SPI or GPIO error, and adds its own protocol errors
/// on top of that.
pub enum Error<SPI: Transfer<u8>, GPIO: OutputPin> {
    /// An SPI transfer failed.
    Spi(SPI::Error),

    /// A GPIO could not be set.
    Gpio(GPIO::Error),

    /// The data submitted for a write onto a Flash chip did not match its 
    /// block length
    BlockLength,

    /// Status register contained unexpected flags.
    ///
    /// This can happen when the chip is faulty, incorrectly connected, or the
    /// driver wasn't constructed or destructed properly (eg. while there is
    /// still a write in progress).
    UnexpectedStatus,

    #[doc(hidden)]
    __NonExhaustive(private::Private),
}

impl<SPI: Transfer<u8>, GPIO: OutputPin> Debug for Error<SPI, GPIO>
where
    SPI::Error: Debug,
    GPIO::Error: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Spi(spi) => write!(f, "Error::Spi({:?})", spi),
            Error::Gpio(gpio) => write!(f, "Error::Gpio({:?})", gpio),
            Error::UnexpectedStatus => f.write_str("Error::UnexpectedStatus"),
            Error::BlockLength => f.write_str("Error::BlockLength"),
            Error::__NonExhaustive(_) => unreachable!(),
        }
    }
}

pub trait Read<Addr, SPI: Transfer<u8>, CS: OutputPin> {
    /// Reads flash contents into `buf`, starting at `addr`.
    ///
    /// This will always read `buf.len()` worth of bytes, filling up `buf`
    /// completely.
    fn read(&mut self, addr: Addr, buf: &mut [u8]) -> Result<(), Error<SPI, CS>>;
}

pub trait FlashWrite<Addr, SPI: Transfer<u8>, CS: OutputPin> {
    const BLOCK_LENGTH: usize;

    /// Writes a block of data onto a flash memory chip, this function checks if the
    /// block is the length set in the associated constant [BLOCK_LENGTH](FlashWrite.BLOCK_LENGTH)
    /// and will throw an error if it isn't.
    fn write(&mut self, addr: Addr, block: &[u8]) -> Result<(), Error<SPI, CS>> {
        if block.len() == Self::BLOCK_LENGTH {
            unsafe {
                self.write_block_unchecked(addr, block)?;
            }
            Ok(())
        }
        else {
            Err(Error::BlockLength)
        }
    }

    /// Writes a block without checking wether it fits the block size
    /// This function should never be used directly, instead the [write](FlashWrite.write) should be used
    /// as it provides a safe frontend for this function
    unsafe fn write_block_unchecked(&mut self, addr: Addr, block: &[u8]) -> Result<(), Error<SPI, CS>>;
}

pub trait EepromWrite<Addr, SPI: Transfer<u8>, CS: OutputPin> {
    type Error;

    /// Writes a block of data towards addr onto an EEPROM chip
    fn write(&mut self, addr: Addr, block: &[u8]) -> Result<(), Error<SPI, CS>>;
}
