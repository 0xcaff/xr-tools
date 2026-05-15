use crate::proto::net::RawRequest;
use crate::proto::usb::firmware_header::FirmwareHeader;
use crate::proto::usb::{Empty, UsbDevice, UsbInboundMessage, UsbTransaction};
use anyhow::bail;
use std::time::Duration;

pub type DspFirmwareHeader = FirmwareHeader<3>;

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
    fn device_progress(&mut self, _phase: DspUpdatePhase, _percent: u8) {}
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DspUpdatePhase {
    WriteFlash,
    Boot,
    ReadFlash,
}

impl DspUpdatePhase {
    fn commands(self) -> ([u8; 2], [u8; 2]) {
        match self {
            Self::WriteFlash => ([0x0e, 0x6c], [0x11, 0x6c]),
            Self::Boot => ([0x10, 0x6c], [0x0f, 0x6c]),
            Self::ReadFlash => ([0x13, 0x6c], [0x14, 0x6c]),
        }
    }
}

impl UsbDevice {
    pub async fn update_dsp(&self, update: &[u8]) -> Result<(), anyhow::Error> {
        struct EmptyReporter;
        impl DspUpdateProgressReporter for EmptyReporter {}

        self.update_dsp_with_progress(update, &mut EmptyReporter)
            .await?;

        Ok(())
    }

    pub async fn update_dsp_with_progress(
        &self,
        update: &[u8],
        progress: &mut impl DspUpdateProgressReporter,
    ) -> Result<(), anyhow::Error> {
        let header = DspFirmwareHeader::load(update)?;

        self.send_message::<DspUpdateStart>(header).await?;
        progress.transmit(DspFirmwareHeader::LEN);

        let mut position = DspFirmwareHeader::LEN;
        while position < update.len() {
            let end_position = std::cmp::min(position + 1002, update.len());
            self.send_message::<DspUpdateTransmit>(RawRequest(&update[position..end_position]))
                .await?;
            progress.transmit(end_position - position);

            position = end_position;
        }

        self.send_message::<DspUpdateFinish>(Empty).await?;

        Ok(())
    }
}
