use clap::{Parser, Subcommand, ValueEnum};
use futures::StreamExt;
use indicatif::ProgressStyle;
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::usb::mcu_update::{McuUpdate, McuUpdateProgressReporter};
use xreal_one_driver::proto::usb::pilot_update::PilotUpdateProgressReporter;
use xreal_one_driver::{ControlNetworkDevice, UsbConfigList, UsbDevice, XrealOneModel};

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
#[command(author, version, about, long_about = None)]
enum Commands {
    GetConfig,
    Info,
    Enable {
        target: Target,
    },
    Disable {
        target: Target,
    },
    EnableCamera,
    Update {
        mcu_path: PathBuf,
        pilot_path: PathBuf,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Target {
    Proximity,
    #[value(alias = "stab")]
    Stabilizer,
}

impl Target {
    fn name(self) -> &'static str {
        match self {
            Self::Proximity => "proximity",
            Self::Stabilizer => "stabilizer",
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    match args.command {
        Commands::GetConfig => {
            let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;
            tokio::spawn(inbound_messages.for_each(|_| async {}));

            let response = device.get_config_raw().await?;
            println!("{}", response);

            for target in [Target::Proximity, Target::Stabilizer] {
                let enabled = get_target_enabled(&mut device, target).await?;
                let status = if enabled { "enabled" } else { "disabled" };
                println!("{}: {}", target.name(), status);
            }

            Ok(())
        }
        Commands::Enable { target } => {
            let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;
            tokio::spawn(inbound_messages.for_each(|_| async {}));

            set_target_enabled(&mut device, target, true).await?;
            println!("enabled {}", target.name());

            Ok(())
        }
        Commands::Disable { target } => {
            let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;
            tokio::spawn(inbound_messages.for_each(|_| async {}));

            set_target_enabled(&mut device, target, false).await?;
            println!("disabled {}", target.name());

            Ok(())
        }
        Commands::Info => {
            let api = hidapi::HidApi::new()?;

            let device = api
                .device_list()
                .find_map(|it| Some(XrealOneModel::detect(it)?))
                .ok_or_else(|| anyhow::anyhow!("no device found"))?;
            let device = UsbDevice::open(&api, device)?;

            let dsp_fw_version = device.get_dsp_fw_version()?;
            println!("dsp_fw_version: {}", dsp_fw_version);

            let mcu_fw_version = device.get_mcu_fw_version()?;
            println!("mcu_fw_version: {}", mcu_fw_version);

            let camera_plugged = device.get_camera_plugged()?;
            println!("camera plugged: {:?}", camera_plugged);

            let usb_config = device.get_usb_config()?;
            println!("usb config: {:#?}", usb_config);

            Ok(())
        }
        Commands::EnableCamera => {
            let api = hidapi::HidApi::new()?;
            let device = api
                .device_list()
                .find_map(|it| Some(XrealOneModel::detect(it)?))
                .ok_or_else(|| anyhow::anyhow!("no device found"))?;
            let device = UsbDevice::open(&api, device)?;

            device.set_usb_config(UsbConfigList::new().with_uvc0(1).with_enable(1))?;

            Ok(())
        }
        Commands::Update {
            mcu_path,
            pilot_path,
        } => {
            let mcu_bytes = std::fs::read(mcu_path)?;
            let pilot_bytes = std::fs::read(pilot_path)?;

            let mcu_update = McuUpdate::parse(&mcu_bytes)?;

            let api = hidapi::HidApi::new()?;
            let device = api
                .device_list()
                .find_map(|it| Some(XrealOneModel::detect(it)?))
                .ok_or_else(|| anyhow::anyhow!("no device found"))?;
            let device = UsbDevice::open(&api, device)?;

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
    }
}

async fn set_target_enabled(
    device: &mut ControlNetworkDevice,
    target: Target,
    enabled: bool,
) -> Result<(), anyhow::Error> {
    match target {
        Target::Proximity => device.set_proximity_enable(enabled).await,
        Target::Stabilizer => device.set_space_screen_eis_enable(enabled).await,
    }
}

async fn get_target_enabled(
    device: &mut ControlNetworkDevice,
    target: Target,
) -> Result<bool, anyhow::Error> {
    match target {
        Target::Proximity => device.get_proximity_enable().await,
        Target::Stabilizer => device.get_space_screen_eis_enable().await,
    }
}
