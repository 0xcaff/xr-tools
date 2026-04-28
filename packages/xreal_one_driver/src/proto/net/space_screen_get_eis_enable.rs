use crate::proto::net::enable_value::EnableValue;
use crate::proto::net::props::{GetPropertyRequest, PropertyResponse, ReadNumericProperty};
use crate::proto::net::NetworkTransaction;

pub struct SpaceScreenGetEisEnable;

impl NetworkTransaction<'static> for SpaceScreenGetEisEnable {
    const MAGIC: [u8; 2] = [0x28, 0x9b];
    type RequestArgs = GetPropertyRequest;
    type Response = PropertyResponse<ReadNumericProperty<EnableValue>>;
}
