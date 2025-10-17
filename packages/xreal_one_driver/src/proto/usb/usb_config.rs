use crate::proto::usb::{Empty, RequestArgs, Response, UsbTransaction};
use crate::UsbDevice;
use modular_bitfield::bitfield;
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
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, anyhow::Error> {
        Ok(Cow::Owned(self.config.into_bytes().to_vec()))
    }
}

pub struct GetUsbConfigAll;

impl UsbTransaction<'static> for GetUsbConfigAll {
    const COMMAND_ID: [u8; 2] = [0xD2, 0x00];
    type RequestArgs = Empty;
    type Response = GetUsbConfigAllResponse;
}

#[bitfield(bits = 32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbConfigList {
    // todo: validate meaning and make this an enum
    // 2 = disable
    // 1 = enable
    // 0 = do nothing
    pub ncm: modular_bitfield::specifiers::B2,
    pub ecm: modular_bitfield::specifiers::B2,
    pub uac: modular_bitfield::specifiers::B2,
    pub hid_ctrl: modular_bitfield::specifiers::B2,
    pub mtp: modular_bitfield::specifiers::B2,
    pub mass_storage: modular_bitfield::specifiers::B2,
    pub uvc0: modular_bitfield::specifiers::B2,
    pub uvc1: modular_bitfield::specifiers::B2,
    pub enable: modular_bitfield::specifiers::B2,
    #[skip]
    __: modular_bitfield::specifiers::B14,
}

#[derive(Debug)]
pub struct GetUsbConfigAllResponse {
    pub config: UsbConfigList,
}

impl Response for GetUsbConfigAllResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        assert_eq!(buffer.len(), 4);
        let bytes: [u8; 4] = buffer.try_into()?;
        let config = UsbConfigList::from_bytes(bytes);
        Ok(Self { config })
    }
}

impl UsbDevice {
    pub fn get_usb_config(&self) -> Result<UsbConfigList, anyhow::Error> {
        Ok(self.send_message::<GetUsbConfigAll>(Empty)?.config)
    }

    pub fn set_usb_config(&self, config: UsbConfigList) -> Result<(), anyhow::Error> {
        self.send_message::<SetUsbConfigAll>(SetUsbConfigAllRequest { config })?;

        Ok(())
    }
}
