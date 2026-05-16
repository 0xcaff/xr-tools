use crate::usb::open_normal_usb_device;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::usb::pilot_update::PilotUpdateProgressReporter;

#[derive(Debug, clap::Args)]
pub struct Args {
    pub path: PathBuf,
}

struct ProgressWrapper(ProgressBar);

impl PilotUpdateProgressReporter for ProgressWrapper {
    fn transmit(&mut self, length: usize) {
        self.0.inc(length as u64);
    }
}

pub async fn run(args: Args) -> Result<(), anyhow::Error> {
    let pilot_bytes = std::fs::read(args.path)?;

    let api = hidapi::HidApi::new()?;
    let (device, _usb_runner) = open_normal_usb_device(&api)?;

    let bar = ProgressBar::new(pilot_bytes.len() as u64);
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(ProgressStyle::with_template(
        "{decimal_bytes}/{decimal_total_bytes} {bar} {bytes_per_sec} ({elapsed_precise} / {eta_precise}) {msg}",
    )?);
    bar.set_message("updating pilot");

    let mut wrapper = ProgressWrapper(bar);
    device
        .update_pilot_with_progress(&pilot_bytes, &mut wrapper)
        .await?;

    wrapper.0.finish_with_message("pilot update complete");

    Ok(())
}
