use std::borrow::Cow;
use anyhow::{anyhow, Error};
use crate::proto::usb::{RequestArgs, Response, UsbTransaction};

pub struct GetCameraStatusTransaction;

impl UsbTransaction for GetCameraStatusTransaction {
    const COMMAND_ID: u8 = 0xD5;
    type RequestArgs = GetCameraStatusRequest;
    type Response = GetCameraStatusResponse;
}

pub struct GetCameraStatusRequest;

impl RequestArgs for GetCameraStatusRequest {
    fn as_bytes(&self) -> Result<Cow<[u8]>, Error> {
        Ok(Cow::Borrowed(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]))
    }
}

#[derive(Debug)]
pub struct GetCameraStatusResponse {
    pub plugged_in: bool,
}

impl Response for GetCameraStatusResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, Error> {
        let plugged_in = match buffer[7] {
            0x00 => true,
            0x01 => false,
            value => return Err(anyhow!("invalid camera status, {}", value)),
        };

        Ok(Self { plugged_in })
    }
}