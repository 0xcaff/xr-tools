use clap::{Parser, Subcommand};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};
use xreal_one_driver::proto::usb::mcu_update::{McuUpdate, McuUpdateProgressReporter};
use xreal_one_driver::proto::usb::pilot_update::PilotUpdateProgressReporter;
use xreal_one_driver::{
    ControlNetworkDevice, RecoveryUsbDevice, UsbConfigList, UsbDevice, XrealOneModel,
    XrealOneRecoveryModel,
};

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
    EnableCamera,
    Update {
        mcu_path: PathBuf,
        pilot_path: PathBuf,
    },
    RecoverMcu {
        mcu_path: PathBuf,
    },
}

type UsbRunnerHandle = JoinHandle<Result<(), anyhow::Error>>;

fn describe_model(model: &XrealOneModel<'_>) -> String {
    format!(
        "{:?} {:04x}:{:04x} {}",
        model.kind(),
        model.vendor_id(),
        model.product_id(),
        model.product_string().unwrap_or("unknown product")
    )
}

fn describe_recovery_model(model: &XrealOneRecoveryModel<'_>) -> String {
    format!(
        "{:?} recovery {:04x}:{:04x} {}",
        model.kind(),
        model.vendor_id(),
        model.product_id(),
        model.product_string().unwrap_or("unknown product")
    )
}

fn try_open_normal_usb_device(
    api: &hidapi::HidApi,
) -> Result<Option<(UsbDevice, UsbRunnerHandle)>, anyhow::Error> {
    let mut matches = api
        .device_list()
        .filter_map(XrealOneModel::detect)
        .collect::<Vec<_>>();

    if matches.len() > 1 {
        let descriptions = matches
            .iter()
            .map(describe_model)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "multiple normal XREAL HID devices found; disconnect extras first: {}",
            descriptions
        );
    }

    let Some(model) = matches.pop() else {
        return Ok(None);
    };

    let (device, usb_runner) = UsbDevice::open(api, model)?;
    Ok(Some((device, tokio::spawn(usb_runner))))
}

fn try_open_recovery_usb_device(
    api: &hidapi::HidApi,
) -> Result<Option<(RecoveryUsbDevice, UsbRunnerHandle)>, anyhow::Error> {
    let mut matches = api
        .device_list()
        .filter_map(XrealOneRecoveryModel::detect)
        .collect::<Vec<_>>();

    if matches.len() > 1 {
        let descriptions = matches
            .iter()
            .map(describe_recovery_model)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "multiple recovery XREAL HID devices found; disconnect extras first: {}",
            descriptions
        );
    }

    let Some(model) = matches.pop() else {
        return Ok(None);
    };

    let (device, usb_runner) = RecoveryUsbDevice::open(api, model)?;
    Ok(Some((device, tokio::spawn(usb_runner))))
}

async fn wait_for_recovery_usb_device(
    api: &mut hidapi::HidApi,
    timeout: Duration,
) -> Result<(RecoveryUsbDevice, UsbRunnerHandle), anyhow::Error> {
    let deadline = Instant::now() + timeout;

    loop {
        api.refresh_devices()?;
        if let Some(device) = try_open_recovery_usb_device(api)? {
            return Ok(device);
        }

        let now = Instant::now();
        if now >= deadline {
            anyhow::bail!(
                "timed out after {}s waiting for recovery XREAL HID device",
                timeout.as_secs()
            );
        }

        sleep(std::cmp::min(
            Duration::from_millis(250),
            deadline.saturating_duration_since(now),
        ))
        .await;
    }
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

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    match args.command {
        Commands::GetConfig => {
            let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;
            tokio::spawn(inbound_messages.for_each(|_| async {}));

            let response = device.get_config_raw().await?;
            println!("{}", response);

            Ok(())
        }
        Commands::Info => {
            let api = hidapi::HidApi::new()?;

            let device = api
                .device_list()
                .find_map(|it| Some(XrealOneModel::detect(it)?))
                .ok_or_else(|| anyhow::anyhow!("no device found"))?;
            let (device, usb_runner) = UsbDevice::open(&api, device)?;
            tokio::spawn(usb_runner);

            let dsp_fw_version = device.get_dsp_fw_version().await?;
            println!("dsp_fw_version: {}", dsp_fw_version);

            let mcu_fw_version = device.get_mcu_fw_version().await?;
            println!("mcu_fw_version: {}", mcu_fw_version);

            let camera_plugged = device.get_camera_plugged().await?;
            println!("camera plugged: {:?}", camera_plugged);

            let usb_config = device.get_usb_config().await?;
            println!("usb config: {:#?}", usb_config);

            Ok(())
        }
        Commands::EnableCamera => {
            let api = hidapi::HidApi::new()?;
            let device = api
                .device_list()
                .find_map(|it| Some(XrealOneModel::detect(it)?))
                .ok_or_else(|| anyhow::anyhow!("no device found"))?;
            let (device, usb_runner) = UsbDevice::open(&api, device)?;
            tokio::spawn(usb_runner);

            device
                .set_usb_config(UsbConfigList::new().with_uvc0(1).with_enable(1))
                .await?;

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
            let (device, usb_runner) = UsbDevice::open(&api, device)?;
            tokio::spawn(usb_runner);

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

                device
                    .update_mcu_with_progress(mcu_update, &mut wrapper)
                    .await?;

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

                device
                    .update_pilot_with_progress(&pilot_bytes, &mut wrapper)
                    .await?;

                wrapper.0
            };

            bar.finish();

            Ok(())
        }
        Commands::RecoverMcu { mcu_path } => {
            let mcu_bytes = std::fs::read(mcu_path)?;

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
            let device_style =
                ProgressStyle::with_template("{percent}% {bar} ({elapsed_precise}) {msg}")?;
            let mut wrapper = RecoveryProgressWrapper::new(bar, device_style);
            recovery_device
                .update_recovery_mcu_with_progress(&mcu_bytes, &mut wrapper)
                .await?;

            let bar = wrapper.into_inner();
            bar.finish_with_message("mcu recovery update complete");

            Ok(())
        }
    }
}
