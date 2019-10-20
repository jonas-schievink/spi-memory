//! Example application.

use dev_board::DevBoard;
use embedded_hal::blocking::spi;
use embedded_hal::digital::v2::OutputPin;
use spi_memory::series25::Flash;
use std::fmt::Debug;
use std::process;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let board = dev_board::open_usb()?;

    example(board)
}

/// Generic example that works with any `DevBoard` impl.
fn example<B>(mut board: B) -> Result<()>
where
    B: DevBoard,
    <B::ChipSelect as OutputPin>::Error: Debug,
    <B::Spi as spi::Transfer<u8>>::Error: Debug,
{
    println!("Accessing chip #0");
    let (spi, cs) = board.access(0);
    let mut flash = Flash::init(spi, cs).dbg_err("flash init")?;
    let id = flash.read_jedec_id().dbg_err("read jedec id")?;
    println!("JEDEC ID: {:?}", id);
    Ok(())
}

trait ResultExt<T, E> {
    fn dbg_err(self, msg: &str) -> std::result::Result<T, Error>;
}

impl<T, E: Debug> ResultExt<T, E> for std::result::Result<T, E> {
    fn dbg_err(self, msg: &str) -> std::result::Result<T, Error> {
        self.map_err(|e| format!("{}: {:?}", msg, e).into())
    }
}
