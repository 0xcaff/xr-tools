use crate::usb::open_normal_usb_device;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::usb::dsp_update::{DspUpdatePhase, DspUpdateProgressReporter};

#[derive(Debug, clap::Args)]
pub struct Args {
    pub path: PathBuf,
}

struct DspProgressWrapper {
    bar: ProgressBar,
    device_style: ProgressStyle,
    device_phase: bool,
}

impl DspProgressWrapper {
    fn new(bar: ProgressBar, device_style: ProgressStyle) -> Self {
        Self {
            bar,
            device_style,
            device_phase: false,
        }
    }

    fn phase_offset(phase: DspUpdatePhase) -> u64 {
        match phase {
            DspUpdatePhase::WriteFlash => 0,
            DspUpdatePhase::Boot => 100,
            DspUpdatePhase::ReadFlash => 200,
        }
    }

    fn phase_message(phase: DspUpdatePhase) -> &'static str {
        match phase {
            DspUpdatePhase::WriteFlash => "writing dsp flash",
            DspUpdatePhase::Boot => "booting dsp",
            DspUpdatePhase::ReadFlash => "verifying dsp flash",
        }
    }

    fn enter_device_phase(&mut self) {
        if self.device_phase {
            return;
        }

        self.device_phase = true;
        self.bar.set_length(300);
        self.bar.set_position(0);
        self.bar.set_style(self.device_style.clone());
    }

    fn into_inner(self) -> ProgressBar {
        self.bar
    }
}

impl DspUpdateProgressReporter for DspProgressWrapper {
    fn transmit(&mut self, length: usize) {
        self.bar.inc(length as u64);
    }

    fn device_progress(&mut self, phase: DspUpdatePhase, percent: u8) {
        self.enter_device_phase();
        self.bar
            .set_position(Self::phase_offset(phase) + percent as u64);
        self.bar
            .set_message(format!("{} {}%", Self::phase_message(phase), percent));
    }

    fn device_finished(&mut self, phase: DspUpdatePhase, status_ok: bool) {
        self.enter_device_phase();
        if status_ok {
            self.bar.set_position(Self::phase_offset(phase) + 100);
            self.bar
                .set_message(format!("{} complete", Self::phase_message(phase)));
        } else {
            self.bar
                .set_message(format!("{} failed", Self::phase_message(phase)));
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

    let device_style = ProgressStyle::with_template("{percent}% {bar} ({elapsed_precise}) {msg}")?;
    let mut wrapper = DspProgressWrapper::new(bar, device_style);
    device
        .update_dsp_with_progress(&dsp_bytes, &mut wrapper)
        .await?;

    let bar = wrapper.into_inner();
    bar.finish_with_message("dsp update complete");

    Ok(())
}
