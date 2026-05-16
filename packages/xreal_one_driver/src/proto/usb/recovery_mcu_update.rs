use crate::proto::net::RawRequest;
use crate::proto::usb::firmware_header::FirmwareHeader;
use crate::proto::usb::mcu_update::McuUpdateProgressReporter;
use crate::proto::usb::{Empty, RecoveryUsbDevice, RequestArgs, UsbInboundMessage, UsbTransaction};
use anyhow::bail;
use std::borrow::Cow;

pub type McuRecoveryFirmwareHeader = FirmwareHeader<1>;

pub struct McuRecoveryUpdateArm;

pub struct McuRecoveryUpdateArmRequest;

impl RequestArgs<'static> for McuRecoveryUpdateArmRequest {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(&[0x01]))
    }
}

impl UsbTransaction<'static> for McuRecoveryUpdateArm {
    const COMMAND_ID: [u8; 2] = [0x12, 0x12];
    type RequestArgs = McuRecoveryUpdateArmRequest;
    type Response = ();
}

pub struct McuRecoveryUpdateStart;

impl<'req> UsbTransaction<'req> for McuRecoveryUpdateStart {
    const COMMAND_ID: [u8; 2] = [0x3f, 0x00];
    type RequestArgs = &'req McuRecoveryFirmwareHeader;
    type Response = ();
}

pub struct McuRecoveryUpdateTransmit;

impl<'req> UsbTransaction<'req> for McuRecoveryUpdateTransmit {
    const COMMAND_ID: [u8; 2] = [0x40, 0x00];
    type RequestArgs = RawRequest<'req>;
    type Response = ();
}

pub struct McuRecoveryUpdateFinish;

impl UsbTransaction<'static> for McuRecoveryUpdateFinish {
    const COMMAND_ID: [u8; 2] = [0x41, 0x00];
    type RequestArgs = Empty;
    type Response = ();
}

async fn wait_recovery_mcu_finish(
    events: &mut tokio::sync::broadcast::Receiver<UsbInboundMessage>,
    progress: &mut impl McuUpdateProgressReporter,
) -> Result<(), anyhow::Error> {
    loop {
        let message = match events.recv().await {
            Ok(message) => message,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                bail!("USB event stream closed while waiting for recovery MCU update")
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => bail!(
                "USB event stream lagged by {skipped} messages while waiting for recovery MCU update"
            ),
        };

        if message.command == [0x16, 0x6c] {
            if !message.payload.is_empty() {
                bail!(
                    "recovery MCU progress event had {} payload bytes",
                    message.payload.len()
                );
            }
            if message.status > 100 {
                bail!(
                    "recovery MCU progress value out of range: {}",
                    message.status
                );
            }

            progress.device_progress(message.status);
        } else if message.command == [0x17, 0x6c] {
            if !message.payload.is_empty() {
                bail!(
                    "recovery MCU done event had {} payload bytes",
                    message.payload.len()
                );
            }

            let status_ok = message.status == 0x00;
            progress.device_finished(status_ok);
            if !status_ok {
                bail!(
                    "recovery MCU done event failed with status {}",
                    message.status
                );
            }

            return Ok(());
        }
    }
}

impl RecoveryUsbDevice {
    pub async fn update_recovery_mcu(&self, update: &[u8]) -> Result<(), anyhow::Error> {
        struct EmptyReporter;

        impl McuUpdateProgressReporter for EmptyReporter {}

        self.update_recovery_mcu_with_progress(update, &mut EmptyReporter)
            .await
    }

    pub async fn update_recovery_mcu_with_progress(
        &self,
        update: &[u8],
        progress: &mut impl McuUpdateProgressReporter,
    ) -> Result<(), anyhow::Error> {
        let header = McuRecoveryFirmwareHeader::load(update)?;

        self.endpoint
            .send_message::<McuRecoveryUpdateArm>(McuRecoveryUpdateArmRequest)
            .await?;

        self.endpoint
            .send_message::<McuRecoveryUpdateStart>(header)
            .await?;
        progress.transmit(McuRecoveryFirmwareHeader::LEN);

        let mut position = McuRecoveryFirmwareHeader::LEN;
        while position < update.len() {
            let end_position = std::cmp::min(position + 1002, update.len());
            self.endpoint
                .send_message::<McuRecoveryUpdateTransmit>(RawRequest(
                    &update[position..end_position],
                ))
                .await?;
            progress.transmit(end_position - position);

            position = end_position;
        }

        let mut events = self.endpoint.subscribe();
        self.endpoint
            .send_message::<McuRecoveryUpdateFinish>(Empty)
            .await?;
        wait_recovery_mcu_finish(&mut events, progress).await?;

        Ok(())
    }
}
