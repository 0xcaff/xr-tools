use crate::proto::usb::{Empty, Response, UsbTransaction};
use anyhow::Error;
use crate::proto::net::RawRequest;

pub struct GetGlassesMcuFwVersion;

impl UsbTransaction<'static> for GetGlassesMcuFwVersion {
    // firmware type 1
    const COMMAND_ID: [u8; 2] = [0x26, 0x00];
    type RequestArgs = Empty;
    type Response = GetGlassesFwVersionResponse;
}

pub struct GetGlassesDspFwVersion;

impl UsbTransaction<'static> for GetGlassesDspFwVersion {
    // firmware type 3
    const COMMAND_ID: [u8; 2] = [0x18, 0x00];
    type RequestArgs = Empty;
    type Response = GetGlassesFwVersionResponse;
}

pub struct GetGlassesPilotFw;

impl UsbTransaction<'static> for GetGlassesPilotFw {
    const COMMAND_ID: [u8; 2] = [0x13, 0x12];
    // firmware type 6

    type RequestArgs = RawRequest<'static>;
    type Response = GetGlassesFwVersionResponse;
}


#[derive(Debug)]
pub struct GetGlassesFwVersionResponse {
    pub version: String,
}

impl Response for GetGlassesFwVersionResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, Error> {
        let version = String::from_utf8_lossy(&buffer).to_string();

        Ok(Self { version })
    }
}
