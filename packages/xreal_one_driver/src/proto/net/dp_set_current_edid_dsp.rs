use crate::proto::net::dp_get_current_edid_dsp::DisplayConfiguration;
use crate::proto::net::props::{
    EmptyPropertyResponse, SetNumericProperty, SetPropertyRequest,
};
use crate::proto::net::NetworkTransaction;

pub struct DpSetCurrentEdidDsp;

impl NetworkTransaction<'static> for DpSetCurrentEdidDsp {
    const MAGIC: [u8; 2] = [0x27, 0x5f];
    type RequestArgs = SetPropertyRequest<SetNumericProperty<DisplayConfiguration>>;
    type Response = EmptyPropertyResponse;
}
