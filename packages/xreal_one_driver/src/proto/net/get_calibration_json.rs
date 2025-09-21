use crate::proto::net::{protos, NetworkTransaction, Response};
use crate::proto::usb::ConstRequestArgs;
use anyhow::Error;
use protobuf::Message;

pub struct GetCalibrationJson;

impl NetworkTransaction for GetCalibrationJson {
    const MAGIC: [u8; 2] = [0x27, 0x1f];
    type RequestArgs = GetCalibrationJsonRequest;
    type Response = protos::get_calibration_json::GetCalibrationJsonResponse;
}

pub struct GetCalibrationJsonRequest;

impl ConstRequestArgs for GetCalibrationJsonRequest {
    const VALUE: &'static [u8] = &[0x1a, 0x00];
}

