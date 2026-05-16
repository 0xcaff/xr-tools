use crate::usb::{try_open_normal_usb_device, wait_for_recovery_usb_device};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use xreal_one_driver::proto::usb::mcu_update::McuUpdateProgressReporter;

#[derive(Debug, clap::Args)]
pub struct Args {
    pub mcu_path: PathBuf,
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
    let mcu_bytes = std::fs::read(args.mcu_path)?;

    let mut api = hidapi::HidApi::new()?;

    let bar = ProgressBar::new(mcu_bytes.len() as u64);
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(ProgressStyle::with_template(
        "{decimal_bytes}/{decimal_total_bytes} {bar} {bytes_per_sec} ({elapsed_precise} / {eta_precise}) {msg}",
    )?);
    bar.set_message("looking for running device");

    api.refresh_devices()?;
    if let Some((normal_device, normal_runner)) = try_open_normal_usb_device(&api)? {
        bar.set_message("setting recovery boot flag");
        normal_device.set_uboot_upgrade_flag().await?;

        bar.set_message("rebooting into recovery");
        normal_device.system_reboot().await?;

        drop(normal_device);
        normal_runner.abort();
        sleep(Duration::from_secs(1)).await;
    } else {
        bar.set_message("running device not found; waiting for recovery");
    }

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
