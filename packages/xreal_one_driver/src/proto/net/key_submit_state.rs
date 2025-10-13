use crate::proto::net::{InboundRequest, Response};
use anyhow::format_err;
use strum::FromRepr;

#[derive(Debug)]
pub struct KeySubmitState {
    key_type: KeyType,
    key_state: KeyState,
    hmd_time_nanos_device: u32,
}

impl InboundRequest for KeySubmitState {
    const MAGIC: [u8; 2] = [0x27, 0x2e];
}

#[derive(Debug, FromRepr)]
#[repr(u32)]
pub enum KeyType {
    BottomSingleButton = 1,
    FrontRockerButton = 2,
    BackRockerButton = 3,
    TopSingleButton = 4,
}

#[derive(Debug, FromRepr)]
#[repr(u32)]
pub enum KeyState {
    Down = 1,
    Up = 2,
}

impl Response for KeySubmitState {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error> {
        assert_eq!(buffer.len(), 64);

        let key_type = u32::from_le_bytes(buffer[0..4].try_into()?);
        let key_state = u32::from_le_bytes(buffer[4..8].try_into()?);
        let hmd_time_nanos_device = u32::from_le_bytes(buffer[8..12].try_into()?);

        Ok(Self {
            key_type: KeyType::from_repr(key_type)
                .ok_or_else(|| format_err!("unknown key type {}", key_type))?,
            key_state: KeyState::from_repr(key_state)
                .ok_or_else(|| format_err!("unknown key state {}", key_state))?,
            hmd_time_nanos_device,
        })
    }
}
