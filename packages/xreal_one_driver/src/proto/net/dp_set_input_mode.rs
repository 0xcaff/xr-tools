use crate::proto::net::{protos, NetworkTransaction};

pub struct DpSetInputMode;

impl NetworkTransaction for DpSetInputMode {
    const MAGIC: [u8; 2] = [0x28, 0x22];
    
    // 1 = SBS
    // 0 = regular
    type RequestArgs = protos::dp_get_current_edid_bsp::Request;
    type Response = protos::dp_get_current_edid_bsp::Response;
}


