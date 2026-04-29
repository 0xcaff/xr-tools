use crate::proto::net::props::{EmptyMessageRequest, PropertyResponse, ReadNumericProperty};
use crate::proto::net::NetworkTransaction;

pub struct SpaceScreenGetEisEnable;

impl NetworkTransaction<'static> for SpaceScreenGetEisEnable {
    const MAGIC: [u8; 2] = [0x28, 0xa6];
    type RequestArgs = EmptyMessageRequest;
    type Response = PropertyResponse<ReadNumericProperty<u8>>;
}
