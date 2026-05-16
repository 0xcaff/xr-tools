use crate::usb::open_normal_usb_device;
use xreal_one_driver::UsbConfigList;

pub async fn run() -> Result<(), anyhow::Error> {
    let api = hidapi::HidApi::new()?;
    let (device, _usb_runner) = open_normal_usb_device(&api)?;

    device
        .set_usb_config(UsbConfigList::new().with_uvc0(1).with_enable(1))
        .await?;

    Ok(())
}
