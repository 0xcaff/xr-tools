use ahrs::Ahrs;
use futures::StreamExt;
use futures::pin_mut;
use nalgebra::{UnitQuaternion, Vector3};
use xreal_one_driver::proto::net::reports;
use xreal_one_driver::{ControlNetworkDevice, ReportType};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
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
            let gyro = Vector3::from([report.gx as f64, report.gy as f64, report.gz as f64]);
            let gyro_bias = Vector3::from(
                config
                    .imu
                    .gyro_bias_temp_data
                    .interpolate(report.temperature as _),
            );

            gyro - gyro_bias
        };

        let accel = {
            let accel = Vector3::from([report.ax as f64, report.ay as f64, report.az as f64]);
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
            "Pitch: {:.3} Roll: {:.3} Yaw: {:.3}",
            pitch.to_degrees(),
            roll.to_degrees(),
            -yaw.to_degrees()
        );
    }

    Ok(())
}
