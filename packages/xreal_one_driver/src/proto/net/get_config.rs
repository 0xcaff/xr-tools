use crate::proto::net::{protos, NetworkTransaction};

pub struct GetConfig;

impl NetworkTransaction<'static> for GetConfig {
    const MAGIC: [u8; 2] = [0x27, 0x1f];
    type RequestArgs = protos::get_config::Request;
    type Response = protos::get_config::Response;
}
