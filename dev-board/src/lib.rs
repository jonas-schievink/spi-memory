//! Internal support crate for the dev board.

// #0 = W25Q16JVSNIQ 16Mbit 133 MHz Flash

use bitflags::bitflags;
use embedded_hal::blocking::spi;
use embedded_hal::digital::v2::OutputPin;
use log::info;
use mcp2210::*;
use std::fmt::Debug;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

/// A connection to a dev board.
pub trait DevBoard
where
    <Self::ChipSelect as OutputPin>::Error: Debug,
{
    type ChipSelect: OutputPin;
    type Spi: spi::Transfer<u8>;

    /// Controls `CVCC`.
    fn set_chip_power(&mut self, on: bool) -> Result<()>;

    /// Sets the SPI transfer frequency.
    ///
    /// The MCP2210 has a max. frequency of 12 MHz.
    fn set_freq(&mut self, hz: u32) -> Result<()>;

    /// Obtains access to the a test chip.
    ///
    /// # Parameters
    ///
    /// * `chip`: The chip number as shows on the back of the PCB. 0-15.
    ///
    /// # Panics
    ///
    /// This function will panic when `chip` is not in range 0-15.
    fn access(&mut self, chip: u8) -> (Self::Spi, Self::ChipSelect);
}

/// Opens a dev board connected via USB.
pub fn open_usb() -> Result<UsbConnection> {
    let devices = mcp2210::scan_devices()?;
    match devices.len() {
        0 => {
            return Err("No MCP2210 device found (are the access permissions correct?)".into());
        }
        1 => {
            let mcp = Mcp2210::open(&devices[0])?;
            info!("successfully opened '{}'", devices[0].display());

            // Reset the chips
            let mut board = UsbConnection {
                mcp: McpRef {
                    mcp: Arc::new(Mutex::new(mcp)),
                },
            };

            // Make sure that the MCP is configured correctly.
            {
                let mut mcp = board.mcp();
                verify_mcp_nvram(&mut mcp)?;
            }

            board.set_chip_power(false)?;
            thread::sleep(Duration::from_millis(10));
            board.set_chip_power(true)?;
            Ok(board)
        }
        n => {
            let paths = devices
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");

            return Err(format!(
                "Found {} MCP2210 devices. Please remove all but one. ({})",
                n, paths
            )
            .into());
        }
    }
}

pub const SAFE_CHIP_SETTINGS: ChipSettings = ChipSettings {
    // CSBIT0-3 are GPIOs
    gp0_mode: PinMode::Gpio,
    gp1_mode: PinMode::Gpio,
    gp2_mode: PinMode::Gpio,
    gp3_mode: PinMode::Gpio,
    // \CSEN
    gp4_mode: PinMode::Gpio,
    // CHIP_PWR
    gp5_mode: PinMode::Gpio,
    gp6_mode: PinMode::Gpio,
    // SPI_REL_ACK
    gp7_mode: PinMode::Dedicated,
    // \SPI_RELEASE
    gp8_mode: PinMode::Dedicated,
    default_gpio_value: GpioValue::ALL_LOW,
    default_gpio_direction: GpioDirection::ALL_INPUTS,
    remote_wakeup: false,
    interrupt_mode: InterruptMode::None,
    bus_release: true,
    nvram_access_control: NvramAccessControl::None,
};

pub fn verify_mcp_nvram(mcp: &mut Mcp2210) -> Result<()> {
    let chip_settings = mcp.get_nvram_chip_settings()?;
    if chip_settings != SAFE_CHIP_SETTINGS {
        return Err(format!(
            "NVRAM chip settings were not set correctly. Expected: {:?}; Got: {:?}",
            SAFE_CHIP_SETTINGS, chip_settings
        )
        .into());
    }

    let chip_settings = mcp.get_chip_settings()?;
    if chip_settings != SAFE_CHIP_SETTINGS {
        return Err(format!(
            "Active chip settings were not set to NVRAM settings. Expected: {:?}; Got: {:?}",
            SAFE_CHIP_SETTINGS, chip_settings
        )
        .into());
    }

    Ok(())
}

#[derive(Clone)]
pub struct McpRef {
    mcp: Arc<Mutex<Mcp2210>>,
}

bitflags! {
    struct Gpios: u16 {
        const CSBIT0 = (1 << 0);
        const CSBIT1 = (1 << 1);
        const CSBIT2 = (1 << 2);
        const CSBIT3 = (1 << 3);
        const N_CSEN = (1 << 4);
        const N_CHIP_PWR = (1 << 5);
    }
}

impl McpRef {
    fn get_gpios(&self) -> Result<Gpios> {
        let mut mcp = self.mcp();
        let dir = mcp.get_gpio_direction()?;
        Ok(Gpios::from_bits_truncate(dir.bits()))
    }

    /// Sets the open-drain GPIO values.
    fn set_gpios(&mut self, gpios: Gpios) -> Result<()> {
        // Direction bits: 1 == input, 0 == output
        // Value bits are all-0
        let dir = GpioDirection::from_bits(gpios.bits()).unwrap();

        let mut mcp = self.mcp();
        mcp.set_gpio_direction(dir)?;
        Ok(())
    }

    fn mcp(&self) -> impl DerefMut<Target = Mcp2210> + '_ {
        self.mcp.lock().unwrap_or_else(|err| err.into_inner())
    }
}

pub struct UsbConnection {
    mcp: McpRef,
}

impl UsbConnection {
    fn mcp(&self) -> impl DerefMut<Target = Mcp2210> + '_ {
        self.mcp.mcp()
    }
}

impl DevBoard for UsbConnection {
    type ChipSelect = UsbChipSelect;
    type Spi = UsbSpi;

    fn set_chip_power(&mut self, on: bool) -> Result<()> {
        let gpios = self.mcp.get_gpios()?;
        if on {
            self.mcp.set_gpios(gpios & !Gpios::N_CHIP_PWR);
        } else {
            self.mcp.set_gpios(gpios | Gpios::N_CHIP_PWR);
        }

        Ok(())
    }

    fn set_freq(&mut self, hz: u32) -> Result<()> {
        let mut mcp = self.mcp();
        let mut settings = mcp.get_spi_transfer_settings()?;
        settings.bit_rate = hz;
        mcp.set_spi_transfer_settings(&settings)?;
        Ok(())
    }

    fn access(&mut self, chip: u8) -> (Self::Spi, Self::ChipSelect) {
        assert!(chip < 16);

        let spi = UsbSpi {
            mcp: self.mcp.clone(),
        };
        let cs = UsbChipSelect {
            mcp: self.mcp.clone(),
            cs: chip,
        };

        (spi, cs)
    }
}

pub struct UsbChipSelect {
    mcp: McpRef,
    /// Chip select pin number (0-15).
    cs: u8,
}

impl OutputPin for UsbChipSelect {
    type Error = Error;

    /// Assert the chip select line, pulling it low and pulling all other CS lines high.
    fn set_low(&mut self) -> Result<()> {
        let mut gpios = self.mcp.get_gpios()?;

        let cs = Gpios::from_bits(u16::from(self.cs & 0b1111)).unwrap();
        let csmask = Gpios::from_bits(0b1111).unwrap();

        gpios &= !csmask;
        gpios |= cs;
        gpios &= !Gpios::N_CSEN; // Pull \CSEN low

        self.mcp.set_gpios(gpios)?;
        Ok(())
    }

    /// Deassert the chip select line.
    fn set_high(&mut self) -> Result<()> {
        let mut gpios = self.mcp.get_gpios()?;
        gpios |= Gpios::N_CSEN;
        self.mcp.set_gpios(gpios)?;
        Ok(())
    }
}

/// MCP2210 wrapper that implements the embedded-hal SPI trait.
pub struct UsbSpi {
    mcp: McpRef,
}

impl spi::Transfer<u8> for UsbSpi {
    type Error = Error;

    fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'w [u8]> {
        let mut buf = Vec::with_capacity(words.len());
        let mut mcp = self.mcp.mcp();
        mcp.spi_transfer_to_end(words, &mut buf)?;

        assert_eq!(buf.len(), words.len());
        words.copy_from_slice(&buf);
        Ok(words)
    }
}
