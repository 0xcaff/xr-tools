use std::borrow::Cow;
use anyhow::Error;
use crate::proto::usb::{RequestArgs, Response, UsbTransaction};

// todo: there are other firmware version read commands (0x18 and 0x13)

pub struct GetGlassesFwVersionTransaction;

impl UsbTransaction for GetGlassesFwVersionTransaction {
    const COMMAND_ID: u8 = 0x26;
    type RequestArgs = GetGlassesFwVersionRequestArgs;
    type Response = GetGlassesFwVersionResponse;
}

pub struct GetGlassesFwVersionRequestArgs;

impl RequestArgs for GetGlassesFwVersionRequestArgs {
    fn as_bytes(&self) -> Result<Cow<[u8]>, Error> {
        Ok(Cow::Borrowed(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]))
    }
}

#[derive(Debug)]
pub struct GetGlassesFwVersionResponse {
    pub version: String,
}

impl Response for GetGlassesFwVersionResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, Error> {
        let version = String::from_utf8_lossy(&buffer[7..]).to_string();

        Ok(Self { version })
    }
}