use crate::proto::net::{NetworkTransaction, RawRequest, RawResponse};

pub struct ProximityIsEnable;

impl NetworkTransaction<'static> for ProximityIsEnable {
    const MAGIC: [u8; 2] = [0x27, 0x19];
    type RequestArgs = RawRequest<'static>;
    type Response = RawResponse;
}
