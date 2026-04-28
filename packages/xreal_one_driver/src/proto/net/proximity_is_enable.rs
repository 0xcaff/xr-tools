use crate::proto::net::enable_value::EnableValue;
use crate::proto::net::props::{PropertyResponse, ReadNumericProperty};
use crate::proto::net::{NetworkTransaction, RawRequest};

pub struct ProximityIsEnable;

impl NetworkTransaction<'static> for ProximityIsEnable {
    const MAGIC: [u8; 2] = [0x27, 0x19];
    type RequestArgs = RawRequest<'static>;
    type Response = PropertyResponse<ReadNumericProperty<EnableValue>>;
}
