
use log::{info, error};

use structopt::StructOpt;

pub use linux_embedded_hal::sysfs_gpio::{Direction, Error as PinError};
pub use linux_embedded_hal::{spidev, Delay, Pin as Pindev, Spidev, spidev::SpiModeFlags};

use simplelog::{TermLogger, LevelFilter, TerminalMode};

use ihex::{Record, Reader};

use spi_memory::{Read, BlockDevice, series25::Flash};

#[derive(Debug, PartialEq, StructOpt)]
struct Options {
    #[structopt(subcommand)]
    operation: Operations,

    /// SPI device
    #[structopt(long, default_value="/dev/spidev0.0", env = "SPI_DEV")]
    spi_dev: String,

    /// SPI baud rate
    #[structopt(long, default_value = "1000000", env = "SPI_BAUD")]
    spi_baud: u32,

    /// Chip Select (output) pin
    #[structopt(long, default_value = "8", env = "CS_PIN")]
    cs_pin: u64,

    /// Configure log level
    #[structopt(long, default_value = "info", env="LOG_LEVEL")]
    log_level: LevelFilter,
}

#[derive(Debug, PartialEq, StructOpt)]
pub enum Operations {
    /// Read device information
    Info,
    /// Read data from the device
    Read {
        /// Flash address for read start in hex
        #[structopt(parse(try_from_str = parse_hex))]
        address: u32,
        /// Length of flash read in bytes
        #[structopt()]
        length: u32,
    },
    /// Write data to the specified block
    Write {
        /// Flash address for write start in hex
        #[structopt(parse(try_from_str = parse_hex))]
        address: u32,

        // Data to write in hexadecimal
        #[structopt(long)]
        data: HexData,
    },
    /// Erase block(s) starting at the specified address
    EraseBlocks {
        /// Flash address for block erase in hex
        #[structopt(parse(try_from_str = parse_hex))]
        address: u32,

        /// Number of blocks to erase
        #[structopt(long, default_value="1")]
        count: u32,
    },
    /// Dump flash into a hex file
    Dump {
        /// Flash address for read start in hex
        #[structopt(parse(try_from_str = parse_hex))]
        address: u32,
        
        /// Length of flash read in bytes
        #[structopt()]
        length: u32,

        /// Output ihex file
        #[structopt(long, default_value="dump.ihex")]
        file: String,
    },
    /// Load flash from a hex file
    Load {
        /// Input ihex file
        file: String,
    },
    /// Erase all data on the device
    EraseAll,
}

#[derive(Debug, PartialEq)]
pub struct HexData(Vec<u8>);

impl std::str::FromStr for HexData {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        hex::decode(s).map(HexData)
    }
}

fn parse_hex(s: &str) -> Result<u32, std::num::ParseIntError> {
    u32::from_str_radix(s, 16)
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>>{
    // Load options
    let mut opts = Options::from_args();

    // Setup logging
    TermLogger::init(opts.log_level, simplelog::Config::default(), TerminalMode::Mixed).unwrap();

    // Connect and configure GPIO pin
    let cs_pin = Pindev::new(opts.cs_pin);

    cs_pin.export().unwrap();
    cs_pin.set_direction(Direction::Out).unwrap();

    // Connect and configure SPI device
    let mut spi = Spidev::open(opts.spi_dev).unwrap();

    let mut spi_config = spidev::SpidevOptions::new();
    spi_config.mode(SpiModeFlags::SPI_MODE_0 | SpiModeFlags::SPI_NO_CS);
    spi_config.max_speed_hz(opts.spi_baud);
    spi.configure(&spi_config).unwrap();

    // Instantiate SPI flash
    let mut flash = match Flash::init(spi, cs_pin) {
        Ok(f) => f,
        Err(e) => {
            error!("Error initialising flash: {:?}", e);
            return Ok(())
        }
    };

    // Read out ID to check we are connected
    let _id = match flash.read_jedec_id() {
        Ok(id) if id.mfr_code() != 0 => {
            info!("Flash ID: {:?}", id);
        },
        Ok(id) => {
            error!("Flash ID read failed ({:?}", id);
            return Ok(())
        },
        Err(e) => {
            error!("Flash ID read error: {:?}", e);
            return Ok(())
        }
    };

    // Perform the requested operation
    match &mut opts.operation {
        Operations::Info => (),
        Operations::Read{address, length} => {
            info!("Reading {} bytes from address 0x{:08x}", length, address);

            let mut buff = vec![0u8; *length as usize];
            flash.read(*address, &mut buff).unwrap();

            info!("Read: {:02x?}", buff);
        },
        Operations::Write{address, data} => {
            info!("Writing {} bytes to address 0x{:08x}", data.0.len(), address);

            flash.write_bytes(*address, &mut data.0).unwrap();

            info!("Write complete");
        },
        Operations::EraseBlocks{address, count} => {
            info!("Erasing {} blocks add address 0x{:08x}", count, address);

            flash.erase_sectors(*address, *count as usize).unwrap();

            info!("Sector erase complete")
        },
        Operations::EraseAll => {
            info!("Erasing all blocsk");

            flash.erase_all().unwrap();

            info!("Full erase complete");
        },
        Operations::Dump{address, length, file} => {
            info!("Reading {} bytes from address 0x{:08x} to file {}", length, address, &file);

            let mut buff = vec![0u8; *length as usize];
            flash.read(*address, &mut buff).unwrap();

            let mut records = Vec::new();
            for (c, chunk) in buff.chunks_mut(32).enumerate() {
                records.push(Record::Data{ offset: (*address as usize + c * 32 as usize) as u16, value: chunk.to_vec() });
            }
            records.push(Record::EndOfFile);

            let data = ihex::create_object_file_representation(&records).unwrap();

            std::fs::write(file, data).unwrap();

            info!("Dump complete");
        },
        Operations::Load{file} => {
            info!("Loading file {}", file);

            let data = String::from_utf8(std::fs::read(&file).unwrap()).unwrap();

            let reader = Reader::new(&data);

            for record in reader {
                match record {
                    Ok(Record::Data{offset, mut value}) => {
                        info!("Writing {} bytes at address 0x{:08x}", value.len(), offset);
                        flash.write_bytes(offset as u32, &mut value).unwrap();
                    },
                    Ok(Record::EndOfFile) => (),
                    Err(e) => {
                        error!("Reader error: {:?}", e);
                        return Ok(())
                    }
                    _ => {
                        error!("Unrecognised record: {:?}", record);
                        return Ok(())
                    }
                }
            }

            info!("Load complete");
        },
    }

    Ok(())
}
