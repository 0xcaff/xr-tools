use crate::usb::open_normal_usb_device;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::usb::mcu_update::{McuUpdate, McuUpdateProgressReporter};

#[derive(Debug, clap::Args)]
pub struct Args {
    pub path: PathBuf,
}

struct ProgressWrapper(ProgressBar);

impl McuUpdateProgressReporter for ProgressWrapper {
    fn transmit(&mut self, length: usize) {
        self.0.inc(length as u64);
    }
}

pub async fn run(args: Args) -> Result<(), anyhow::Error> {
    let mcu_bytes = std::fs::read(args.path)?;
    let mcu_update = McuUpdate::parse(&mcu_bytes)?;

    let api = hidapi::HidApi::new()?;
    let (device, _usb_runner) = open_normal_usb_device(&api)?;

    let bar = ProgressBar::new(mcu_update.size() as u64);
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(ProgressStyle::with_template(
        "{decimal_bytes}/{decimal_total_bytes} {bar} {bytes_per_sec} ({elapsed_precise} / {eta_precise}) {msg}",
    )?);
    bar.set_message("updating mcu");

    let mut wrapper = ProgressWrapper(bar);
    device
        .update_mcu_with_progress(mcu_update, &mut wrapper)
        .await?;

    wrapper.0.finish_with_message("mcu update complete");

    Ok(())
}
