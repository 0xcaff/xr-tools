use crate::proto::net::props::EmptyMessageRequest;
use crate::proto::net::{NetworkTransaction, RawResponse};

pub struct SpaceScreenGetEisEnable;

impl NetworkTransaction<'static> for SpaceScreenGetEisEnable {
    const MAGIC: [u8; 2] = [0x28, 0x9b];
    type RequestArgs = EmptyMessageRequest;
    type Response = RawResponse;
}
