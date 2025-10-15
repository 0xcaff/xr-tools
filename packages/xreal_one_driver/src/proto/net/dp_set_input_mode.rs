use crate::proto::net::props::{EmptyPropertyResponse, SetNumericProperty, SetPropertyRequest};
use crate::proto::net::NetworkTransaction;
use strum::FromRepr;

pub struct DpSetInputMode;

impl NetworkTransaction<'static> for DpSetInputMode {
    const MAGIC: [u8; 2] = [0x28, 0x22];
    type RequestArgs = SetPropertyRequest<SetNumericProperty<InputMode>>;
    type Response = EmptyPropertyResponse;
}

#[derive(Debug, Clone, Copy, FromRepr)]
#[repr(u8)]
pub enum InputMode {
    Regular = 0,
    SideBySide = 1,
}

impl Into<u8> for InputMode {
    fn into(self) -> u8 {
        self as u8
    }
}
