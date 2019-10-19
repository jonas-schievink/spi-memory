use mcp2210::*;
use std::{io::stdin, mem, process, thread, time::Duration};

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
    test_chip_power(&mut mcp)?;
    configure_nvram(&mut mcp)?;
    verify_nvram(&mut mcp)?;

    println!();
    println!("Bringup Complete. The tool will now exit. Bye!");
    Ok(())
}

const SAFE_CHIP_SETTINGS: ChipSettings = ChipSettings {
    // CSBIT0-3 are GPIOs
    gp0_mode: PinMode::Gpio,
    gp1_mode: PinMode::Gpio,
    gp2_mode: PinMode::Gpio,
    gp3_mode: PinMode::Gpio,
    // \CSEN
    gp4_mode: PinMode::ChipSelect,
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

struct Step {
    name: &'static str,
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
        println!();
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
        notes: r"
            Make sure that the board is *not* connected to a Raspberry Pi.

            Connect the board to the computer using a Micro USB cable.

            After connecting, there should be 5 V on the 5V and VBUS test points, and 3.3 V on the
            3V3 test point.

            The POWER LEDs should now look like this:

            +---------+
            |  POWER  |
            +-----+---+
            |  PI |   |
            | USB | X |
            +-----+---+

            The MASTER LEDs may be in an undefined state if the MCP2210 has not yet been configured
            by this tool.
        ",
    }
    .show();
    Ok(())
}

fn connect_to_mcp2210() -> Result<Mcp2210> {
    Step {
        name: "Initial connection",
        notes: r"
            The bringup tool will now connect to the MCP2210.

            Ensure that the user running this program has permissions to access the MCP2210 (eg.
            by installing a proper udev rule).
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
        return Err(format!(
            r"
Unexpected GPIO values:
    Expected {:?}
    Got      {:?}
             {:?} should not be set
Are any pull-up resistors missing or improperly soldered?
            ",
            expected,
            actual,
            actual - expected,
        )
        .into());
    }

    let gpios = [
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

    for (dir, expected) in &gpios {
        // Pull down one GPIO at a time. If the pin's direction bit is set it's an input, so invert:
        let real_dir = GpioDirection::ALL_INPUTS - *dir;
        mcp.set_gpio_direction(real_dir)?;

        // Set the output state again. This is apparently necessary after setting directions.
        mcp.set_gpio_value(GpioValue::ALL_LOW)?;

        // We expect all to be high, except the one we pulled low. However, GP6 doesn't have an ext.
        // pullup, so we ignore it (it floats, so could be any value).
        let expected = GpioValue::ALL_HIGH - GpioValue::GP6 - *expected;

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
        name: "Bus Release preparation",
        notes: "The tool will now prepare to release the bus.",
    }
    .show();

    let mut settings = mcp.get_chip_settings()?;
    settings.gp7_mode = PinMode::Dedicated;
    settings.gp8_mode = PinMode::Dedicated;
    settings.bus_release = true;
    mcp.set_chip_settings(&settings)?;

    // Request bus release.
    // FIXME the bool parameter doesn't seem to control the REL_ACK pin
    mcp.request_bus_release(true)?;

    let status = mcp.get_chip_status()?;
    if status.is_bus_release_pending {
        return Err("Unexpected pending bus release".into());
    }
    if status.bus_owner != BusOwner::None {
        return Err(format!(
            "Unexpected bus owner `{:?}`, expected `None`",
            status.bus_owner
        )
        .into());
    }

    Step {
        name: "Bus Release test",
        notes: r"
            The MASTER LEDs should now look like this:

            +---------+
            |  MASTER |
            +-----+---+
            |  PI |   |
            | USB | X |
            +-----+---+

            After confirming this prompt, please request a bus release by briefly connecting the
            `REL` test point to ground. This should immediately switch the MASTER LEDs to look like
            this:

            +---------+
            |  MASTER |
            +-----+---+
            |  PI | X |
            | USB |   |
            +-----+---+

            The bringup tool will wait until the bus release has been requested.
        ",
    }
    .show();

    println!("Waiting for bus release request...");
    loop {
        let status = mcp.get_chip_status()?;
        if status.is_bus_release_pending {
            if status.bus_owner == BusOwner::ExternalMaster {
                println!("Success! Bus owner: {:?}", status.bus_owner);
                break;
            } else {
                return Err(format!(
                    "Bus release pending, but bus owner is `{:?}`, expected `ExternalMaster`",
                    status.bus_owner
                )
                .into());
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn test_chip_power(mcp: &mut Mcp2210) -> Result<()> {
    Step {
        name: "Chip Power Test",
        notes: r"
            Please ensure that there are now 0 V between the CVCC test point and ground and 3.3 V
            between the CHIP_PWR test point and ground.

            The bringup tool will now enable the SPI chip power.
        ",
    }
    .show();

    // Make CHIP_PWR (GP5) and output and pull it low.
    let gpios = mcp.get_gpio_value()?;
    mcp.set_gpio_value(gpios - GpioValue::GP5)?;

    let dir = mcp.get_gpio_direction()?;
    mcp.set_gpio_direction(dir - GpioDirection::GP5DIR)?;

    let v = mcp.get_gpio_value()?;
    assert!(!v.contains(GpioValue::GP5));

    Step {
        name: "Chip Power Confirmation",
        notes: r"
            Chip power enabled.

            Please ensure that there are now 0 V between CHIP_PWR and ground, and 3.3 V between
            CVCC and ground.
        ",
    }
    .show();

    Ok(())
}

fn configure_nvram(mcp: &mut Mcp2210) -> Result<()> {
    Step {
        name: "NVRAM Configuration",
        notes: r"
            The bringup tool will now configure the MCP2210 parameters to make it safe to use with
            a Raspberry Pi and USB.

            **WARNING**: This will change the persistent configuration of the chip!
        ",
    }
    .show();

    let old_chip_settings = mcp.get_nvram_chip_settings()?;
    let new_chip_settings = SAFE_CHIP_SETTINGS;

    println!("Old NVRAM Chip Settings: {:?}", old_chip_settings);
    println!("New NVRAM Chip Settings: {:?}", new_chip_settings);
    mcp.set_nvram_chip_settings(&new_chip_settings, None)?;

    mcp.set_nvram_usb_product_name("SPI Memory development board")?;

    Ok(())
}

fn verify_nvram(mcp: &mut Mcp2210) -> Result<()> {
    Step {
        name: "NVRAM Verification",
        notes: r"
            The tool will now verify that the NVRAM settings were written and applied correctly.
        ",
    }
    .show();

    // Wait for unplug by checking that `mcp` operations start erroring
    println!("Waiting for board to be unplugged...");
    while !mcp.get_chip_status().is_err() {
        thread::sleep(Duration::from_millis(200));
    }
    println!("Board unplugged, waiting for reconnection...");

    let devices = loop {
        let devices = mcp2210::scan_devices()?;
        if devices.is_empty() {
            thread::sleep(Duration::from_millis(300));
        } else {
            break devices;
        }
    };

    let new_mcp = match devices.len() {
        0 => unreachable!(),
        1 => Mcp2210::open(&devices[0])?,
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
    };

    println!("Successfully opened '{}'", devices[0].display());
    mem::replace(mcp, new_mcp);

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

    println!("Verification successful!");

    Ok(())
}
