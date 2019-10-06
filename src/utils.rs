use crate::Error;
use core::fmt;
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;

pub struct HexSlice<T>(pub T)
where
    T: AsRef<[u8]>;

impl<T: AsRef<[u8]>> fmt::Debug for HexSlice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        for (i, byte) in self.0.as_ref().iter().enumerate() {
            if i != 0 {
                f.write_str(", ")?;
            }
            write!(f, "{:02x}", byte)?;
        }
        f.write_str("]")
    }
}

pub(crate) fn spi_command<SPI, CS>(
    spi: &mut SPI,
    cs: &mut CS,
    command: &mut [u8],
) -> Result<(), Error<SPI, CS>>
where
    SPI: Transfer<u8>,
    CS: OutputPin,
{
    cs.set_low().map_err(Error::Gpio)?;
    let spi_result = spi.transfer(command).map_err(Error::Spi);
    cs.set_high().map_err(Error::Gpio)?;
    spi_result?;
    Ok(())
}
