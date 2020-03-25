//! An `embedded-hal`-based SPI-Flash chip driver.
//!
//! This crate aims to be compatible with common families of SPI flash chips.
//! Currently, reading 25-series chips is supported, and support for writing and
//! erasing as well as other chip families (eg. 24-series chips) is planned.
//! Contributions are always welcome!

#![doc(html_root_url = "https://docs.rs/spi-memory/0.2.0")]
#![warn(missing_debug_implementations, rust_2018_idioms)]
#![cfg_attr(not(test), no_std)]

#[macro_use]
mod log;
pub mod prelude;
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
            Error::__NonExhaustive(_) => unreachable!(),
        }
    }
}

/// A trait for reading operations from a memory chip.
pub trait Read<Addr, SPI: Transfer<u8>, CS: OutputPin> {
    /// Reads bytes from a memory chip.
    ///
    /// # Parameters
    /// * `addr`: The address to start reading at.
    /// * `buf`: The buffer to read `buf.len()` bytes into.
    fn read(&mut self, addr: Addr, buf: &mut [u8]) -> Result<(), Error<SPI, CS>>;
}

/// A trait for writing and erasing operations on a memory chip.
pub trait BlockDevice<Addr, SPI: Transfer<u8>, CS: OutputPin> {
    /// Erases sectors from the memory chip.
    ///
    /// # Parameters
    /// * `addr`: The address to start erasing at. If the address is not on a sector boundary,
    ///   the lower bits can be ignored in order to make it fit.
    fn erase_sectors(&mut self, addr: Addr, amount: usize) -> Result<(), Error<SPI, CS>>;

    /// Erases the memory chip fully.
    ///
    /// Warning: Full erase operations can take a significant amount of time.
    /// Check your device's datasheet for precise numbers.
    fn erase_all(&mut self) -> Result<(), Error<SPI, CS>>;
    /// Writes bytes onto the memory chip. This method is supposed to assume that the sectors
    /// it is writing to have already been erased and should not do any erasing themselves.
    ///
    /// # Parameters
    /// * `addr`: The address to write to.
    /// * `data`: The bytes to write to `addr`.
    fn write_bytes(&mut self, addr: Addr, data: &mut [u8]) -> Result<(), Error<SPI, CS>>;
}
