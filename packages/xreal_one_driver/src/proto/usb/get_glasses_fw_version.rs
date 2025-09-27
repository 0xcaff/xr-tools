use crate::proto::usb::{Empty, Response, UsbTransaction};
use anyhow::Error;
use crate::proto::net::RawRequest;

pub struct GetGlassesMcuFwVersion;

impl UsbTransaction for GetGlassesMcuFwVersion {
    // firmware type 1
    const COMMAND_ID: u8 = 0x26;
    type RequestArgs = Empty;
    type Response = GetGlassesFwVersionResponse;
}

pub struct GetGlassesDspFwVersion;

impl UsbTransaction for GetGlassesDspFwVersion {
    // firmware type 3
    const COMMAND_ID: u8 = 0x18;
    type RequestArgs = Empty;
    type Response = GetGlassesFwVersionResponse;
}

pub struct GetGlassesPilotFw;

impl UsbTransaction for GetGlassesPilotFw {
    // firmware type 6
    const COMMAND_ID: u8 = 0x13;
    const UNKONWN_VALUES: [u8; 6] = [0x12, 0x00, 0x00, 0x00, 0x00, 0x00];

    type RequestArgs = RawRequest;
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
