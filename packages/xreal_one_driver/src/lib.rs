mod proto;

use crate::proto::usb::get_camera_status::GetCameraStatusTransaction;
use crate::proto::usb::get_glasses_fw_version::GetGlassesFwVersionTransaction;
use crate::proto::usb::get_usb_config_all::{GetUsbConfigAll, UsbConfigList};
use crate::proto::usb::{RequestArgs, Response, UsbTransaction};
use anyhow::bail;
use bytemuck::{Pod, Zeroable};
use std::mem;
use std::mem::offset_of;
use crate::proto::usb::set_usb_config_all::{SetUsbConfigAll, SetUsbConfigAllRequest};

struct XREALOneDevice {
    device: hidapi::HidDevice,
}

impl XREALOneDevice {
    pub fn open(api: &hidapi::HidApi) -> Result<Self, anyhow::Error> {
        let device = api.open(0x3318, 0x0436)?;

        Ok(Self { device })
    }

    pub fn send_mesasge<Txn: UsbTransaction>(
        &self,
        request: Txn::RequestArgs,
    ) -> Result<Txn::Response, anyhow::Error> {
        let mut data = [0u8; 1024];

        let len = request.serialize_into(&mut data)?;

        let response = self.send_message_raw(Txn::COMMAND_ID, &data[..len])?;

        let response = Response::deserialize_from(&response.data())?;

        Ok(response)
    }

    fn send_message_raw(
        &self,
        command_tag: u8,
        data: &[u8],
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
            }

            outbound_packet[size_of::<ControlMessageHeader>()..].copy_from_slice(data);

            let checksum =
                crc_adler::crc32(&outbound_packet[offset_of!(ControlMessageHeader, fields)..]);
            let header = bytemuck::from_bytes_mut::<ControlMessageHeader>(
                &mut outbound_packet[..size_of::<ControlMessageHeader>()],
            );
            header.checksum = checksum;

            println!("{:x?}", outbound_packet);

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
                "invalid response command: {}, expected: {}",
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

        println!("{:x?}", body);

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
        &self.body[size_of::<ControlMessageHeader>()
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
    command: u8,
    unknown_1: u32,
    unknown_2: u16,
}

#[test]
fn test() -> Result<(), anyhow::Error> {
    let api = hidapi::HidApi::new()?;
    let device = XREALOneDevice::open(&api)?;
    // device.send_message_raw(0xD3, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x01, 0x00])?;
    // let response = device.send_message_raw(0xD5, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x0])?;
    // println!("{:?}", response.data());
    let response = device.send_mesasge::<SetUsbConfigAll>(SetUsbConfigAllRequest {
        config: UsbConfigList::new()
            .with_uvc0(1)
            .with_mtp(1)
            .with_enable(1)
    })?;
    println!("{:?}", response);

    Ok(())
}
