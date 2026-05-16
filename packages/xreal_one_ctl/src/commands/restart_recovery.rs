use crate::usb::open_normal_usb_device;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub async fn run() -> Result<(), anyhow::Error> {
    let api = hidapi::HidApi::new()?;
    let (device, usb_runner) = open_normal_usb_device(&api)?;

    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(ProgressStyle::with_template(
        "{spinner} {elapsed_precise} {msg}",
    )?);

    bar.set_message("setting recovery boot flag");
    device.set_uboot_upgrade_flag().await?;

    bar.set_message("rebooting into recovery");
    device.system_reboot().await?;

    drop(device);
    usb_runner.abort();
    bar.finish_with_message("recovery reboot requested");

    Ok(())
}
