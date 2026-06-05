//! VITURE Luma interfaces.
//!
//! The Luma SDK path used by the demo opens V1 pose mode over HID and forwards
//! `0x0308` pose frames containing roll, pitch, yaw, and quaternion values.

use anyhow::{bail, Result};
use futures::Stream;
use hidapi::{DeviceInfo, HidApi};
use std::borrow::Cow;

mod endpoint;
mod imu;

pub use endpoint::UsbEndpoint;
pub use imu::{ImuFrequency, ImuMode, ImuPose, SetImuFrequency, SetImuMode};

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
        if bytes.len() > buffer.len() {
            bail!(
                "VITURE request payload is too large: {} bytes, max {}",
                bytes.len(),
                buffer.len()
            );
        }

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

impl Response for () {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        assert_eq!(buffer.len(), 0);

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VitureLumaModelKind {
    Luma,
}

impl VitureLumaModelKind {
    pub fn detect(product_id: u16) -> Option<Self> {
        match product_id {
            0x1131 => Some(Self::Luma),
            _ => None,
        }
    }
}

pub struct VitureLumaModel<'a> {
    pub control_device: &'a DeviceInfo,
    pub imu_device: &'a DeviceInfo,
}

impl VitureLumaModel<'_> {
    pub fn detect(api: &HidApi) -> Option<VitureLumaModel<'_>> {
        Self::detect_product_id(api, 0x1131)
    }

    pub fn detect_product_id(api: &HidApi, product_id: u16) -> Option<VitureLumaModel<'_>> {
        VitureLumaModelKind::detect(product_id)?;

        let mut control_devices = Vec::new();
        let mut imu_devices = Vec::new();

        for device in api
            .device_list()
            .filter(|device| device.vendor_id() == 0x35ca && device.product_id() == product_id)
        {
            let Some(serial_number) = device.serial_number() else {
                continue;
            };

            match device.interface_number() {
                0 => {
                    if let Some(control_device) = control_devices
                        .iter()
                        .copied()
                        .find(|device: &&DeviceInfo| device.serial_number() == Some(serial_number))
                    {
                        return Some(VitureLumaModel {
                            control_device,
                            imu_device: device,
                        });
                    }

                    imu_devices.push(device);
                }
                1 => {
                    if let Some(imu_device) = imu_devices
                        .iter()
                        .copied()
                        .find(|device: &&DeviceInfo| device.serial_number() == Some(serial_number))
                    {
                        return Some(VitureLumaModel {
                            control_device: device,
                            imu_device,
                        });
                    }

                    control_devices.push(device);
                }
                _ => continue,
            };
        }

        None
    }
}

pub struct UsbDevice {
    endpoint: UsbEndpoint,
}

impl UsbDevice {
    pub fn open(api: &HidApi, device: VitureLumaModel<'_>) -> Result<Self, anyhow::Error> {
        let endpoint =
            UsbEndpoint::open_device_info(api, device.control_device, device.imu_device)?;
        Ok(Self { endpoint })
    }

    pub fn start_imu(self) -> Result<impl Stream<Item = ImuPose> + Send + 'static, anyhow::Error> {
        self.endpoint.start_imu()
    }
}
