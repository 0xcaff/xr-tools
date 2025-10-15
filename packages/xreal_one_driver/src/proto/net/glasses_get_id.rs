use crate::proto::net::props::{GetPropertyRequest, PropertyResponse};
use crate::proto::net::NetworkTransaction;

pub struct GlassesGetId;

impl NetworkTransaction<'static> for GlassesGetId {
    const MAGIC: [u8; 2] = [0x27, 0x29];
    type RequestArgs = GetPropertyRequest;
    type Response = PropertyResponse<String>;
}
