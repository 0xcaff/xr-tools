#[derive(Copy, Clone, Debug)]
pub struct EnableValue(pub bool);

impl Into<u8> for EnableValue {
    fn into(self) -> u8 {
        self.0 as u8
    }
}

impl From<u8> for EnableValue {
    fn from(value: u8) -> Self {
        Self(value != 0)
    }
}
