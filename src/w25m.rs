//! Provides an implementation for switching between the two dies stacked upon each other inside the W25M series
use crate::utils::spi_command;
use crate::{BlockDevice, Error, Read};
use core::marker::PhantomData;
use core::mem;
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;

#[allow(missing_debug_implementations)]
pub struct Die0;
#[allow(missing_debug_implementations)]
pub struct Die1;

/// All dies which are supposed to be supported by the W25M struct have to implement this trait
pub trait Stackable<SPI: Transfer<u8>, CS: OutputPin>:
    BlockDevice<SPI, CS> + Read<SPI, CS> + Sized
{
    fn new(spi: SPI, cs: CS) -> Result<Self, Error<SPI, CS>>;
    /// Returns the SPI and chip select objects so they can be used elsewhere
    fn free(self) -> (SPI, CS);
}

/// Driver for W25M SPI Flash chips.
///
/// # Type Parameters
///
/// * **`DIE0`**: The type of one of the two dies inside the W25M package
/// * **`DIE0`**: The type of the other of the two dies inside the W25M package
/// * **`DIE`**: A type state, used to indicate which of the two die's is the currently active one
#[derive(Debug)]
pub struct Flash<DIE0, DIE1, DIE> {
    inner: Inner<DIE0, DIE1>,
    _die: PhantomData<DIE>,
}

#[derive(Debug)]
enum Inner<DIE0, DIE1> {
    Die0(DIE0),
    Die1(DIE1),
    Dummy,
}

impl<DIE0, DIE1> Flash<DIE0, DIE1, Die0> {
    /// Creates a new W25M device
    ///
    /// At
    /// the moment the only way to call this function is sadly
    /// ```
    /// let mut flash: Flash<W25N<_, _>, W25N<_, _>, _> = Flash::init(spi, cs).unwrap();
    /// ```
    /// TODO: Improve this API, its not very convenient
    pub fn init<SPI, CS>(spi: SPI, cs: CS) -> Result<Flash<DIE0, DIE1, Die0>, Error<SPI, CS>>
    where
        SPI: Transfer<u8>,
        CS: OutputPin,
        DIE0: Stackable<SPI, CS>,
        DIE1: Stackable<SPI, CS>,
    {
        Ok(Flash {
            inner: Inner::Die0(DIE0::new(spi, cs)?),
            _die: PhantomData,
        })
    }
}

impl<DIE0, DIE1> Flash<DIE0, DIE1, Die0> {
    pub fn switch_die<SPI, CS>(mut self) -> Result<Flash<DIE0, DIE1, Die1>, Error<SPI, CS>>
    where
        DIE0: Stackable<SPI, CS>,
        DIE1: Stackable<SPI, CS>,
        SPI: Transfer<u8>,
        CS: OutputPin,
    {
        let (mut spi, mut cs) = match mem::replace(&mut self.inner, Inner::Dummy) {
            Inner::Die0(die) => die.free(),
            _ => unreachable!(),
        };
        let mut command = [0xC2, 0x01];
        spi_command(&mut spi, &mut cs, &mut command)?;

        Ok(Flash {
            inner: Inner::Die1(DIE1::new(spi, cs)?),
            _die: PhantomData,
        })
    }
}

impl<DIE0, DIE1> Flash<DIE0, DIE1, Die1> {
    pub fn switch_die<SPI, CS>(mut self) -> Result<Flash<DIE0, DIE1, Die0>, Error<SPI, CS>>
    where
        DIE0: Stackable<SPI, CS>,
        DIE1: Stackable<SPI, CS>,
        SPI: Transfer<u8>,
        CS: OutputPin,
    {
        let (mut spi, mut cs) = match mem::replace(&mut self.inner, Inner::Dummy) {
            Inner::Die1(die) => die.free(),
            _ => unreachable!(),
        };

        let mut command = [0xC2, 0x00];
        spi_command(&mut spi, &mut cs, &mut command)?;

        Ok(Flash {
            inner: Inner::Die0(DIE0::new(spi, cs)?),
            _die: PhantomData,
        })
    }
}

impl<DIE0, DIE1, SPI, CS, DIE> BlockDevice<SPI, CS> for Flash<DIE0, DIE1, DIE>
where
    DIE0: Stackable<SPI, CS>,
    DIE1: Stackable<SPI, CS>,
    SPI: Transfer<u8>,
    CS: OutputPin,
{
    fn erase(&mut self, addr: u32, amount: usize) -> Result<(), Error<SPI, CS>> {
        match &mut self.inner {
            Inner::Die0(die) => die.erase(addr, amount),
            Inner::Die1(die) => die.erase(addr, amount),
            _ => unreachable!(),
        }
    }

    fn erase_all(&mut self) -> Result<(), Error<SPI, CS>> {
        match &mut self.inner {
            Inner::Die0(die) => die.erase_all(),
            Inner::Die1(die) => die.erase_all(),
            _ => unreachable!(),
        }
    }

    fn write_bytes(&mut self, addr: u32, data: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        match &mut self.inner {
            Inner::Die0(die) => die.write_bytes(addr, data),
            Inner::Die1(die) => die.write_bytes(addr, data),
            _ => unreachable!(),
        }
    }
}

impl<DIE0, DIE1, SPI, CS, DIE> Read<SPI, CS> for Flash<DIE0, DIE1, DIE>
where
    DIE0: Stackable<SPI, CS>,
    DIE1: Stackable<SPI, CS>,
    SPI: Transfer<u8>,
    CS: OutputPin,
{
    fn read(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        match &mut self.inner {
            Inner::Die0(die) => die.read(addr, buf),
            Inner::Die1(die) => die.read(addr, buf),
            _ => unreachable!(),
        }
    }
}
