use crate::usb::open_normal_usb_device;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::usb::dsp_update::{DspUpdatePhase, DspUpdateProgressReporter};

#[derive(Debug, clap::Args)]
pub struct Args {
    pub path: PathBuf,
}

struct DspProgressWrapper(ProgressBar);

impl DspUpdateProgressReporter for DspProgressWrapper {
    fn transmit(&mut self, length: usize) {
        self.0.inc(length as u64);
    }

    fn device_progress(&mut self, phase: DspUpdatePhase, status_ok: bool) {
        let phase = match phase {
            DspUpdatePhase::WriteFlash => "writing dsp flash",
            DspUpdatePhase::Boot => "booting dsp",
            DspUpdatePhase::ReadFlash => "verifying dsp flash",
        };

        if status_ok {
            self.0.set_message(phase);
        } else {
            self.0.set_message(format!("{phase} failed"));
        }
    }
}

pub async fn run(args: Args) -> Result<(), anyhow::Error> {
    let dsp_bytes = std::fs::read(args.path)?;

    let api = hidapi::HidApi::new()?;
    let (device, _usb_runner) = open_normal_usb_device(&api)?;

    let bar = ProgressBar::new(dsp_bytes.len() as u64);
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(ProgressStyle::with_template(
        "{decimal_bytes}/{decimal_total_bytes} {bar} {bytes_per_sec} ({elapsed_precise} / {eta_precise}) {msg}",
    )?);
    bar.set_message("uploading dsp");

    let mut wrapper = DspProgressWrapper(bar);
    device
        .update_dsp_with_progress(&dsp_bytes, &mut wrapper)
        .await?;

    wrapper.0.finish_with_message("dsp update complete");

    Ok(())
}
