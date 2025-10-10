use crate::proto::usb::{Empty, UsbTransaction};

pub struct GetInternalCode;

impl UsbTransaction<'static> for GetInternalCode {
    const COMMAND_ID: [u8; 2] = [0xD4, 0x00];
    type RequestArgs = Empty;
    type Response = ();
}
