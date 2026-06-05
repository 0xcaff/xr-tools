use anyhow::{bail, Context, Result};
use hidapi::HidApi;
use std::thread;
use std::time::Duration;
use viture_driver::{DisplayMode, UsbDevice, VitureLumaModel};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    if matches!(args.next().as_deref(), Some("--list")) {
        for mode in DisplayMode::STANDARD_MODES {
            println!("{mode}");
        }
        return Ok(());
    }

    let mode = std::env::args()
        .nth(1)
        .map(|arg| parse_display_mode(&arg))
        .transpose()?;

    let api = HidApi::new().context("failed to initialize hidapi")?;
    let model = VitureLumaModel::detect(&api).context("no VITURE Luma HID device detected")?;

    println!(
        "Opening VITURE Luma pid=0x{:04x}",
        model.control_device.product_id()
    );

    let device = UsbDevice::open(&api, model)?;
    let before = device.display_mode()?;
    println!("display before: {before}");

    if let Some(mode) = mode {
        println!("setting display mode: {mode}");
        let echoed = device.set_display_mode(mode)?;
        println!("set response: {echoed}");
        thread::sleep(Duration::from_secs(2));

        let after = device.display_mode()?;
        println!("display after: {after}");
    }

    Ok(())
}

fn parse_display_mode(value: &str) -> Result<DisplayMode> {
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    let mode = u8::from_str_radix(value, 16)
        .or_else(|_| value.parse::<u8>())
        .with_context(|| format!("failed to parse display mode {value:?}"))?;

    if !DisplayMode::STANDARD_MODES
        .iter()
        .any(|standard_mode| standard_mode.wire_value() == mode)
    {
        bail!("unsupported SDK display mode 0x{mode:02x}; pass --list to see known modes");
    }

    DisplayMode::from_wire_value(mode)
}
