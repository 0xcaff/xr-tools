use crate::proto::net::enable_value::EnableValue;
use crate::proto::net::props::{EmptyPropertyResponse, SetNumericProperty, SetPropertyRequest};
use crate::proto::net::NetworkTransaction;

pub struct SpaceScreenSetEisEnable;

impl NetworkTransaction<'static> for SpaceScreenSetEisEnable {
    const MAGIC: [u8; 2] = [0x28, 0xa7];
    type RequestArgs = SetPropertyRequest<SetNumericProperty<EnableValue>>;
    type Response = EmptyPropertyResponse;
}
