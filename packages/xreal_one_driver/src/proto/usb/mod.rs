use std::borrow::Cow;

pub mod get_camera_status;
pub mod get_glasses_fw_version;
pub mod get_usb_config_all;
pub mod set_usb_config_all;

pub trait UsbTransaction {
    const COMMAND_ID: u8;

    type RequestArgs: RequestArgs;
    type Response: Response;
}

pub trait RequestArgs {
    fn as_bytes(&self) -> Result<Cow<[u8]>, anyhow::Error>;

    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, anyhow::Error> {
        let bytes = self.as_bytes()?;
        buffer[..bytes.len()].copy_from_slice(&bytes);

        Ok(bytes.len())
    }
}

pub trait Response: Sized {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error>;
}

impl RequestArgs for () {
    fn as_bytes(&self) -> Result<Cow<[u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(&[]))
    }
}

impl Response for () {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], 0);

        Ok(())
    }
}
