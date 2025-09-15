use crate::proto::usb::{Response, UsbTransaction};
use modular_bitfield::bitfield;

pub struct GetUsbConfigAll;

impl UsbTransaction for GetUsbConfigAll {
    const COMMAND_ID: u8 = 0xD2;
    type RequestArgs = ();
    type Response = GetUsbConfigAllResponse;
}

#[bitfield(bits = 32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbConfigList {
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
        assert_eq!(buffer.len(), 5);
        let bytes: [u8; 4] = buffer[1..5].try_into()?;
        let config = UsbConfigList::from_bytes(bytes);
        Ok(Self { config })
    }
}
