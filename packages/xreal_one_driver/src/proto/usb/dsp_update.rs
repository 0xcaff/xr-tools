use crate::proto::net::RawRequest;
use crate::proto::usb::firmware_header::FirmwareHeader;
use crate::proto::usb::{Empty, UsbDevice, UsbInboundMessage, UsbTransaction};
use anyhow::bail;

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
    fn device_progress(&mut self, _phase: DspUpdatePhase, _status_ok: bool) {}
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

async fn wait_phase(
    events: &mut tokio::sync::broadcast::Receiver<UsbInboundMessage>,
    phase: DspUpdatePhase,
    progress: &mut impl DspUpdateProgressReporter,
) -> Result<(), anyhow::Error> {
    let (progress_command, finish_command) = phase.commands();

    loop {
        let message = match events.recv().await {
            Ok(message) => message,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                bail!("USB event stream closed while waiting for DSP {phase:?}")
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => bail!(
                "USB event stream lagged by {skipped} messages while waiting for DSP {phase:?}"
            ),
        };

        if message.command == progress_command {
            if !message.payload.is_empty() {
                bail!(
                    "DSP {phase:?} progress event had {} payload bytes",
                    message.payload.len()
                );
            }

            progress.device_progress(phase, message.status == 0x00);
        } else if message.command == finish_command {
            if !message.payload.is_empty() {
                bail!(
                    "DSP {phase:?} finish event had {} payload bytes",
                    message.payload.len()
                );
            }
            if message.status != 0 {
                bail!(
                    "DSP {phase:?} finish event failed with status {}",
                    message.status
                );
            }

            return Ok(());
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

        self.endpoint.send_message::<DspUpdateStart>(header).await?;
        progress.transmit(DspFirmwareHeader::LEN);

        let mut position = DspFirmwareHeader::LEN;
        while position < update.len() {
            let end_position = std::cmp::min(position + 1002, update.len());
            self.endpoint
                .send_message::<DspUpdateTransmit>(RawRequest(&update[position..end_position]))
                .await?;
            progress.transmit(end_position - position);

            position = end_position;
        }

        let mut events = self.endpoint.subscribe();
        self.endpoint.send_message::<DspUpdateFinish>(Empty).await?;

        wait_phase(&mut events, DspUpdatePhase::WriteFlash, progress).await?;
        wait_phase(&mut events, DspUpdatePhase::Boot, progress).await?;
        wait_phase(&mut events, DspUpdatePhase::ReadFlash, progress).await?;

        Ok(())
    }
}
