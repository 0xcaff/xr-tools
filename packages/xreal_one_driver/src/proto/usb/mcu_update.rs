use crate::proto::net::RawRequest;
use crate::proto::usb::{Empty, RequestArgs, UsbTransaction};
use anyhow::Error;
use std::borrow::Cow;

pub struct McuUpdateKernelStart;

pub struct McuUpdateKernelStartRequest {
    checksum: u32,
    length: u32,
}

impl <'a> RequestArgs<'a> for McuUpdateKernelStartRequest {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, Error> {
        let mut out = [0u8; 8];
        out[..4].copy_from_slice(&self.checksum.to_le_bytes());
        out[4..8].copy_from_slice(&self.length.to_le_bytes());
        Ok(Cow::Owned(out.to_vec()))
    }
}

impl UsbTransaction<'static> for McuUpdateKernelStart {
    const COMMAND_ID: [u8; 2] = [0x03, 0x12];
    type RequestArgs = McuUpdateKernelStartRequest;
    type Response = ();
}

pub struct McuUpdateKernelTransmit;

impl <'req> UsbTransaction<'req> for McuUpdateKernelTransmit {
    const COMMAND_ID: [u8; 2] = [0x04, 0x12];
    type RequestArgs = RawRequest<'req>;
    type Response = ();
}

pub struct McuUpdateKernelFinish;

impl UsbTransaction<'static> for McuUpdateKernelFinish {
    const COMMAND_ID: [u8; 2] = [0x05, 0x12];
    type RequestArgs = Empty;
    type Response = ();
}

pub struct McuUpdateSegmentStart;

pub struct McuUpdateSegmentStartRequest {
    checksum: u32,
    flash_offset: u64,
    decompressed_len: u64,
}

impl RequestArgs<'static> for McuUpdateSegmentStartRequest {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, Error> {
        let mut out = [0u8; 20];
        out[..4].copy_from_slice(&self.checksum.to_le_bytes());
        out[4..12].copy_from_slice(&self.flash_offset.to_le_bytes());
        out[12..20].copy_from_slice(&self.decompressed_len.to_le_bytes());
        Ok(Cow::Owned(out.to_vec()))
    }
}

impl UsbTransaction<'static> for McuUpdateSegmentStart {
    const COMMAND_ID: [u8; 2] = [0x06, 0x12];
    type RequestArgs = McuUpdateSegmentStartRequest;
    type Response = ();
}

pub struct McuUpdateSegmentTransmit;

impl <'req> UsbTransaction<'req> for McuUpdateSegmentTransmit {
    const COMMAND_ID: [u8; 2] = [0x07, 0x12];
    type RequestArgs = RawRequest<'req>;
    type Response = ();
}

pub struct McuUpdateSegmentFinish;

pub struct McuUpdateSegmentFinishRequest;

impl <'req> UsbTransaction<'req> for McuUpdateSegmentFinish {
    const COMMAND_ID: [u8; 2] = [0x08, 0x12];
    type RequestArgs = RawRequest<'req>;
    type Response = ();
}

impl RequestArgs<'static> for McuUpdateSegmentFinishRequest {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, Error> {
        Ok(Cow::Borrowed(&[0xff]))
    }
}