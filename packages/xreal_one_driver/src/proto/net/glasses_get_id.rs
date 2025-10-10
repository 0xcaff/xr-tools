use crate::proto::net::{protos, NetworkTransaction};

pub struct GlassesGetId;

impl NetworkTransaction<'static> for GlassesGetId {
    const MAGIC: [u8; 2] = [0x27, 0x29];
    type RequestArgs = protos::get_config::Request;
    // todo: response is missing a bunch of things
    type Response = protos::get_config::Response;
}
