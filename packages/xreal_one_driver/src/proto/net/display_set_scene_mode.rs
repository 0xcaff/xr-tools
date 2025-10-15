use crate::proto::net::props::{EmptyPropertyResponse, SetNumericProperty, SetPropertyRequest};
use crate::proto::net::NetworkTransaction;
use strum::FromRepr;

pub struct SetSceneMode;

impl NetworkTransaction<'static> for SetSceneMode {
    const MAGIC: [u8; 2] = [0x28, 0x29];
    type RequestArgs = SetPropertyRequest<SetNumericProperty<SceneMode>>;
    type Response = EmptyPropertyResponse;
}

#[derive(Debug, Clone, Copy, FromRepr)]
#[repr(u8)]
pub enum SceneMode {
    ButtonsEnabled = 0,
    ButtonsDisabled = 1,
}

impl Into<u8> for SceneMode {
    fn into(self) -> u8 {
        self as u8
    }
}
