use crate::proto::net::enable_value::EnableValue;
use crate::proto::net::props::{EmptyPropertyResponse, SetNumericProperty, SetPropertyRequest};
use crate::proto::net::NetworkTransaction;

pub struct ProximitySetEnable;

impl NetworkTransaction<'static> for ProximitySetEnable {
    const MAGIC: [u8; 2] = [0x27, 0x18];
    type RequestArgs = SetPropertyRequest<SetNumericProperty<EnableValue>>;
    type Response = EmptyPropertyResponse;
}
