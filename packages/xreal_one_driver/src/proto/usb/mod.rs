use anyhow::{bail, Error};
use bytemuck::{Pod, Zeroable};
use protobuf::Message;
use std::borrow::Cow;
use std::mem::offset_of;

pub mod get_camera_status;
pub mod get_glasses_fw_version;
pub mod get_internal_code;
pub mod get_usb_config_all;
pub mod pilot_update;
pub mod set_usb_config_all;

pub trait UsbTransaction<'req> {
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

impl<T: Message> RequestArgs<'static> for T {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, anyhow::Error> {
        let bytes = self.write_to_bytes()?;
        Ok(Cow::Owned(bytes))
    }
}

pub trait Response: Sized {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, anyhow::Error>;
}

pub struct RawResponse(pub Vec<u8>);

impl Response for RawResponse {
    fn deserialize_from(buffer: &[u8]) -> Result<Self, Error> {
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

pub struct UsbDevice {
    device: hidapi::HidDevice,
}

impl UsbDevice {
    pub fn open(api: &hidapi::HidApi) -> Result<Self, anyhow::Error> {
        let device = api.open(0x3318, 0x0436)?;

        Ok(Self { device })
    }

    pub fn send_mesasge<'req, Txn: UsbTransaction<'req>>(
        &self,
        request: Txn::RequestArgs,
    ) -> Result<Txn::Response, anyhow::Error> {
        let mut data = [0u8; 1024];

        let len = request.serialize_into(&mut data)?;

        let response = self.send_message_raw(Txn::COMMAND_ID, &data[..len], Txn::UNKONWN_VALUES)?;

        let response = Response::deserialize_from(&response.data())?;

        Ok(response)
    }

    fn send_message_raw(
        &self,
        command_tag: [u8; 2],
        data: &[u8],
        unknown: [u8; 5],
    ) -> Result<ControlMessageResponse, anyhow::Error> {
        let mut body = [0u8; 1024];

        const REQUEST_ID: u32 = 0x0;

        {
            let outbound_packet = &mut body[..size_of::<ControlMessageHeader>() + data.len()];

            {
                let header = bytemuck::from_bytes_mut::<ControlMessageHeader>(
                    &mut outbound_packet[..size_of::<ControlMessageHeader>()],
                );
                header.magic = 0xFD;
                header.fields.length =
                    (size_of::<ControlMessageHeaderChecksummed>() + data.len()) as u16;
                header.fields.request_id = REQUEST_ID;
                header.fields.timestamp = 0;
                header.fields.command = command_tag;
                header.fields.unknown = unknown;
            }

            outbound_packet[size_of::<ControlMessageHeader>()..].copy_from_slice(data);

            let checksum =
                crc_adler::crc32(&outbound_packet[offset_of!(ControlMessageHeader, fields)..]);
            let header = bytemuck::from_bytes_mut::<ControlMessageHeader>(
                &mut outbound_packet[..size_of::<ControlMessageHeader>()],
            );
            header.checksum = checksum;

            let bytes_written = self.device.write(&outbound_packet)?;
            if bytes_written != outbound_packet.len() {
                bail!(
                    "failed to write message, only wrote {} bytes, expected {} bytes",
                    bytes_written,
                    outbound_packet.len()
                );
            }
        }

        let bytes_read = self.device.read(&mut body)?;
        if bytes_read < size_of::<ControlMessageHeader>() {
            bail!(
                "failed to read message, only read {} bytes, expected at least {} bytes",
                bytes_read,
                size_of::<ControlMessageHeader>()
            );
        }

        let response_header = *bytemuck::from_bytes::<ControlMessageHeader>(
            &body[..size_of::<ControlMessageHeader>()],
        );

        if response_header.magic != 0xfd {
            bail!("invalid response magic: {}", response_header.magic);
        }

        let expected_checksum = crc_adler::crc32(
            &body[offset_of!(ControlMessageHeader, fields)
                ..(response_header.fields.length as usize
                    + offset_of!(ControlMessageHeader, fields))],
        );
        if expected_checksum != response_header.checksum {
            bail!("invalid response checksum: {}", expected_checksum);
        }

        if response_header.fields.command != command_tag {
            bail!(
                "invalid response command: {:?}, expected: {:?}",
                response_header.fields.command,
                command_tag
            );
        }

        let request_id = response_header.fields.request_id;
        if request_id != REQUEST_ID {
            bail!(
                "invalid response request id: {}, expected: {}",
                request_id,
                0
            );
        }

        let status = body[size_of::<ControlMessageHeader>()];
        if status != 0 {
            bail!("invalid response status: {}", status);
        }

        Ok(ControlMessageResponse { body })
    }
}

struct ControlMessageResponse {
    body: [u8; 1024],
}

impl ControlMessageResponse {
    pub fn header(&self) -> &ControlMessageHeader {
        &bytemuck::from_bytes::<ControlMessageHeader>(
            &self.body[..size_of::<ControlMessageHeader>()],
        )
    }

    pub fn data(&self) -> &[u8] {
        &self.body[(size_of::<ControlMessageHeader>() + 1)
            ..self.header().fields.length as usize + offset_of!(ControlMessageHeader, fields)]
    }
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
