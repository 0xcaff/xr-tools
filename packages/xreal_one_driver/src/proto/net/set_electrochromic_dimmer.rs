use crate::proto::net::props::{EmptyPropertyResponse, SetNumericProperty, SetPropertyRequest};
pub use crate::proto::net::NetworkTransaction;

pub struct SetElectrochromicDimmer;

impl NetworkTransaction<'static> for SetElectrochromicDimmer {
    const MAGIC: [u8; 2] = [0x27, 0x27];
    type RequestArgs = SetPropertyRequest<SetNumericProperty<ElectricDimmerLevels>>;
    type Response = EmptyPropertyResponse;
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ElectricDimmerLevels {
    First = 0,
    Second = 1,
    Third = 2,
}

impl Into<u8> for ElectricDimmerLevels {
    fn into(self) -> u8 {
        self as u8
    }
}
