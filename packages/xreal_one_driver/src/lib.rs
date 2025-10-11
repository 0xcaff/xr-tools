use crate::proto::net::RawRequest;
use crate::proto::usb::get_glasses_fw_version::GetGlassesPilotFw;
use crate::proto::usb::UsbDevice;

pub mod proto;

#[test]
fn test() -> Result<(), anyhow::Error> {
    let api = hidapi::HidApi::new()?;
    let device = UsbDevice::open(&api)?;
    // device.send_message_raw(0xD3, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x01, 0x00])?;
    // let response = device.send_message_raw(0xD5, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x0])?;
    // println!("{:?}", response.data());
    let response = device.send_mesasge::<GetGlassesPilotFw>(RawRequest(&[0x02]))?;
    println!("{:?}", response);

    Ok(())
}
