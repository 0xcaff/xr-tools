use crate::proto::net::{protos, NetworkTransaction};

pub struct DpSetCurrentEdidDsp;

impl NetworkTransaction for DpSetCurrentEdidDsp {
    const MAGIC: [u8; 2] = [0x27, 0x5f];
    type RequestArgs = protos::dp_get_current_edid_bsp::Request;
    type Response = protos::dp_get_current_edid_bsp::Response;
}


