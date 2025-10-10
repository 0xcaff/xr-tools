use crate::proto::usb::{Empty, Response, UsbTransaction};
use anyhow::{anyhow, Error};

pub struct GetCameraStatus;

impl UsbTransaction<'static> for GetCameraStatus {
    const COMMAND_ID: [u8; 2] = [0xD5, 0x00];
    type RequestArgs = Empty;
    type Response = GetCameraStatusResponse;
}

#[derive(Debug)]
pub struct GetCameraStatusResponse {
    pub plugged_in: bool,
}

impl Response for GetCameraStatusResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, Error> {
        assert_eq!(buffer.len(), 1);
        let plugged_in = match buffer[0] {
            0x00 => true,
            0x01 => false,
            value => return Err(anyhow!("invalid camera status, {}", value)),
        };

        Ok(Self { plugged_in })
    }
}
