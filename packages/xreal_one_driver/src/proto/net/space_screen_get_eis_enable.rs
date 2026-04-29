use crate::proto::net::enable_value::EnableValue;
use crate::proto::net::props::EmptyMessageRequest;
use crate::proto::net::props::{PropertyResponse, ReadNumericProperty};
use crate::proto::net::NetworkTransaction;

pub struct SpaceScreenGetEisEnable;

impl NetworkTransaction<'static> for SpaceScreenGetEisEnable {
    const MAGIC: [u8; 2] = [0x28, 0xa6];
    type RequestArgs = EmptyMessageRequest;
    type Response = PropertyResponse<ReadNumericProperty<EnableValue>>;
}
