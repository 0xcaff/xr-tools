use crate::proto::net::RawRequest;
use crate::proto::usb::{Empty, RawResponse, UsbTransaction};
use std::marker::PhantomData;

pub struct PilotUpdateStart<'a> {
    phantom_data: PhantomData<&'a ()>,
}

impl<'req> UsbTransaction<'req> for PilotUpdateStart<'req> {
    const COMMAND_ID: [u8; 2] = [0x14, 0x12];
    type RequestArgs = RawRequest<'req>;

    type Response = RawResponse;
}

pub struct PilotUpdateTransmit<'req> {
    _data: PhantomData<&'req ()>,
}

impl<'req> UsbTransaction<'req> for PilotUpdateTransmit<'req> {
    const COMMAND_ID: [u8; 2] = [0x15, 0x12];
    type RequestArgs = RawRequest<'req>;
    type Response = ();
}

pub struct PilotUpdateFinish;

impl UsbTransaction<'static> for PilotUpdateFinish {
    const COMMAND_ID: [u8; 2] = [0x16, 0x12];
    type RequestArgs = Empty;
    type Response = ();
}
