use std::io::Empty;
use crate::proto::net::{protos, NetworkTransaction, Response};
use crate::proto::usb::ConstRequestArgs;
use anyhow::Error;
use protobuf::Message;

pub struct GetCalibrationJson;

impl NetworkTransaction for GetCalibrationJson {
    const MAGIC: [u8; 2] = [0x27, 0x1f];
    type RequestArgs = protos::get_calibration_json::Request;
    type Response = protos::get_calibration_json::Response;
}

pub struct GetCalibrationJsonRequest;

