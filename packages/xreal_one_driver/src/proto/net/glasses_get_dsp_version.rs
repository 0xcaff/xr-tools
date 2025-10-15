use crate::proto::net::props::{GetPropertyRequest, PropertyResponse};
use crate::proto::net::NetworkTransaction;

pub struct GlassesGetDspVersion;

impl NetworkTransaction<'static> for GlassesGetDspVersion {
    const MAGIC: [u8; 2] = [0x27, 0x2d];
    type RequestArgs = GetPropertyRequest;
    type Response = PropertyResponse<String>;
}
