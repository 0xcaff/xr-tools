use crate::proto::net::props::{GetPropertyRequest, PropertyResponse, ReadNumericProperty};
use crate::proto::net::NetworkTransaction;
use anyhow::format_err;
use strum::FromRepr;

pub struct DpGetCurrentEdidDsp;

impl NetworkTransaction<'static> for DpGetCurrentEdidDsp {
    const MAGIC: [u8; 2] = [0x27, 0x5e];
    type RequestArgs = GetPropertyRequest;
    type Response = PropertyResponse<ReadNumericProperty<DisplayConfiguration>>;
}

#[derive(Copy, Clone, FromRepr, Debug)]
#[repr(u8)]
pub enum DisplayConfiguration {
    _1920x1080_60Hz = 2,
    _1920x1080_90Hz = 3,
    _1920x1080_120Hz = 4,
    _3840x1080_60Hz = 5,
}

impl Into<u8> for DisplayConfiguration {
    fn into(self) -> u8 {
        self as u8
    }
}

impl From<u8> for DisplayConfiguration {
    fn from(value: u8) -> Self {
        Self::from_repr(value)
            .ok_or_else(|| format_err!("invalid display configuration: {}", value))
            .unwrap()
    }
}
