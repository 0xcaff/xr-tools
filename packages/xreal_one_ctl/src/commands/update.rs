use crate::usb::open_normal_usb_device;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::usb::mcu_update::{McuUpdate, McuUpdateProgressReporter};
use xreal_one_driver::proto::usb::pilot_update::PilotUpdateProgressReporter;

#[derive(Debug, clap::Args)]
pub struct Args {
    pub mcu_path: PathBuf,
    pub pilot_path: PathBuf,
}

pub async fn run(args: Args) -> Result<(), anyhow::Error> {
    let mcu_bytes = std::fs::read(args.mcu_path)?;
    let pilot_bytes = std::fs::read(args.pilot_path)?;

    let mcu_update = McuUpdate::parse(&mcu_bytes)?;

    let api = hidapi::HidApi::new()?;
    let (device, _usb_runner) = open_normal_usb_device(&api)?;

    let bar = ProgressBar::new((pilot_bytes.len() + mcu_update.size()) as u64);
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(ProgressStyle::with_template(
        "{decimal_bytes}/{decimal_total_bytes} {bar} {bytes_per_sec} ({elapsed_precise} / {eta_precise}) {msg}",
    )?);

    let bar = {
        bar.set_message("updating kernel");

        struct ProgressWrapper(ProgressBar);

        impl McuUpdateProgressReporter for ProgressWrapper {
            fn transmit(&mut self, length: usize) {
                self.0.inc(length as u64);
            }
        }

        let mut wrapper = ProgressWrapper(bar);

        device
            .update_mcu_with_progress(mcu_update, &mut wrapper)
            .await?;

        wrapper.0
    };

    let bar = {
        bar.set_message("updating pilot");
        struct ProgressWrapper(ProgressBar);

        impl PilotUpdateProgressReporter for ProgressWrapper {
            fn transmit(&mut self, length: usize) {
                self.0.inc(length as u64);
            }
        }

        let mut wrapper = ProgressWrapper(bar);

        device
            .update_pilot_with_progress(&pilot_bytes, &mut wrapper)
            .await?;

        wrapper.0
    };

    bar.finish();

    Ok(())
}
