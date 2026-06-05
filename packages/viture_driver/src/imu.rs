use crate::{RequestArgs, UsbTransaction};
use anyhow::{bail, Result};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default)]
pub struct ImuPose {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub quaternion_w: f32,
    pub quaternion_x: f32,
    pub quaternion_y: f32,
    pub quaternion_z: f32,
    pub timestamp_ms: u32,
}

impl ImuPose {
    pub(crate) fn deserialize_from(
        timestamp_ms: u32,
        buffer: &[u8],
    ) -> Result<Self, anyhow::Error> {
        if buffer.len() < 0x24 {
            bail!(
                "failed to read VITURE IMU pose, only read {} bytes, expected at least 36",
                buffer.len()
            );
        }

        Ok(Self {
            roll: f32::from_bits(u32::from_be_bytes(buffer[0x00..0x04].try_into()?)),
            pitch: f32::from_bits(u32::from_be_bytes(buffer[0x04..0x08].try_into()?)),
            yaw: f32::from_bits(u32::from_be_bytes(buffer[0x08..0x0c].try_into()?)),
            quaternion_w: f32::from_bits(u32::from_be_bytes(buffer[0x14..0x18].try_into()?)),
            quaternion_x: f32::from_bits(u32::from_be_bytes(buffer[0x18..0x1c].try_into()?)),
            quaternion_y: f32::from_bits(u32::from_be_bytes(buffer[0x1c..0x20].try_into()?)),
            quaternion_z: f32::from_bits(u32::from_be_bytes(buffer[0x20..0x24].try_into()?)),
            timestamp_ms,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImuMode {
    Off,
    Pose,
}

impl ImuMode {
    fn wire_value(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Pose => 1,
        }
    }
}

impl RequestArgs<'static> for ImuMode {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>> {
        Ok(Cow::Owned(vec![self.wire_value()]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImuFrequency {
    High,
}

impl ImuFrequency {
    fn wire_value(self) -> u8 {
        match self {
            Self::High => 4,
        }
    }
}

impl RequestArgs<'static> for ImuFrequency {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>> {
        Ok(Cow::Owned(vec![self.wire_value()]))
    }
}

pub struct SetImuMode;

impl UsbTransaction<'static> for SetImuMode {
    const COMMAND_ID: [u8; 2] = [0x15, 0x00];

    type RequestArgs = ImuMode;
    type Response = ();
}

pub struct SetImuFrequency;

impl UsbTransaction<'static> for SetImuFrequency {
    const COMMAND_ID: [u8; 2] = [0x18, 0x00];

    type RequestArgs = ImuFrequency;
    type Response = ();
}
