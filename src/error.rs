use core::fmt::{self, Debug, Display};
use embedded_hal::digital::v2::OutputPin;

mod private {
    #[derive(Debug)]
    pub enum Private {}
}

/// The error type used by this library.
///
/// This can encapsulate an SPI or GPIO error, and adds its own protocol errors
/// on top of that.
pub enum Error<E, GPIO: OutputPin> {
    /// An SPI transfer failed.
    Spi(E),

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

impl<E, GPIO: OutputPin> Debug for Error<E, GPIO>
where
    E: Debug,
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

impl<E, GPIO: OutputPin> Display for Error<E, GPIO>
where
    E: Display,
    GPIO::Error: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Spi(spi) => write!(f, "SPI error: {}", spi),
            Error::Gpio(gpio) => write!(f, "GPIO error: {}", gpio),
            Error::UnexpectedStatus => f.write_str("unexpected value in status register"),
            Error::__NonExhaustive(_) => unreachable!(),
        }
    }
}
