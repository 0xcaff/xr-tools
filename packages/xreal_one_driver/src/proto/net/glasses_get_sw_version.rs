use crate::proto::net::{protos, NetworkTransaction};

pub struct GlassesGetFwVersion;

impl NetworkTransaction<'static> for GlassesGetFwVersion {
    const MAGIC: [u8; 2] = [0x27, 0x1d];
    type RequestArgs = protos::get_config::Request;
    type Response = protos::get_config::Response;
}