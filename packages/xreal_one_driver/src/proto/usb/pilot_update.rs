use crate::proto::net::RawRequest;
use crate::proto::usb::{Empty, RawResponse, UsbDevice, UsbTransaction};
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

impl UsbDevice {
    pub fn update_pilot(&self, update: &[u8]) -> Result<(), anyhow::Error> {
        self.send_message::<PilotUpdateStart>(RawRequest(&update[0..64]))?;

        let mut position = 64;
        while position < update.len() {
            let end_position = std::cmp::min(position + 1002, update.len());
            self.send_message::<PilotUpdateTransmit>(RawRequest(&update[position..end_position]))?;
            position = end_position;
        }

        self.send_message::<PilotUpdateFinish>(Empty)?;

        Ok(())
    }
}
