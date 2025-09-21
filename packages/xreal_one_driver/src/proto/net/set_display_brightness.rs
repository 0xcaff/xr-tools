use crate::proto::net::{protos, NetworkTransaction};

pub struct SetDisplayBrightness;

impl NetworkTransaction for SetDisplayBrightness {
    const MAGIC: [u8; 2] = [0x27, 0x1c];
    type RequestArgs = protos::set_display_brightness::Request;
    type Response = protos::set_display_brightness::Response;
}
