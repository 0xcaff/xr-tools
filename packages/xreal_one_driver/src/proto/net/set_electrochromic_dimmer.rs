use crate::proto::net::{protos, NetworkTransaction};

pub struct SetElectrochromicDimmer;

impl NetworkTransaction<'static> for SetElectrochromicDimmer {
    const MAGIC: [u8; 2] = [0x27, 0x27];
    type RequestArgs = protos::set_electrochromic_dimmer::Request;
    type Response = protos::set_electrochromic_dimmer::Response;
}
