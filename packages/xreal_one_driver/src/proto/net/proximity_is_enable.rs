use crate::proto::net::enable_value::EnableValue;
use crate::proto::net::props::{GetPropertyRequest, PropertyResponse, ReadNumericProperty};
use crate::proto::net::NetworkTransaction;

pub struct ProximityIsEnable;

impl NetworkTransaction<'static> for ProximityIsEnable {
    const MAGIC: [u8; 2] = [0x27, 0x19];
    type RequestArgs = GetPropertyRequest;
    type Response = PropertyResponse<ReadNumericProperty<EnableValue>>;
}
