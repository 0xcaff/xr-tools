use crate::proto::net::{protos, NetworkTransaction};

pub struct GlassesGetDspVersion;

impl NetworkTransaction for GlassesGetDspVersion {
    const MAGIC: [u8; 2] = [0x27, 0x2d];
    type RequestArgs = protos::get_config::Request;
    type Response = protos::get_config::Response;
}
