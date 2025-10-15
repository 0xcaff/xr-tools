use crate::proto::net::props::{EmptyPropertyResponse, SetNumericProperty, SetPropertyRequest};
use crate::proto::net::NetworkTransaction;

pub struct SetDisplayBrightness;

impl NetworkTransaction<'static> for SetDisplayBrightness {
    const MAGIC: [u8; 2] = [0x27, 0x1c];
    type RequestArgs = SetPropertyRequest<SetNumericProperty<DisplayBrightness>>;
    type Response = EmptyPropertyResponse;
}

#[derive(Copy, Clone)]
pub struct DisplayBrightness(pub u8);

impl Into<u8> for DisplayBrightness {
    fn into(self) -> u8 {
        self.0
    }
}
