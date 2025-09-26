use anyhow::Error;
use crate::proto::net::{protos, NetworkTransaction, Response};

pub struct GlassesGetDspVersion;

impl NetworkTransaction for GlassesGetDspVersion {
    const MAGIC: [u8; 2] = [0x27, 0x2d];
    type RequestArgs = protos::get_config::Request;
    type Response = protos::get_config::Response;
}

#[derive(Debug)]
pub struct RawResponse(pub Vec<u8>);

impl Response for RawResponse {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(buffer))
    }
}