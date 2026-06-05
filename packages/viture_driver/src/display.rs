use crate::{Empty, RequestArgs, Response, UsbTransaction};
use anyhow::{bail, Result};
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DisplayMode {
    Fhd60Hz = 0x31,
    Fhd3d60Hz = 0x32,
    Fhd90Hz = 0x33,
    Fhd120Hz = 0x34,
    Fhd3d90Hz = 0x35,
    Wuxga60Hz = 0x41,
    Wuxga3d60Hz = 0x42,
    Wuxga90Hz = 0x43,
    Wuxga120Hz = 0x44,
    Wuxga3d90Hz = 0x45,
}

impl DisplayMode {
    pub const STANDARD_MODES: &'static [Self] = &[
        Self::Fhd60Hz,
        Self::Fhd3d60Hz,
        Self::Fhd90Hz,
        Self::Fhd120Hz,
        Self::Fhd3d90Hz,
        Self::Wuxga60Hz,
        Self::Wuxga3d60Hz,
        Self::Wuxga90Hz,
        Self::Wuxga120Hz,
        Self::Wuxga3d90Hz,
    ];

    pub fn from_wire_value(value: u8) -> Result<Self> {
        value.try_into()
    }

    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Fhd60Hz => "1920x1080 @ 60Hz",
            Self::Fhd3d60Hz => "3840x1080 @ 60Hz 3D",
            Self::Fhd90Hz => "1920x1080 @ 90Hz",
            Self::Fhd120Hz => "1920x1080 @ 120Hz",
            Self::Fhd3d90Hz => "3840x1080 @ 90Hz 3D",
            Self::Wuxga60Hz => "1920x1200 @ 60Hz",
            Self::Wuxga3d60Hz => "3840x1200 @ 60Hz 3D",
            Self::Wuxga90Hz => "1920x1200 @ 90Hz",
            Self::Wuxga120Hz => "1920x1200 @ 120Hz",
            Self::Wuxga3d90Hz => "3840x1200 @ 90Hz 3D",
        }
    }
}

impl fmt::Display for DisplayMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} (0x{:02x})",
            self.description(),
            self.wire_value()
        )
    }
}

impl TryFrom<u8> for DisplayMode {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x31 => Ok(Self::Fhd60Hz),
            0x32 => Ok(Self::Fhd3d60Hz),
            0x33 => Ok(Self::Fhd90Hz),
            0x34 => Ok(Self::Fhd120Hz),
            0x35 => Ok(Self::Fhd3d90Hz),
            0x41 => Ok(Self::Wuxga60Hz),
            0x42 => Ok(Self::Wuxga3d60Hz),
            0x43 => Ok(Self::Wuxga90Hz),
            0x44 => Ok(Self::Wuxga120Hz),
            0x45 => Ok(Self::Wuxga3d90Hz),
            _ => bail!("unsupported VITURE SDK display mode 0x{value:02x}"),
        }
    }
}

impl From<DisplayMode> for u8 {
    fn from(mode: DisplayMode) -> Self {
        mode.wire_value()
    }
}

impl RequestArgs<'static> for DisplayMode {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>> {
        Ok(Cow::Owned(vec![self.wire_value()]))
    }
}

impl Response for DisplayMode {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        match buffer {
            [mode] => Self::from_wire_value(*mode),
            [] => bail!("VITURE display mode response was empty"),
            _ => bail!(
                "expected one-byte VITURE display mode response, got {} bytes",
                buffer.len()
            ),
        }
    }
}

pub struct GetDisplayMode;

impl UsbTransaction<'static> for GetDisplayMode {
    const COMMAND_ID: [u8; 2] = [0x07, 0x00];

    type RequestArgs = Empty;
    type Response = DisplayMode;
}

pub struct SetDisplayMode;

impl UsbTransaction<'static> for SetDisplayMode {
    const COMMAND_ID: [u8; 2] = [0x08, 0x00];

    type RequestArgs = DisplayMode;
    type Response = DisplayMode;
}
