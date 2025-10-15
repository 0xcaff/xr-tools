use crate::proto::net::props::{EmptyPropertyResponse, SetNumericProperty, SetPropertyRequest};
pub use crate::proto::net::NetworkTransaction;

pub struct SetElechromicDimmer;

impl NetworkTransaction<'static> for SetElechromicDimmer {
    const MAGIC: [u8; 2] = [0x27, 0x27];
    type RequestArgs = SetPropertyRequest<SetNumericProperty<ElectricDimmerLevel>>;
    type Response = EmptyPropertyResponse;
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ElectricDimmerLevel {
    Lightest = 0,
    Middle = 1,
    /// The dimmer is set to its maximum level, allowing very little background light through the shade.
    Dimmest = 2,
}

impl Into<u8> for ElectricDimmerLevel {
    fn into(self) -> u8 {
        self as u8
    }
}
