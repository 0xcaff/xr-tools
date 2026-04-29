use crate::proto::net::enable_value::EnableValue;
use crate::proto::net::props::EmptyMessageRequest;
use crate::proto::net::props::{PropertyResponse, ReadNumericProperty};
use crate::proto::net::NetworkTransaction;

pub struct ProximityIsEnable;

impl NetworkTransaction<'static> for ProximityIsEnable {
    const MAGIC: [u8; 2] = [0x27, 0x18];
    type RequestArgs = EmptyMessageRequest;
    type Response = PropertyResponse<ReadNumericProperty<EnableValue>>;
}
