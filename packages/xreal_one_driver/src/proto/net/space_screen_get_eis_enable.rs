use crate::proto::net::enable_status_response::EnableStatusResponse;
use crate::proto::net::props::GetPropertyRequest;
use crate::proto::net::NetworkTransaction;

pub struct SpaceScreenGetEisEnable;

impl NetworkTransaction<'static> for SpaceScreenGetEisEnable {
    const MAGIC: [u8; 2] = [0x28, 0xa6];
    type RequestArgs = GetPropertyRequest;
    type Response = EnableStatusResponse;
}
