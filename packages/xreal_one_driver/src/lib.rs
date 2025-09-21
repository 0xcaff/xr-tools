use crate::proto::usb::get_usb_config_all::UsbConfigList;
use crate::proto::usb::set_usb_config_all::{SetUsbConfigAll, SetUsbConfigAllRequest};
use crate::proto::usb::UsbDevice;

pub mod proto;

#[test]
fn test() -> Result<(), anyhow::Error> {
    let api = hidapi::HidApi::new()?;
    let device = UsbDevice::open(&api)?;
    // device.send_message_raw(0xD3, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x01, 0x00])?;
    // let response = device.send_message_raw(0xD5, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x0])?;
    // println!("{:?}", response.data());
    let response = device.send_mesasge::<SetUsbConfigAll>(SetUsbConfigAllRequest {
        config: UsbConfigList::new()
            .with_uvc0(1)
            .with_uvc1(0)
            .with_mtp(1)
            .with_enable(1)
    })?;
    println!("{:?}", response);

    Ok(())
}
