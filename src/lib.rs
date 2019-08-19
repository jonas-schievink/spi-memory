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

/// A trait for reading operations from a memory chip
pub trait Read<Addr, SPI: Transfer<u8>, CS: OutputPin> {
    /// Reads bytes from a memory chip
    ///
    /// # Parameters
    /// * `addr`: The address to start reading at
    /// * `buf`: The buffer to read buf.len() bytes into
    fn read(&mut self, addr: Addr, buf: &mut [u8]) -> Result<(), Error<SPI, CS>>;
}

/// A trait for writing and erasing operations on a memory chip
pub trait BlockDevice<Addr, SPI: Transfer<u8>, CS: OutputPin> {
    /// The block length in bytes, should be set to 1 for EEPROM implementations
    const BLOCK_LENGTH: usize;

    /// Erases bytes from the memory chip
    ///
    /// This function will return a `BlockLength` error if `amount` is not a multiple
    /// of [BLOCK_LENGTH](BlockDevice::BLOCK_LENGTH)
    ///
    /// # Parameters
    /// * `addr`: The address to start erasing at
    /// * `amount`: The amount of bytes to erase, starting at `addr`
    fn erase_bytes(&mut self, addr: Addr, amount: Addr) -> Result<(), Error<SPI, CS>> {
        if amount < Self::BLOCK_LENGTH || amount % Self::BLOCK_LENGTH != 0 {
            Err(Error::BlockLength)
        }
        unsafe {
            erase_unchecked(addr, amount)
        }
    }

    /// The "internal" method called by [erase_bytes](BlockDevice::erase_bytes), this function doesn't
    /// need to perform the checks regarding [BLOCK_LENGTH](BlockDevice::BLOCK_LENGTH) and is not supposed
    /// to be called by the end user of this library (which is the reason it is marked unsafe)
    unsafe fn erase_bytes_unchecked(&mut self, addr: Addr, amount: Addr) -> Result<(), Error<SPI, CS>>;

    /// Erases the memory chip fully
    fn erase_all(&mut self) -> Result<(), Error<SPI, CS>>;
    /// Writes bytes onto the memory chip
    ///
    /// # Parameters
    /// * `addr`: The address to write to
    /// * `data`: The bytes to write to `addr`
    fn write_block(&mut self, addr: Addr, data: &[u8]) -> Result<(), Error<SPI, CS>>;
}