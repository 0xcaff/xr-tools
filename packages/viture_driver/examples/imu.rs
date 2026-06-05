use anyhow::{Context, Result};
use futures::StreamExt;
use hidapi::HidApi;
use tokio::time::Instant;
use viture_driver::{UsbDevice, VitureLumaModel};

#[tokio::main]
async fn main() -> Result<()> {
    let api = HidApi::new().context("failed to initialize hidapi")?;

    let model = VitureLumaModel::detect(&api).context("no VITURE Luma HID device detected")?;

    println!(
        "Opening VITURE Luma pid=0x{:04x}",
        model.control_device.product_id()
    );

    let device = UsbDevice::open(&api, model)?;
    let mut reports = device.start_imu()?;
    let start = Instant::now();
    let mut sample_count = 0u64;

    while let Some(pose) = reports.next().await {
        sample_count += 1;
        println!(
            "{:>8.3}s sample={} rpy=({:>8.3}, {:>8.3}, {:>8.3}) quat=({:>8.5}, {:>8.5}, {:>8.5}, {:>8.5}) t={}ms",
            start.elapsed().as_secs_f32(),
            sample_count,
            pose.roll,
            pose.pitch,
            pose.yaw,
            pose.quaternion_w,
            pose.quaternion_x,
            pose.quaternion_y,
            pose.quaternion_z,
            pose.timestamp_ms
        );
    }

    Ok(())
}
