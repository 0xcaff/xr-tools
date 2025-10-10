use crate::proto::usb::{Empty, RequestArgs, UsbTransaction};
use anyhow::Error;
use std::borrow::Cow;
use std::marker::PhantomData;

pub struct PilotUpdateStart<'a> {
    phantom_data: PhantomData<&'a ()>,
}

pub struct PilotUpdateBody<'a> {
    bytes: &'a [u8]
}

impl <'req> RequestArgs<'req> for PilotUpdateBody<'req> {
    fn as_bytes(&self) -> Result<Cow<'req, [u8]>, Error> {
        Ok(Cow::Borrowed(self.bytes))
    }
}

impl <'req> UsbTransaction<'req> for PilotUpdateStart<'req> {
    const COMMAND_ID: [u8; 2] = [0x14, 0x12];
    type RequestArgs = PilotUpdateBody<'req>;

    // todo: there are bytes in the response
    type Response = ();
}

pub struct PilotUpdateTransmit<'req> {
    _data: PhantomData<&'req ()>,
}

impl <'req> UsbTransaction<'req> for PilotUpdateTransmit<'req> {
    const COMMAND_ID: [u8; 2] = [0x15, 0x12];
    type RequestArgs = PilotUpdateBody<'req>;
    type Response = ();
}

pub struct PilotUpdateFinish;

impl  UsbTransaction<'static> for PilotUpdateFinish {
    const COMMAND_ID: [u8; 2] = [0x16, 0x12];
    type RequestArgs = Empty;
    type Response = ();
}
