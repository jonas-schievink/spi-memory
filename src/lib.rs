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
