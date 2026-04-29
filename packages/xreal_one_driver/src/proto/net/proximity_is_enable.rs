use crate::proto::net::enable_status_response::EnableStatusResponse;
use crate::proto::net::props::GetPropertyRequest;
use crate::proto::net::NetworkTransaction;

pub struct ProximityIsEnable;

impl NetworkTransaction<'static> for ProximityIsEnable {
    const MAGIC: [u8; 2] = [0x27, 0x18];
    type RequestArgs = GetPropertyRequest;
    type Response = EnableStatusResponse;
}
