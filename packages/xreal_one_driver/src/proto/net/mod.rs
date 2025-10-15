pub mod config;
pub mod control;
pub mod display_set_scene_mode;
pub mod dp_get_current_edid_dsp;
pub mod dp_set_current_edid_dsp;
pub mod dp_set_input_mode;
pub mod get_config;
pub mod glasses_get_dsp_version;
pub mod glasses_get_id;
pub mod glasses_get_sw_version;
pub mod key_submit_state;
pub mod props;
pub mod reports;
pub mod set_display_brightness;
pub mod set_elechromic_dimmer;

use crate::proto::usb::RequestArgs;
use futures::TryStreamExt;
use std::borrow::Cow;

#[derive(Debug)]
pub struct RawResponse(pub Vec<u8>);

impl Response for RawResponse {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error> {
        Ok(Self(buffer))
    }
}

pub struct RawRequest<'a>(pub &'a [u8]);

impl<'a> RequestArgs<'a> for RawRequest<'a> {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(self.0))
    }
}

pub trait NetworkTransaction<'request> {
    const MAGIC: [u8; 2];
    type RequestArgs: RequestArgs<'request>;
    type Response: Response;
}

trait InboundMessage: Response {
    const MAGIC: [u8; 2];
}

pub trait Response: Sized {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error>;
}
