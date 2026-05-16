use crate::proto::usb::RequestArgs;
use anyhow::bail;
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;

pub const XREAL_ONE_PLATFORM_CODE: u32 = 0x1500;

#[derive(Pod, Copy, Clone, Zeroable, Debug)]
#[repr(transparent)]
pub struct LeU32([u8; 4]);

impl LeU32 {
    pub fn get(&self) -> u32 {
        u32::from_le_bytes(self.0)
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct FirmwareHeader<const COMPONENT_TYPE: u32> {
    pub checksum_or_tag: LeU32,
    pub file_size_minus_8: LeU32,
    pub platform_code: LeU32,
    pub component_type: LeU32,
    pub version_raw: [u8; 20],
    pub build_timestamp_raw: [u8; 16],
    pub reserved: [u8; 12],
}

const _: [(); FirmwareHeader::<0>::LEN] = [(); size_of::<FirmwareHeader<0>>()];

// SAFETY: every field is byte-backed with alignment 1, and the size assertion
// above verifies that the wire layout has no padding.
unsafe impl<const COMPONENT_TYPE: u32> Zeroable for FirmwareHeader<COMPONENT_TYPE> {}
unsafe impl<const COMPONENT_TYPE: u32> Pod for FirmwareHeader<COMPONENT_TYPE> {}

impl<const EXPECTED_COMPONENT_TYPE: u32> FirmwareHeader<EXPECTED_COMPONENT_TYPE> {
    pub const LEN: usize = 64;

    pub fn load_named<'a>(bytes: &'a [u8], name: &str) -> Result<&'a Self, anyhow::Error> {
        if bytes.len() < Self::LEN {
            bail!("{} update must be at least {} bytes", name, Self::LEN);
        }

        let header = bytemuck::from_bytes::<Self>(&bytes[..Self::LEN]);
        if header.platform_code.get() != XREAL_ONE_PLATFORM_CODE {
            bail!(
                "invalid {} platform code {:#x}, expected {:#x}",
                name,
                header.platform_code.get(),
                XREAL_ONE_PLATFORM_CODE
            );
        }

        if header.component_type.get() != EXPECTED_COMPONENT_TYPE {
            bail!(
                "invalid {} component type {}, expected {}",
                name,
                header.component_type.get(),
                EXPECTED_COMPONENT_TYPE
            );
        }

        Ok(header)
    }
}

impl FirmwareHeader<1> {
    pub fn load(bytes: &[u8]) -> Result<&Self, anyhow::Error> {
        Self::load_named(bytes, "MCU recovery")
    }
}

impl FirmwareHeader<3> {
    pub fn load(bytes: &[u8]) -> Result<&Self, anyhow::Error> {
        Self::load_named(bytes, "DSP")
    }
}

impl FirmwareHeader<4> {
    pub fn load(bytes: &[u8]) -> Result<&Self, anyhow::Error> {
        Self::load_named(bytes, "Pilot")
    }
}

impl<'a, const COMPONENT_TYPE: u32> RequestArgs<'a> for &'a FirmwareHeader<COMPONENT_TYPE> {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(bytemuck::bytes_of(*self)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of};

    #[test]
    fn header_layout_matches_wire_format() {
        assert_eq!(size_of::<FirmwareHeader<3>>(), 64);
        assert_eq!(align_of::<FirmwareHeader<3>>(), 1);
        assert_eq!(offset_of!(FirmwareHeader<3>, checksum_or_tag), 0x00);
        assert_eq!(offset_of!(FirmwareHeader<3>, file_size_minus_8), 0x04);
        assert_eq!(offset_of!(FirmwareHeader<3>, platform_code), 0x08);
        assert_eq!(offset_of!(FirmwareHeader<3>, component_type), 0x0c);
        assert_eq!(offset_of!(FirmwareHeader<3>, version_raw), 0x10);
        assert_eq!(offset_of!(FirmwareHeader<3>, build_timestamp_raw), 0x24);
        assert_eq!(offset_of!(FirmwareHeader<3>, reserved), 0x34);
    }

    #[test]
    fn integer_fields_decode_as_little_endian_without_changing_bytes() {
        let mut bytes = [0u8; FirmwareHeader::<3>::LEN];
        bytes[0x00..0x04].copy_from_slice(&0x12345678u32.to_le_bytes());
        bytes[0x04..0x08].copy_from_slice(&0x90abcdefu32.to_le_bytes());
        bytes[0x08..0x0c].copy_from_slice(&XREAL_ONE_PLATFORM_CODE.to_le_bytes());
        bytes[0x0c..0x10].copy_from_slice(&3u32.to_le_bytes());

        let header = FirmwareHeader::<3>::load(&bytes).unwrap();

        assert_eq!(header.checksum_or_tag.get(), 0x12345678);
        assert_eq!(header.file_size_minus_8.get(), 0x90abcdef);
        assert_eq!(header.platform_code.get(), XREAL_ONE_PLATFORM_CODE);
        assert_eq!(header.component_type.get(), 3);
        assert_eq!(bytemuck::bytes_of(header), bytes);
    }
}
