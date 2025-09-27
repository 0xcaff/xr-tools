use crate::proto::net::{NetworkTransaction, RawRequest, RawResponse};

pub struct GlassesDp;

impl NetworkTransaction for GlassesDp {
    const MAGIC: [u8; 2] = [0x27, 0x10];
    type RequestArgs = RawRequest;
    type Response = RawResponse;
}

