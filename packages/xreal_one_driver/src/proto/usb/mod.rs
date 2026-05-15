use anyhow::{anyhow, bail};
use bytemuck::{Pod, Zeroable};
use futures::Future;
use std::borrow::Cow;
use std::collections::HashMap;
use std::mem::offset_of;
use std::sync::{Arc, Mutex};

pub mod dsp_update;
pub mod firmware_header;
pub mod get_camera_status;
pub mod get_glasses_fw_version;
pub mod get_internal_code;
pub mod mcu_update;
pub mod pilot_update;
pub mod recovery;
pub mod usb_config;

pub use usb_config::UsbConfigList;

pub(crate) trait UsbTransaction<'req> {
    const COMMAND_ID: [u8; 2];
    const UNKONWN_VALUES: [u8; 5] = [0u8; 5];

    type RequestArgs: RequestArgs<'req>;
    type Response: Response;
}

pub trait RequestArgs<'a> {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, anyhow::Error>;

    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, anyhow::Error> {
        let bytes = self.as_bytes()?;
        buffer[..bytes.len()].copy_from_slice(&bytes);

        Ok(bytes.len())
    }
}

pub trait Response: Sized {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error>;
}

pub struct RawResponse(pub Vec<u8>);

impl Response for RawResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        Ok(Self(buffer.to_vec()))
    }
}

pub struct Empty;

impl RequestArgs<'static> for Empty {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(&[]))
    }
}

pub trait ConstRequestArgs {
    const VALUE: &'static [u8];
}

// impl <T: ConstRequestArgs> RequestArgs for T {
//     fn as_bytes(&self) -> Result<Cow<[u8]>, anyhow::Error> {
//         Ok(Cow::Borrowed(T::VALUE))
//     }
// }

impl Response for () {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        assert_eq!(buffer.len(), 0);

        Ok(())
    }
}

pub enum XrealOneModelKind {
    XrealOne,
    XrealOnePro,
}

pub struct XrealOneModel<'a> {
    device: &'a hidapi::DeviceInfo,
    kind: XrealOneModelKind,
}

impl XrealOneModel<'_> {
    pub fn detect<'a>(device: &'a hidapi::DeviceInfo) -> Option<XrealOneModel<'a>> {
        if device.vendor_id() != 0x3318 {
            return None;
        }

        let kind = match device.product_id() {
            0x0435 | 0x0436 => XrealOneModelKind::XrealOne,
            0x0437 | 0x0438 => XrealOneModelKind::XrealOnePro,
            _ => return None,
        };

        Some(XrealOneModel { kind, device })
    }
}

pub struct UsbDevice {
    command_tx: tokio::sync::mpsc::Sender<(
        UsbWriteCommand,
        tokio::sync::oneshot::Sender<UsbInboundMessage>,
    )>,
    model: XrealOneModelKind,
}

const USB_COMMAND_QUEUE_CAPACITY: usize = 1;
const USB_EVENT_QUEUE_CAPACITY: usize = 1024;

impl UsbDevice {
    pub fn open(
        api: &hidapi::HidApi,
        device: XrealOneModel<'_>,
    ) -> Result<
        (
            Self,
            impl Future<Output = Result<(), anyhow::Error>> + Send + 'static,
        ),
        anyhow::Error,
    > {
        #[cfg(target_os = "macos")]
        api.set_open_exclusive(false);

        let read_device = device.device.open_device(api)?;
        let write_device = device.device.open_device(api)?;

        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel::<(
            UsbWriteCommand,
            tokio::sync::oneshot::Sender<UsbInboundMessage>,
        )>(USB_COMMAND_QUEUE_CAPACITY);

        type PendingRequests =
            Arc<Mutex<HashMap<(u32, [u8; 2]), tokio::sync::oneshot::Sender<UsbInboundMessage>>>>;

        let pending_requests = PendingRequests::default();
        let task = {
            let pending_requests = pending_requests.clone();

            async move {
                let pending_requests = pending_requests.clone();
                tokio::task::spawn_blocking(|| {

                        let mut body = [0u8; 1024];

                        loop {
                            let bytes_read = read_device.read(&mut body)?;

                            let message = UsbInboundMessage::parse(&body[..bytes_read])?;

                            let mut pending_requests = pending_requests
                                .lock()
                                .map_err(|_| anyhow!("failed to lock pending requests"))?;
                            let exact_key = (message.request_id, message.command);
                            let Some(response_tx) = pending_requests.remove(&exact_key) else {
                                bail!(
                                    "received message with unknown request ID and command: {:?}",
                                    message
                                );
                            };

                            response_tx
                                .send(message)
                                .map_err(|_| anyhow!("failed to send response for message"))?;
                        }
                });

                let mut request_id = 1u32;

                while let Some((command, sender)) = command_rx.recv().await {
                    let current_request_id = request_id;
                    request_id = request_id.wrapping_add(1).max(1);
                    let key = (current_request_id, command.command);

                    if let Some(..) = pending_requests
                        .lock()
                        .map_err(|_| anyhow!("failed to acquire pending requests lock"))?
                        .insert(key, sender)
                    {
                        bail!(
                            "USB request id collision for command {}",
                            format_command(command.command)
                        );
                    };

                    let bytes = command.as_control_bytes(current_request_id)?;

                    let bytes_written = write_device.write(&bytes)?;
                    if bytes_written != bytes.len() {
                        bail!(
                            "failed to write message, only wrote {} bytes, expected {} bytes",
                            bytes_written,
                            bytes.len()
                        );
                    }
                }

                Ok(())
            }
        };

        Ok((
            Self {
                command_tx,
                model: device.kind,
            },
            task,
        ))
    }

    pub(crate) async fn send_message<'req, Txn: UsbTransaction<'req>>(
        &self,
        request: Txn::RequestArgs,
    ) -> Result<Txn::Response, anyhow::Error> {
        let payload = {
            let mut data = [0u8; 1024];
            let len = request.serialize_into(&mut data)?;
            drop(request);
            data[..len].to_vec()
        };

        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        self.command_tx
            .send((
                UsbWriteCommand {
                    command: Txn::COMMAND_ID,
                    unknown: Txn::UNKONWN_VALUES,
                    payload,
                },
                response_tx,
            ))
            .await
            .map_err(|_| anyhow::anyhow!("USB write task stopped"))?;

        let response = response_rx.await?;

        if response.status != 0 {
            bail!("invalid response status: {}", response.status);
        }

        let response = Response::deserialize_from(&response.payload)?;

        Ok(response)
    }
}

fn format_command(command: [u8; 2]) -> String {
    format!("0x{:04x}", u16::from_le_bytes(command))
}

#[derive(Pod, Copy, Clone, Zeroable)]
#[repr(C, packed)]
struct ControlMessageHeader {
    magic: u8,
    checksum: u32,
    fields: ControlMessageHeaderChecksummed,
}

#[derive(Pod, Copy, Clone, Zeroable)]
#[repr(C, packed)]
struct ControlMessageHeaderChecksummed {
    length: u16,
    request_id: u32,
    timestamp: u32,
    command: [u8; 2],
    unknown: [u8; 5],
}

#[derive(Clone, Debug)]
pub struct UsbInboundMessage {
    pub command: [u8; 2],
    pub request_id: u32,
    pub status: u8,
    pub payload: Vec<u8>,
}

impl UsbInboundMessage {
    pub fn parse(body: &[u8]) -> Result<Self, anyhow::Error> {
        if body.len() < size_of::<ControlMessageHeader>() {
            bail!(
                "failed to read message, only read {} bytes, expected at least {} bytes",
                body.len(),
                size_of::<ControlMessageHeader>()
            );
        }

        let response_header = *bytemuck::from_bytes::<ControlMessageHeader>(
            &body[..size_of::<ControlMessageHeader>()],
        );

        if response_header.magic != 0xfd {
            bail!("invalid response magic: {}", response_header.magic);
        }

        let response_checksum = response_header.checksum;
        let response_fields = response_header.fields;
        let response_length = response_fields.length;
        let response_command = response_fields.command;
        let response_request_id = response_fields.request_id;
        let frame_len = response_length as usize + offset_of!(ControlMessageHeader, fields);
        if body.len() < frame_len {
            bail!(
                "failed to read complete message, only read {} bytes, expected {} bytes",
                body.len(),
                frame_len
            );
        }

        let expected_checksum =
            crc_adler::crc32(&body[offset_of!(ControlMessageHeader, fields)..frame_len]);
        if expected_checksum != response_checksum {
            bail!("invalid response checksum: {}", expected_checksum);
        }

        if (response_length as usize)
            < size_of::<ControlMessageHeaderChecksummed>() + size_of::<u8>()
        {
            bail!(
                "invalid response length {}, expected at least {}",
                response_length,
                size_of::<ControlMessageHeaderChecksummed>() + size_of::<u8>()
            );
        }

        let status_offset = size_of::<ControlMessageHeader>();
        let status = body[status_offset];
        let payload = body[(status_offset + 1)..frame_len].to_vec();

        Ok(UsbInboundMessage {
            command: response_command,
            request_id: response_request_id,
            status,
            payload,
        })
    }
}

struct UsbWriteCommand {
    command: [u8; 2],
    unknown: [u8; 5],
    payload: Vec<u8>,
}

impl UsbWriteCommand {
    pub fn as_control_bytes(&self, current_request_id: u32) -> Result<[u8; 1024], anyhow::Error> {
        let command = self.command;
        let data = &self.payload;
        let unknown = self.unknown;
        let mut body = [0u8; 1024];
        let packet_len = size_of::<ControlMessageHeader>() + data.len();
        if packet_len > body.len() {
            bail!(
                "USB payload for command {} is too large: {} bytes",
                format_command(command),
                data.len()
            );
        }

        let outbound_packet = &mut body[..packet_len];

        {
            let header = bytemuck::from_bytes_mut::<ControlMessageHeader>(
                &mut outbound_packet[..size_of::<ControlMessageHeader>()],
            );
            header.magic = 0xFD;
            header.fields.length =
                (size_of::<ControlMessageHeaderChecksummed>() + data.len()) as u16;
            header.fields.request_id = current_request_id;
            header.fields.timestamp = 0;
            header.fields.command = command;
            header.fields.unknown = unknown;
        }

        outbound_packet[size_of::<ControlMessageHeader>()..].copy_from_slice(data);

        let checksum =
            crc_adler::crc32(&outbound_packet[offset_of!(ControlMessageHeader, fields)..]);
        let header = bytemuck::from_bytes_mut::<ControlMessageHeader>(
            &mut outbound_packet[..size_of::<ControlMessageHeader>()],
        );
        header.checksum = checksum;

        Ok(body)
    }
}
