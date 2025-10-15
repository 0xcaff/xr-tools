use crate::proto::net::RawRequest;
use crate::proto::usb::{Empty, Response, UsbTransaction};
use crate::UsbDevice;

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
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        let version = String::from_utf8_lossy(&buffer).to_string();

        Ok(Self { version })
    }
}

impl UsbDevice {
    pub fn get_mcu_fw_version(&self) -> Result<String, anyhow::Error> {
        Ok(self.send_message::<GetGlassesMcuFwVersion>(Empty)?.version)
    }

    pub fn get_dsp_fw_version(&self) -> Result<String, anyhow::Error> {
        Ok(self.send_message::<GetGlassesDspFwVersion>(Empty)?.version)
    }

    // todo: figure out wtf goes here
    // pub fn get_pilot_fw_version(&self) -> Result<String, anyhow::Error> {
    // Ok(self.send_message::<GetGlassesPilotFw>(RawRequest(&[]))?.version)
    // }
}
