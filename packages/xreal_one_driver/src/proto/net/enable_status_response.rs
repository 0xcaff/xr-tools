use crate::proto::net::props::{PropertyResponse, ReadNumericProperty};
use crate::proto::net::Response;

pub struct EnableStatusResponse(pub u8);

impl Response for EnableStatusResponse {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error> {
        if buffer == [0x22, 0x00] {
            return Ok(Self(0));
        }

        let response = PropertyResponse::<ReadNumericProperty<u8>>::deserialize_from(buffer)?;
        Ok(Self(response.value.0))
    }
}
