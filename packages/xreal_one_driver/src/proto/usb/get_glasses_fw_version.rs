use crate::proto::usb::{Response, UsbTransaction};
use anyhow::Error;

// todo: there are other firmware version read commands (0x18 and 0x13)

pub struct GetGlassesFwVersionTransaction;

impl UsbTransaction for GetGlassesFwVersionTransaction {
    const COMMAND_ID: u8 = 0x26;
    type RequestArgs = ();
    type Response = GetGlassesFwVersionResponse;
}

#[derive(Debug)]
pub struct GetGlassesFwVersionResponse {
    pub version: String,
}

impl Response for GetGlassesFwVersionResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, Error> {
        let version = String::from_utf8_lossy(&buffer[1..]).to_string();

        Ok(Self { version })
    }
}
