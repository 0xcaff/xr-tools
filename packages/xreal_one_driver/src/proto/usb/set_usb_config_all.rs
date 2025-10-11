use crate::proto::usb::get_usb_config_all::UsbConfigList;
use crate::proto::usb::{RequestArgs, UsbTransaction};
use anyhow::Error;
use std::borrow::Cow;

pub struct SetUsbConfigAll;

impl<'req> UsbTransaction<'req> for SetUsbConfigAll {
    const COMMAND_ID: [u8; 2] = [0xD3, 0x00];
    type RequestArgs = SetUsbConfigAllRequest;
    type Response = ();
}

pub struct SetUsbConfigAllRequest {
    pub config: UsbConfigList,
}

impl<'a> RequestArgs<'a> for SetUsbConfigAllRequest {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, Error> {
        Ok(Cow::Owned(self.config.into_bytes().to_vec()))
    }
}
