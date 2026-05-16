use crate::usb::open_normal_usb_device;

#[derive(Debug, clap::Args)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<(), anyhow::Error> {
    let api = hidapi::HidApi::new()?;
    let (device, _usb_runner) = open_normal_usb_device(&api)?;

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
