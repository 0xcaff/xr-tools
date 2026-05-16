use futures::Future;
use std::borrow::Cow;

pub mod dsp_update;
mod endpoint;
pub mod firmware_header;
pub mod get_camera_status;
pub mod get_glasses_fw_version;
pub mod get_internal_code;
pub mod mcu_update;
pub mod pilot_update;
pub mod recovery;
pub mod recovery_mcu_update;
pub mod usb_config;

pub use endpoint::{UsbEndpoint, UsbInboundMessage};
pub use usb_config::UsbConfigList;

pub trait UsbTransaction<'req> {
    const COMMAND_ID: [u8; 2];
    const UNKONWN_VALUES: [u8; 5] = [0u8; 5];

    type RequestArgs: RequestArgs<'req>;
    type Response: Response;
}

pub trait RequestArgs<'a> {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, anyhow::Error>;

    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, anyhow::Error> {
        let bytes = self.as_bytes()?;
        buffer[..bytes.len()].copy_from_slice(&bytes);

        Ok(bytes.len())
    }
}

pub trait Response: Sized {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error>;
}

pub struct RawResponse(pub Vec<u8>);

impl Response for RawResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        Ok(Self(buffer.to_vec()))
    }
}

pub struct Empty;

impl RequestArgs<'static> for Empty {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(&[]))
    }
}

pub trait ConstRequestArgs {
    const VALUE: &'static [u8];
}

// impl <T: ConstRequestArgs> RequestArgs for T {
//     fn as_bytes(&self) -> Result<Cow<[u8]>, anyhow::Error> {
//         Ok(Cow::Borrowed(T::VALUE))
//     }
// }

impl Response for () {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        assert_eq!(buffer.len(), 0);

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XrealOneModelKind {
    XrealOne,
    XrealOnePro,
}

pub struct XrealOneModel<'a> {
    device: &'a hidapi::DeviceInfo,
    kind: XrealOneModelKind,
}

impl XrealOneModel<'_> {
    pub fn detect<'a>(device: &'a hidapi::DeviceInfo) -> Option<XrealOneModel<'a>> {
        if device.vendor_id() != 0x3318 {
            return None;
        }

        let kind = match device.product_id() {
            0x0436 => XrealOneModelKind::XrealOnePro,
            0x0438 => XrealOneModelKind::XrealOne,
            _ => return None,
        };

        Some(XrealOneModel { kind, device })
    }

    pub fn kind(&self) -> XrealOneModelKind {
        self.kind
    }

    pub fn vendor_id(&self) -> u16 {
        self.device.vendor_id()
    }

    pub fn product_id(&self) -> u16 {
        self.device.product_id()
    }

    pub fn product_string(&self) -> Option<&str> {
        self.device.product_string()
    }

    pub fn serial_number(&self) -> Option<&str> {
        self.device.serial_number()
    }
}

pub struct XrealOneRecoveryModel<'a> {
    device: &'a hidapi::DeviceInfo,
    kind: XrealOneModelKind,
}

impl XrealOneRecoveryModel<'_> {
    pub fn detect<'a>(device: &'a hidapi::DeviceInfo) -> Option<XrealOneRecoveryModel<'a>> {
        if device.vendor_id() != 0x3318 {
            return None;
        }

        let kind = match device.product_id() {
            0x0435 => XrealOneModelKind::XrealOnePro,
            0x0437 => XrealOneModelKind::XrealOne,
            _ => return None,
        };

        Some(XrealOneRecoveryModel { kind, device })
    }

    pub fn kind(&self) -> XrealOneModelKind {
        self.kind
    }

    pub fn vendor_id(&self) -> u16 {
        self.device.vendor_id()
    }

    pub fn product_id(&self) -> u16 {
        self.device.product_id()
    }

    pub fn product_string(&self) -> Option<&str> {
        self.device.product_string()
    }

    pub fn serial_number(&self) -> Option<&str> {
        self.device.serial_number()
    }
}

pub struct UsbDevice {
    endpoint: UsbEndpoint,
}

pub struct RecoveryUsbDevice {
    endpoint: UsbEndpoint,
}

impl UsbDevice {
    pub fn open(
        api: &hidapi::HidApi,
        device: XrealOneModel<'_>,
    ) -> Result<
        (
            Self,
            impl Future<Output = Result<(), anyhow::Error>> + Send + 'static,
        ),
        anyhow::Error,
    > {
        let (endpoint, runner) = UsbEndpoint::open_device_info(api, device.device)?;
        Ok((Self { endpoint }, runner))
    }
}

impl RecoveryUsbDevice {
    pub fn open(
        api: &hidapi::HidApi,
        device: XrealOneRecoveryModel<'_>,
    ) -> Result<
        (
            Self,
            impl Future<Output = Result<(), anyhow::Error>> + Send + 'static,
        ),
        anyhow::Error,
    > {
        let (endpoint, runner) = UsbEndpoint::open_device_info(api, device.device)?;
        Ok((Self { endpoint }, runner))
    }
}
