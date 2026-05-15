use crate::proto::net::RawRequest;
use crate::proto::usb::{Empty, RequestArgs, UsbDevice, UsbTransaction};
use anyhow::bail;
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;

#[derive(Pod, Copy, Clone, Zeroable, Debug)]
#[repr(C)]
pub struct DspFirmwareHeader {
    pub checksum_or_tag_raw: [u8; 4],
    pub file_size_minus_8_raw: [u8; 4],
    pub platform_code_raw: [u8; 4],
    pub component_type_raw: [u8; 4],
    pub version_raw: [u8; 20],
    pub build_timestamp_raw: [u8; 16],
    pub reserved: [u8; 12],
}

const _: [(); DspFirmwareHeader::LEN] = [(); size_of::<DspFirmwareHeader>()];

impl DspFirmwareHeader {
    pub const LEN: usize = 64;
    pub const COMPONENT_TYPE: u32 = 3;

    pub fn load(bytes: &[u8]) -> Result<&Self, anyhow::Error> {
        if bytes.len() < Self::LEN {
            bail!("DSP update must be at least {} bytes", Self::LEN);
        }

        let header = bytemuck::from_bytes::<Self>(&bytes[..Self::LEN]);
        if header.component_type() != Self::COMPONENT_TYPE {
            bail!(
                "invalid DSP component type {}, expected {}",
                header.component_type(),
                Self::COMPONENT_TYPE
            );
        }

        Ok(header)
    }

    pub fn checksum_or_tag(&self) -> u32 {
        u32::from_le_bytes(self.checksum_or_tag_raw)
    }

    pub fn file_size_minus_8(&self) -> u32 {
        u32::from_le_bytes(self.file_size_minus_8_raw)
    }

    pub fn platform_code(&self) -> u32 {
        u32::from_le_bytes(self.platform_code_raw)
    }

    pub fn component_type(&self) -> u32 {
        u32::from_le_bytes(self.component_type_raw)
    }

    pub fn version(&self) -> Result<&str, std::str::Utf8Error> {
        nul_trimmed_str(&self.version_raw)
    }

    pub fn build_timestamp(&self) -> Result<&str, std::str::Utf8Error> {
        nul_trimmed_str(&self.build_timestamp_raw)
    }
}

impl<'a> RequestArgs<'a> for &'a DspFirmwareHeader {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(bytemuck::bytes_of(*self)))
    }
}

fn nul_trimmed_str(bytes: &[u8]) -> Result<&str, std::str::Utf8Error> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end])
}

pub struct DspUpdateStart;

impl<'req> UsbTransaction<'req> for DspUpdateStart {
    const COMMAND_ID: [u8; 2] = [0x46, 0x00];
    type RequestArgs = &'req DspFirmwareHeader;
    type Response = ();
}

pub struct DspUpdateTransmit;

impl<'req> UsbTransaction<'req> for DspUpdateTransmit {
    const COMMAND_ID: [u8; 2] = [0x47, 0x00];
    type RequestArgs = RawRequest<'req>;
    type Response = ();
}

pub struct DspUpdateFinish;

impl UsbTransaction<'static> for DspUpdateFinish {
    const COMMAND_ID: [u8; 2] = [0x48, 0x00];
    type RequestArgs = Empty;
    type Response = ();
}

pub trait DspUpdateProgressReporter {
    fn transmit(&mut self, _length: usize) {}
}

impl UsbDevice {
    pub fn update_dsp(&self, update: &[u8]) -> Result<(), anyhow::Error> {
        struct EmptyReporter;
        impl DspUpdateProgressReporter for EmptyReporter {}

        self.update_dsp_with_progress(update, &mut EmptyReporter)?;

        Ok(())
    }

    pub fn update_dsp_with_progress(
        &self,
        update: &[u8],
        progress: &mut impl DspUpdateProgressReporter,
    ) -> Result<(), anyhow::Error> {
        let header = DspFirmwareHeader::load(update)?;

        self.send_message::<DspUpdateStart>(header)?;
        progress.transmit(DspFirmwareHeader::LEN);

        let mut position = DspFirmwareHeader::LEN;
        while position < update.len() {
            let end_position = std::cmp::min(position + 1002, update.len());
            self.send_message::<DspUpdateTransmit>(RawRequest(&update[position..end_position]))?;
            progress.transmit(end_position - position);

            position = end_position;
        }

        self.send_message::<DspUpdateFinish>(Empty)?;

        Ok(())
    }
}
