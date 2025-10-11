mod display_stop_osd_render;
mod dp_get_current_edid_dsp;
mod dp_set_current_edid_dsp;
mod dp_set_input_mode;
mod get_config;
mod glasses_get_dsp_version;
mod glasses_get_id;
mod glasses_get_sw_version;
mod protos;
mod set_display_brightness;
mod set_electrochromic_dimmer;

use crate::proto::net::get_config::GetConfig;
use crate::proto::usb::RequestArgs;
use protobuf::Message;
use std::borrow::Cow;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::{fs, io};

#[derive(Debug)]
pub struct RawResponse(pub Vec<u8>);

impl Response for RawResponse {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error> {
        Ok(Self(buffer))
    }
}

pub struct RawRequest<'a>(pub &'a [u8]);

impl<'a> RequestArgs<'a> for RawRequest<'a> {
    fn as_bytes(&self) -> Result<Cow<'a, [u8]>, anyhow::Error> {
        Ok(Cow::Borrowed(self.0))
    }
}

trait NetworkTransaction<'request> {
    const MAGIC: [u8; 2];
    type RequestArgs: RequestArgs<'request>;
    type Response: Response;
}

pub trait Response: Sized {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error>;
}

impl<T> Response for T
where
    T: Message,
{
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error> {
        Ok(T::parse_from_bytes(&buffer)?)
    }
}

struct NetworkMessageHeader {
    magic: [u8; 2],
    length: u32,
    transaction_id: u32,
}

impl NetworkMessageHeader {
    pub fn write(&self, mut writer: impl Write) -> Result<(), io::Error> {
        writer.write_all(&self.magic)?;
        writer.write_all(&self.length.to_be_bytes())?;
        writer.write_all(&self.transaction_id.to_be_bytes())?;

        Ok(())
    }

    pub fn from_bytes(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        let mut magic = [0u8; 2];
        magic.copy_from_slice(&buffer[0..2]);
        let length = u32::from_be_bytes([buffer[2], buffer[3], buffer[4], buffer[5]]);
        let transaction_id = u32::from_be_bytes([buffer[6], buffer[7], buffer[8], buffer[9]]);

        Ok(Self {
            magic,
            length,
            transaction_id,
        })
    }
}

struct NetworkDevice {
    connection: TcpStream,
}

impl NetworkDevice {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            connection: TcpStream::connect("169.254.2.1:52999")?,
        })
    }

    pub fn send_message<'a, T: NetworkTransaction<'a>>(
        &mut self,
        request: T::RequestArgs,
    ) -> Result<T::Response, anyhow::Error> {
        let body = request.as_bytes()?;
        println!("{:#x?}", body);

        let tx_id = 1;

        let header = NetworkMessageHeader {
            magic: T::MAGIC.clone(),
            length: body.len() as u32 + 4,
            transaction_id: tx_id | 0x80000000,
        };

        header.write(&mut self.connection)?;
        self.connection.write_all(&body)?;

        let mut header = [0u8; 10];
        self.connection.read_exact(&mut header)?;

        let header = NetworkMessageHeader::from_bytes(&header)?;

        // if header.transaction_id != tx_id {
        //     let transaction_id = header.transaction_id;
        //     bail!("invalid transaction id, got {}, expected: {}", tx_id, transaction_id);
        // }

        let mut body = vec![0u8; (header.length - 4) as usize];
        self.connection.read_exact(&mut body)?;

        Ok(T::Response::deserialize_from(body)?)
    }
}

#[test]
fn test() -> Result<(), anyhow::Error> {
    let mut device = NetworkDevice::new()?;

    let response = device.send_message::<GetConfig>(protos::get_config::Request {
        ..Default::default()
    })?;

    fs::write("./calibration.json", &response.value.data)?;

    Ok(())
}
