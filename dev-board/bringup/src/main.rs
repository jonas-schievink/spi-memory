use mcp2210::*;
use std::{io::stdin, process};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    powerup()?;
    let mut mcp = connect_to_mcp2210()?;
    test_gpio_shorts(&mut mcp)?;
    test_bus_release(&mut mcp)?;
    Ok(())
}

struct Step {
    name: &'static str,
    task: &'static str,
    notes: &'static str,
}

impl Step {
    fn show(&self) {
        let common_indent = self
            .notes
            .lines()
            .filter_map(|line| line.find(|c: char| !c.is_whitespace()))
            .max()
            .unwrap_or(0);

        let text = self
            .notes
            .lines()
            .map(|line| line.get(common_indent..).unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        let text = text.trim();

        println!();
        println!("Step: {}", self.name);
        if !self.task.is_empty() {
            println!("=> {}", self.task);
            println!();
        }
        println!("{}", text);
        println!();
        println!("Press <Enter> to continue. Press Ctrl+C to exit.");

        match stdin().read_line(&mut String::new()) {
            Ok(0) => {
                eprintln!("stdin was closed, exiting");
                process::exit(1);
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("failed to read from stdin ({}), exiting", e);
                process::exit(1);
            }
        }
    }
}

fn powerup() -> Result<()> {
    Step {
        name: "Powering up",
        task: "Connect the board to the computer",
        notes: r"
            Make sure that the board is *not* connected to a Raspberry Pi.

            The LEDs should now look like this:

            +---------+
            |  POWER  |
            +-----+---+
            |  PI |   |
            | USB | X |
            +-----+---+

            +---------+
            |  MASTER |
            +-----+---+
            |  PI |   |
            | USB | X |
            +-----+---+

            There should be 5 V on the 5V and VBUS test points, and 3.3 V on the 3V3 test point.
        ",
    }
    .show();
    Ok(())
}

fn connect_to_mcp2210() -> Result<Mcp2210> {
    Step {
        name: "Initial connection",
        task: "Connect the board to the computer",
        notes: r"
            Ensure that the user running this program has permissions to access the MCP2210 (eg.
            by installing a proper udev rule).

            Also make sure that the board is not connected to a Raspberry Pi.
        ",
    }
    .show();

    let devices = mcp2210::scan_devices()?;
    match devices.len() {
        0 => {
            return Err("No MCP2210 device found (are the access permissions correct?)".into());
        }
        1 => {
            let mcp = Mcp2210::open(&devices[0])?;
            println!("Successfully opened '{}'", devices[0].display());
            Ok(mcp)
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

fn test_gpio_shorts(mcp: &mut Mcp2210) -> Result<()> {
    Step {
        name: "Testing for shorted GPIOs",
        task: "",
        notes: r"
            The tool will now automatically check for shorted GPIOs.
        ",
    }
    .show();

    mcp.set_gpio_direction(GpioDirection::ALL_INPUTS)?;
    mcp.set_gpio_value(GpioValue::ALL_LOW)?;

    let mut settings = mcp.get_chip_settings()?;
    settings.gp0_mode = PinMode::Gpio;
    settings.gp1_mode = PinMode::Gpio;
    settings.gp2_mode = PinMode::Gpio;
    settings.gp3_mode = PinMode::Gpio;
    settings.gp4_mode = PinMode::Gpio;
    settings.gp5_mode = PinMode::Gpio;
    settings.gp6_mode = PinMode::Gpio;
    settings.gp7_mode = PinMode::Gpio;
    settings.gp8_mode = PinMode::Gpio;
    settings.default_gpio_direction = GpioDirection::ALL_INPUTS;
    settings.default_gpio_value = GpioValue::ALL_LOW;
    mcp.set_chip_settings(&settings)?;

    // All GPIOs except GP6 are pulled high externally.
    let expected = GpioValue::ALL_HIGH - GpioValue::GP6;
    let actual = mcp.get_gpio_value()?;
    if !actual.contains(expected) {
        println!(
            r"
Unexpected GPIO values:
    Expected {:?}
    Got      {:?}
             {:?} should not be set
            ",
            expected,
            actual,
            actual - expected,
        );
    }

    let tests = [
        (GpioDirection::GP0DIR, GpioValue::GP0),
        (GpioDirection::GP1DIR, GpioValue::GP1),
        (GpioDirection::GP2DIR, GpioValue::GP2),
        (GpioDirection::GP3DIR, GpioValue::GP3),
        (GpioDirection::GP4DIR, GpioValue::GP4),
        (GpioDirection::GP5DIR, GpioValue::GP5),
        (GpioDirection::GP6DIR, GpioValue::GP6),
        (GpioDirection::GP7DIR, GpioValue::GP7),
        (GpioDirection::GP8DIR, GpioValue::GP8),
    ];

    for (dir, expected) in &tests {
        // Pull down one GPIO at a time. If the pin's direction bit is set it's an input, so invert:
        let real_dir = GpioDirection::ALL_INPUTS - *dir;
        mcp.set_gpio_direction(real_dir)?;

        // Set the output state again. This is apparently necessary after setting directions.
        mcp.set_gpio_value(GpioValue::ALL_LOW)?;

        // We expect all to be high, except the one we pulled low...
        let expected = GpioValue::ALL_HIGH - GpioValue::GP6 - *expected;

        // ...however GP6 is not pulled anywhere, so mask it out.
        let actual = mcp.get_gpio_value()? - GpioValue::GP6;

        if actual != expected {
            println!(
                r"
Unexpected GPIO values after pulling {:?} low:
    Expected {:?}
    Got      {:?}
             {:?} should not be set
                ",
                dir,
                expected,
                actual,
                actual - expected,
            );
        }
    }

    // Put GPIOs back to undriven state.
    mcp.set_gpio_direction(GpioDirection::ALL_INPUTS)?;

    println!("GPIOs are working correctly!");

    Ok(())
}

fn test_bus_release(mcp: &mut Mcp2210) -> Result<()> {
    Step {
        name: "Testing the bus-release",
        task: "The tool will now release the bus",
        notes: "",
    }
    .show();

    let mut settings = mcp.get_chip_settings()?;
    settings.gp7_mode = PinMode::Dedicated;

    mcp.request_bus_release(true)?;
}

fn confirm_bus_released() -> Result<()> {
    Step {
        name: "Bus release confirmation",
        task: "Please confirm that the LEDs look like below",
        notes: r"
            The bus has been released. The LEDs should now look like this:

            +---------+
            |  POWER  |
            +-----+---+
            |  PI |   |
            | USB | X |
            +-----+---+

            +---------+
            |  MASTER |
            +-----+---+
            |  PI | X |
            | USB |   |
            +-----+---+
        ",
    }
    .show();

    Ok(())
}
