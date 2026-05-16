use crate::usb::wait_for_recovery_usb_device;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::usb::mcu_update::McuUpdateProgressReporter;

#[derive(Debug, clap::Args)]
pub struct Args {
    pub path: PathBuf,
}

struct RecoveryProgressWrapper {
    bar: ProgressBar,
    device_style: ProgressStyle,
    device_phase: bool,
}

impl RecoveryProgressWrapper {
    fn new(bar: ProgressBar, device_style: ProgressStyle) -> Self {
        Self {
            bar,
            device_style,
            device_phase: false,
        }
    }

    fn enter_device_phase(&mut self) {
        if self.device_phase {
            return;
        }

        self.device_phase = true;
        self.bar.set_length(100);
        self.bar.set_position(0);
        self.bar.set_style(self.device_style.clone());
        self.bar.set_message("flashing mcu on device");
    }

    fn into_inner(self) -> ProgressBar {
        self.bar
    }
}

impl McuUpdateProgressReporter for RecoveryProgressWrapper {
    fn transmit(&mut self, length: usize) {
        self.bar.inc(length as u64);
    }

    fn device_progress(&mut self, percent: u8) {
        self.enter_device_phase();
        self.bar.set_position(percent as u64);
    }

    fn device_finished(&mut self, status_ok: bool) {
        self.enter_device_phase();
        if status_ok {
            self.bar.set_position(100);
            self.bar.set_message("mcu flash complete");
        } else {
            self.bar.set_message("mcu flash failed");
        }
    }
}

pub async fn run(args: Args) -> Result<(), anyhow::Error> {
    let mcu_bytes = std::fs::read(args.path)?;
    let mut api = hidapi::HidApi::new()?;

    let bar = ProgressBar::new(mcu_bytes.len() as u64);
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(ProgressStyle::with_template(
        "{decimal_bytes}/{decimal_total_bytes} {bar} {bytes_per_sec} ({elapsed_precise} / {eta_precise}) {msg}",
    )?);

    bar.set_message("waiting for recovery device");
    let (recovery_device, _recovery_runner) =
        wait_for_recovery_usb_device(&mut api, Duration::from_secs(120)).await?;

    bar.set_message("uploading recovery mcu");
    let device_style = ProgressStyle::with_template("{percent}% {bar} ({elapsed_precise}) {msg}")?;
    let mut wrapper = RecoveryProgressWrapper::new(bar, device_style);
    recovery_device
        .update_recovery_mcu_with_progress(&mcu_bytes, &mut wrapper)
        .await?;

    let bar = wrapper.into_inner();
    bar.finish_with_message("mcu recovery update complete");

    Ok(())
}
