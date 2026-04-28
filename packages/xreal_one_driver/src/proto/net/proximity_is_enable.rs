use crate::proto::net::props::GetPropertyRequest;
use crate::proto::net::{NetworkTransaction, RawResponse};

pub struct ProximityIsEnable;

impl NetworkTransaction<'static> for ProximityIsEnable {
    const MAGIC: [u8; 2] = [0x27, 0x19];
    type RequestArgs = GetPropertyRequest;
    type Response = RawResponse;
}
