use crate::proto::net::{protos, NetworkTransaction};

pub struct DisplayStopOSDRender;

impl NetworkTransaction<'static> for DisplayStopOSDRender {
    const MAGIC: [u8; 2] = [0x28, 0x29];
    type RequestArgs = protos::set_scene_mode::Request;
    type Response = protos::set_scene_mode::Response;
}
