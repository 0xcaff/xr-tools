use anyhow::{Context, Result};
use hidapi::HidApi;
use std::thread;
use std::time::{Duration, Instant};
use viture_driver::{VitureDevice, VitureLumaModel, VITURE_VENDOR_ID};

const RUN_DURATION: Duration = Duration::from_secs(5);

fn main() -> Result<()> {
    let api = HidApi::new().context("failed to initialize hidapi")?;
    #[cfg(target_os = "macos")]
    api.set_open_exclusive(false);

    println!("VITURE HID endpoints:");
    for device in api
        .device_list()
        .filter(|device| device.vendor_id() == VITURE_VENDOR_ID)
    {
        let product = device.product_string().unwrap_or("<unknown>");
        let serial = device.serial_number().unwrap_or("<none>");
        println!(
            "  path={} pid=0x{:04x} interface={} usage=0x{:04x}:0x{:04x} product={product} serial={serial}",
            device.path().to_string_lossy(),
            device.product_id(),
            device.interface_number(),
            device.usage_page(),
            device.usage()
        );
    }

    let model = api
        .device_list()
        .find_map(VitureLumaModel::detect)
        .context("no VITURE Luma HID device detected")?;

    println!(
        "Opening {} pid=0x{:04x}",
        model.market_name(),
        model.product_id()
    );

    let device = VitureDevice::open(&api, model)?;
    let start = Instant::now();
    let mut last_sample_count = 0;

    loop {
        let snapshot = device.latest_pose();
        if snapshot.sample_count != last_sample_count {
            last_sample_count = snapshot.sample_count;
            if let Some(pose) = snapshot.pose {
                println!(
                    "{:>8.3}s sample={} rpy=({:>8.3}, {:>8.3}, {:>8.3}) quat=({:>8.5}, {:>8.5}, {:>8.5}, {:>8.5}) t={}ns",
                    start.elapsed().as_secs_f32(),
                    snapshot.sample_count,
                    pose.roll,
                    pose.pitch,
                    pose.yaw,
                    pose.quaternion_w,
                    pose.quaternion_x,
                    pose.quaternion_y,
                    pose.quaternion_z,
                    pose.timestamp_ns
                );
            }
        }

        if start.elapsed() >= RUN_DURATION {
            break;
        }

        thread::sleep(Duration::from_millis(20));
    }

    Ok(())
}
