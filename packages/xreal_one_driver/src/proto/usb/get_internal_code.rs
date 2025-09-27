use crate::proto::usb::{Empty, Response, UsbTransaction};
use anyhow::Error;

pub struct GetInternalCode;

impl UsbTransaction for GetInternalCode {
    const COMMAND_ID: u8 = 0xD4;
    type RequestArgs = Empty;
    type Response = GetInternalCodeResponse;
}

#[derive(Debug)]
pub struct GetInternalCodeResponse {
    pub value: u8,
}

impl Response for GetInternalCodeResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, Error> {
        assert_eq!(buffer.len(), 1);

        Ok(Self { value: buffer[0] })
    }
}
