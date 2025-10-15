use crate::proto::net::props::{
    GetPropertyRequest, PropertyResponse, ReadNumericProperty, SetNumericProperty,
};
use crate::proto::net::NetworkTransaction;
use strum::FromRepr;

pub struct DpGetCurrentEdidDsp;

impl NetworkTransaction<'static> for DpGetCurrentEdidDsp {
    const MAGIC: [u8; 2] = [0x27, 0x5e];
    type RequestArgs = GetPropertyRequest;
    type Response = PropertyResponse<ReadNumericProperty<DisplayConfiguration>>;
}

#[derive(Copy, Clone, FromRepr)]
#[repr(u8)]
pub enum DisplayConfiguration {
    First = 0,
}

impl Into<u8> for DisplayConfiguration {
    fn into(self) -> u8 {
        self as u8
    }
}

impl From<u8> for DisplayConfiguration {
    fn from(value: u8) -> Self {
        Self::from_repr(value).unwrap()
    }
}
