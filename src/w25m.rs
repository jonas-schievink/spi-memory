//! Provides an implementation for switching between the two dies stacked upon each other inside the W25M series
use crate::{BlockDevice, Error, Read};
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;
use core::marker::PhantomData;
use core::mem;

pub struct Die0;
pub struct Die1;

/// All dies which are supposed to be supported by the W25M struct have to implement this trait
pub trait Stackable<SPI: Transfer<u8>, CS: OutputPin>: BlockDevice<SPI, CS> + Read<SPI, CS> {
    fn new(spi: SPI, cs: CS) -> Self;
    /// Returns the SPI and chip select objects so they can be used elsewhere
    fn free(self) -> (SPI, CS);
}

#[derive(Debug)]
pub struct W25M<SPI, CS, DIE0, DIE1, DIE> 
{
    inner: Inner<DIE0, DIE1>,
    spi: Option<SPI>,
    cs: Option<CS>,
    _die: PhantomData<DIE>,
}

#[derive(Debug)]
enum Inner<DIE0, DIE1> {
    Die0(DIE0),
    Die1(DIE1),
    Dummy,
}

impl<DIE0, DIE1, SPI, CS, DIE> W25M<SPI, CS, DIE0, DIE1, DIE> 
    where DIE0: Stackable<SPI, CS>,
    DIE1: Stackable<SPI, CS>,
    SPI: Transfer<u8>,
    CS: OutputPin 
{
    pub fn new(spi: SPI, cs: CS) -> W25M<SPI, CS, DIE0, DIE1, Die0> {
        W25M{
            inner: Inner::Die0(DIE0::new(spi, cs)),
            spi: None,
            cs: None,
            _die: PhantomData
        }
    }

    // TODO: This is a duplicate from the series25 implementation, deduplicate this
    fn command(&mut self, bytes: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        // If the SPI transfer fails, make sure to disable CS anyways
        self.cs.as_mut().unwrap().set_low().map_err(Error::Gpio)?;
        let spi_result = self.spi.as_mut().unwrap().transfer(bytes).map_err(Error::Spi);
        self.cs.as_mut().unwrap().set_high().map_err(Error::Gpio)?;
        spi_result?;
        Ok(())
    }
}

impl<DIE0, DIE1, SPI, CS> W25M<SPI, CS, DIE0, DIE1, Die0> 
    where DIE0: Stackable<SPI, CS>,
    DIE1: Stackable<SPI, CS>,
    SPI: Transfer<u8>,
    CS: OutputPin 
{
    pub fn switch_die(mut self) -> Result<W25M<SPI, CS, DIE0, DIE1, Die1>, Error<SPI, CS>> {
        let (spi, cs) = match mem::replace(&mut self.inner, Inner::Dummy) {
            Inner::Die0(die) => die.free(),
            _ => unreachable!()
        };
        mem::replace(&mut self.spi, Some(spi));
        mem::replace(&mut self.cs, Some(cs));
        
        self.command(&mut [0xC2, 0x01])?;

        let spi = mem::replace(&mut self.spi, None).unwrap();
        let cs = mem::replace(&mut self.cs, None).unwrap();

        Ok(W25M{
            inner: Inner::Die1(DIE1::new(spi, cs)),
            spi: None,
            cs: None,
            _die: PhantomData
        })
    }
}

impl<DIE0, DIE1, SPI, CS> W25M<SPI, CS, DIE0, DIE1, Die1> 
    where DIE0: Stackable<SPI, CS>,
    DIE1: Stackable<SPI, CS>,
    SPI: Transfer<u8>,
    CS: OutputPin 
{
    pub fn switch_die(mut self) -> Result<W25M<SPI, CS, DIE0, DIE1, Die0>, Error<SPI, CS>> {
        let (spi, cs) = match mem::replace(&mut self.inner, Inner::Dummy) {
            Inner::Die1(die) => die.free(),
            _ => unreachable!()
        };
        mem::replace(&mut self.spi, Some(spi));
        mem::replace(&mut self.cs, Some(cs));
        
        self.command(&mut [0xC2, 0x00])?;

        let spi = mem::replace(&mut self.spi, None).unwrap();
        let cs = mem::replace(&mut self.cs, None).unwrap();

        Ok(W25M{
            inner: Inner::Die0(DIE0::new(spi, cs)),
            spi: None,
            cs: None,
            _die: PhantomData
        })
    }
}