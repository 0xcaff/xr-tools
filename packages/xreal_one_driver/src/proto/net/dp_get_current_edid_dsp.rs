use crate::proto::net::{protos, NetworkTransaction};

pub struct DpGetCurrentEdidDsp;

impl NetworkTransaction for DpGetCurrentEdidDsp {
    const MAGIC: [u8; 2] = [0x27, 0x5e];
    type RequestArgs = protos::dp_get_current_edid_bsp::Request;
    type Response = protos::dp_get_current_edid_bsp::Response;
}


