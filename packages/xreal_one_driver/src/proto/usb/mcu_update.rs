use crate::proto::net::RawRequest;
use crate::proto::usb::{Empty, RequestArgs, UsbDevice, UsbTransaction};
use anyhow::bail;
use binrw::{binrw, BinReaderExt};
use std::borrow::Cow;
use std::io::{Cursor, SeekFrom};

pub struct McuUpdateKernelStart;

pub struct McuUpdateKernelStartRequest {
    checksum: u32,
    length: u32,
}

impl<'a> RequestArgs<'a> for McuUpdateKernelStartRequest {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, anyhow::Error> {
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

impl<'req> UsbTransaction<'req> for McuUpdateKernelTransmit {
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
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, anyhow::Error> {
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

impl<'req> UsbTransaction<'req> for McuUpdateSegmentTransmit {
    const COMMAND_ID: [u8; 2] = [0x07, 0x12];
    type RequestArgs = RawRequest<'req>;
    type Response = ();
}

pub struct McuUpdateSegmentFinish;

pub struct McuUpdateSegmentFinishRequest;

impl<'req> UsbTransaction<'req> for McuUpdateSegmentFinish {
    const COMMAND_ID: [u8; 2] = [0x08, 0x12];
    type RequestArgs = RawRequest<'req>;
    type Response = ();
}

impl RequestArgs<'static> for McuUpdateSegmentFinishRequest {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(&[0xff]))
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
pub struct UpgradeHeader {
    #[br(seek_before = SeekFrom::Start(0x40))]
    pub magic: [u8; 4], // 0x40
    pub hdr_version: u8,                   // 0x44
    pub compressed: u8,                    // 0x45
    pub flash_type: u8,                    // 0x46
    pub reserved_47: u8,                   // 0x47
    pub header_ext_sz: u16,                // 0x48
    pub hash_sz: u16,                      // 0x4A
    pub sig_sz: u16,                       // 0x4C
    pub sig_real_sz: u16,                  // 0x4E
    pub img_size: u64,                     // 0x50
    pub rom_size: u32,                     // 0x58
    pub loader_size: u32,                  // 0x5C
    pub partitions_cnt: u16,               // 0x60
    pub segments_cnt: u16,                 // 0x62
    pub object_version: u32,               // 0x64
    pub depend_version: u32,               // 0x68
    pub reserved_tail: [u8; 0x140 - 0x6C], // 0x6C..0x13F

    // ===== variable sections (cursor is at 0x140 here) =====
    #[br(count = header_ext_sz)]
    pub hdr_ext: Vec<u8>,

    #[br(count = 32)]
    pub hash: Vec<u8>,

    #[br(count = 256)]
    pub rsa: Vec<u8>,

    #[br(count = rom_size)]
    pub rom_code: Vec<u8>,

    #[br(count = loader_size)]
    pub bootloader: Vec<u8>,

    // ===== tables =====
    #[br(count = partitions_cnt)]
    pub partitions: Vec<PartitionEntry>,

    #[br(count = segments_cnt)]
    pub segments: Vec<SegmentEntry>,
}

impl UpgradeHeader {
    pub fn load(bytes: &[u8]) -> Result<(Self, usize), anyhow::Error> {
        let mut reader = Cursor::new(bytes);
        let header: UpgradeHeader = reader.read_le()?;
        if &header.magic != b"OTRA" {
            bail!("invalid magic");
        }

        Ok((header, reader.position() as usize))
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
pub struct PartitionEntry {
    pub name_raw: [u8; 32],
    pub _reserved8: [u8; 8],
    pub length: u64,
    #[br(count = 4)]
    pub _tail: Vec<u8>,
}

impl PartitionEntry {
    pub fn name(&self) -> &str {
        let end = self
            .name_raw
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.name_raw.len());
        std::str::from_utf8(&self.name_raw[..end]).unwrap_or("")
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
pub struct SegmentEntry {
    pub img_offset: u64,
    pub flash_offset: u64,
    pub compress_size: u64,
    pub decompress_size: u64,
}

impl SegmentEntry {
    pub fn load(&self, bytes: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        let start_offset = (self.img_offset as usize) + 64;
        let segment_bytes = &bytes[start_offset..(start_offset + self.compress_size as usize)];

        let decompressed =
            lzokay_native::decompress_all(segment_bytes, Some(self.decompress_size as usize))?;

        if decompressed.len() != self.decompress_size as usize {
            bail!(
                "decompressed size mismatch {} != {}",
                decompressed.len(),
                self.decompress_size as usize
            );
        }

        Ok(decompressed)
    }
}

pub trait McuUpdateProgressReporter {
    fn transmit(&mut self, _length: usize) {}
}

pub struct McuUpdate<'a> {
    bytes: &'a [u8],
    upgrade_header: UpgradeHeader,
    header_end_offset: usize,
}

impl McuUpdate<'_> {
    pub fn parse<'a>(bytes: &'a [u8]) -> Result<McuUpdate<'a>, anyhow::Error> {
        let (upgrade_header, header_end_offset) = UpgradeHeader::load(bytes)?;

        Ok(McuUpdate {
            bytes,
            upgrade_header,
            header_end_offset,
        })
    }

    pub fn size(&self) -> usize {
        self.header_end_offset
            + self
                .upgrade_header
                .segments
                .iter()
                .map(|s| s.decompress_size as usize)
                .sum::<usize>()
    }

    pub fn kernel_bytes(&self) -> &[u8] {
        &self.bytes[..self.header_end_offset]
    }
}

impl UsbDevice {
    pub fn update_mcu(&self, update: McuUpdate) -> Result<(), anyhow::Error> {
        struct EmptyReporter;

        impl McuUpdateProgressReporter for EmptyReporter {}

        self.update_mcu_with_progress(update, &mut EmptyReporter)
    }

    pub fn update_mcu_with_progress(
        &self,
        update: McuUpdate,
        progress: &mut impl McuUpdateProgressReporter,
    ) -> Result<(), anyhow::Error> {
        let kernel_bytes = update.kernel_bytes();

        let checksum = crc_adler::crc32(kernel_bytes);

        self.send_message::<McuUpdateKernelStart>(McuUpdateKernelStartRequest {
            checksum,
            length: kernel_bytes.len() as u32,
        })?;

        let mut offset = 0;

        while offset < kernel_bytes.len() {
            let end_offset = std::cmp::min(offset + 1002, kernel_bytes.len());
            let segment = &kernel_bytes[offset..end_offset];

            self.send_message::<McuUpdateKernelTransmit>(RawRequest(segment))?;
            progress.transmit(end_offset - offset);

            offset = end_offset;
        }

        self.send_message::<McuUpdateKernelFinish>(Empty)?;

        for segment in &update.upgrade_header.segments {
            let segment_bytes = segment.load(update.bytes)?;
            self.send_message::<McuUpdateSegmentStart>(McuUpdateSegmentStartRequest {
                checksum: crc_adler::crc32(&segment_bytes),
                flash_offset: segment.flash_offset,
                decompressed_len: segment.decompress_size,
            })?;

            let mut offset = 0;
            while offset < segment_bytes.len() {
                let end_offset = std::cmp::min(offset + 1002, segment_bytes.len());
                let segment = &segment_bytes[offset..end_offset];

                self.send_message::<McuUpdateSegmentTransmit>(RawRequest(segment))?;
                progress.transmit(end_offset - offset);
                offset = end_offset;
            }

            self.send_message::<McuUpdateSegmentFinish>(RawRequest(&[0xff]))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::proto::usb::mcu_update::UpgradeHeader;
    use std::fs;

    #[test]
    #[ignore]
    fn read_update() -> Result<(), anyhow::Error> {
        let bytes = fs::read("/Volumes/data-lake/data-lake-1/xreal-one-updates/02.746_20250925_01.097_20250430_1.7.0.20250925155123/mcu/15.1.02.746_20250925.bin")?;
        let (header, header_end_offset) = UpgradeHeader::load(&bytes)?;

        println!("{:?}", &header);

        let kernel_bytes = &bytes[..header_end_offset];

        let checksum = crc_adler::crc32(kernel_bytes);

        println!("kernel checksum: {:x}", checksum);
        println!("kernel size: {:x}", kernel_bytes.len());

        for segment in &header.segments {
            let segment_bytes = segment.load(&bytes)?;

            hexdump::hexdump(&segment_bytes);

            println!("segment checksum: {:x}", crc_adler::crc32(&segment_bytes));
            println!("segment flash offset: {:x}", segment.flash_offset);
            println!("segment size: {:x}", segment_bytes.len());
        }

        Ok(())
    }
}
