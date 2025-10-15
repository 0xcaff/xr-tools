use clap::{Parser, Subcommand};
use futures::StreamExt;
use indicatif::ProgressStyle;
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::usb::mcu_update::{McuUpdate, McuUpdateProgressReporter};
use xreal_one_driver::proto::usb::pilot_update::PilotUpdateProgressReporter;
use xreal_one_driver::{ControlNetworkDevice, UsbDevice};

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
#[command(author, version, about, long_about = None)]
enum Commands {
    Update {
        mcu_path: PathBuf,
        pilot_path: PathBuf,
    },
    GetConfig,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    match args.command {
        Commands::Update {
            mcu_path,
            pilot_path,
        } => {
            let mcu_bytes = std::fs::read(mcu_path)?;
            let pilot_bytes = std::fs::read(pilot_path)?;

            let mcu_update = McuUpdate::parse(&mcu_bytes)?;

            let api = hidapi::HidApi::new()?;
            let device = UsbDevice::open(&api)?;

            let bar = indicatif::ProgressBar::new((pilot_bytes.len() + mcu_update.size()) as u64);
            bar.enable_steady_tick(Duration::from_millis(100));
            bar.set_style(ProgressStyle::with_template(
                "{decimal_bytes}/{decimal_total_bytes} {bar} {bytes_per_sec} ({elapsed_precise} / {eta_precise}) {msg}",
            )?);

            let bar = {
                bar.set_message("updating kernel");

                struct ProgressWrapper(indicatif::ProgressBar);

                impl McuUpdateProgressReporter for ProgressWrapper {
                    fn transmit(&mut self, length: usize) {
                        self.0.inc(length as u64);
                    }
                }

                let mut wrapper = ProgressWrapper(bar);

                device.update_mcu_with_progress(mcu_update, &mut wrapper)?;

                wrapper.0
            };

            let bar = {
                bar.set_message("updating pilot");
                struct ProgressWrapper(indicatif::ProgressBar);

                impl PilotUpdateProgressReporter for ProgressWrapper {
                    fn transmit(&mut self, length: usize) {
                        self.0.inc(length as u64);
                    }
                }

                let mut wrapper = ProgressWrapper(bar);

                device.update_pilot_with_progress(&pilot_bytes, &mut wrapper)?;

                wrapper.0
            };

            bar.finish();

            Ok(())
        }

        Commands::GetConfig => {
            let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;
            tokio::spawn(inbound_messages.for_each(|_| async {}));

            let response = device.get_config().await?;
            println!("{}", response);

            Ok(())
        }
    }
}
