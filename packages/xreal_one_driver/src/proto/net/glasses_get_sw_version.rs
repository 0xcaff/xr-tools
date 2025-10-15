use crate::proto::net::props::{GetPropertyRequest, PropertyResponse};
use crate::proto::net::NetworkTransaction;

pub struct GlassesGetFwVersion;

impl NetworkTransaction<'static> for GlassesGetFwVersion {
    const MAGIC: [u8; 2] = [0x27, 0x1d];
    type RequestArgs = GetPropertyRequest;
    type Response = PropertyResponse<String>;
}
