use ahrs::Ahrs;
use clap::{Parser, Subcommand};
use futures::{pin_mut, StreamExt};
use indicatif::ProgressStyle;
use nalgebra::{UnitQuaternion, Vector3};
use std::path::PathBuf;
use std::time::Duration;
use xreal_one_driver::proto::net::reports;
use xreal_one_driver::proto::usb::mcu_update::{McuUpdate, McuUpdateProgressReporter};
use xreal_one_driver::proto::usb::pilot_update::PilotUpdateProgressReporter;
use xreal_one_driver::{ControlNetworkDevice, ReportType, UsbConfigList, UsbDevice};

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
    Info,
    EnableCameras,
    Imu,
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

            let response = device.get_config_raw().await?;
            println!("{}", response);

            Ok(())
        }
        Commands::Info => {
            let api = hidapi::HidApi::new()?;
            let device = UsbDevice::open(&api)?;

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
        Commands::EnableCameras => {
            let api = hidapi::HidApi::new()?;
            let device = UsbDevice::open(&api)?;

            device.set_usb_config(UsbConfigList::new().with_uvc0(1).with_enable(1))?;

            Ok(())
        }
        Commands::Imu => {
            let reports = reports::listen().await?;
            pin_mut!(reports);

            let (mut control, messages) = ControlNetworkDevice::new().await?;
            tokio::spawn(messages.for_each(|_| async {}));

            let config = control.get_config().await?;

            let mut ahrs = ahrs::Madgwick::new(1.0f64 / 1000.0, 0.1);

            while let Some(report) = reports.next().await.transpose()? {
                let reports::InboundMessageType::Report(report) = report else {
                    continue;
                };

                if report.report_type != ReportType::IMU {
                    continue;
                }

                println!("{:#?}", report);

                let gyro = {
                    let gyro =
                        Vector3::from([report.gx as f64, report.gy as f64, report.gz as f64]);
                    let gyro_bias = Vector3::from(
                        config
                            .imu
                            .gyro_bias_temp_data
                            .interpolate(report.temperature as _),
                    );

                    gyro - gyro_bias
                };

                let accel = {
                    let accel =
                        Vector3::from([report.ax as f64, report.ay as f64, report.az as f64]);
                    let accel_bias = Vector3::from(config.imu.accel_bias);

                    accel - accel_bias
                };

                let next = ahrs.update_imu(&gyro, &accel).unwrap();

                // IMU Space -> Display Space
                let next = config.display.right.transform.rotation * next;

                // Display Space -> Glasses Space
                let next = next * UnitQuaternion::from_euler_angles(90.0f64.to_radians(), 0.0, 0.0);

                let (pitch, roll, yaw) = next.euler_angles();

                println!(
                    "{:.3} {:.3} {:.3}",
                    pitch.to_degrees(),
                    roll.to_degrees(),
                    yaw.to_degrees()
                );
            }

            Ok(())
        }
    }
}
