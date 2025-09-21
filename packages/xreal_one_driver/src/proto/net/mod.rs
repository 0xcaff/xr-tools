mod get_calibration_json;
mod protos;

use std::{fs, io};
use std::io::{Read, Write};
use std::net::TcpStream;
use anyhow::{bail, Error};
use bytemuck::{Pod, Zeroable};
use protobuf::Message;
use crate::proto::net::get_calibration_json::{GetCalibrationJson, GetCalibrationJsonRequest};
use crate::proto::usb::{RequestArgs};

trait NetworkTransaction {
    const MAGIC: [u8; 2];
    type RequestArgs: RequestArgs;
    type Response: Response;
}

pub trait Response: Sized {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error>;
}

impl <T> Response for T where T: Message {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, Error> {
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
    connection: TcpStream
}

impl NetworkDevice {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            connection: TcpStream::connect("169.254.2.1:52999")?
        })
    }

    pub fn send_message<T: NetworkTransaction>(&mut self, request: T::RequestArgs) -> Result<T::Response, anyhow::Error> {
        let body = request.as_bytes()?;

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

        if header.transaction_id != tx_id {
            let transaction_id = header.transaction_id;
            bail!("invalid transaction id, got {}, expected: {}", tx_id, transaction_id);
        }

        let mut body = vec![0u8; (header.length - 4) as usize];
        self.connection.read_exact(&mut body)?;

        Ok(T::Response::deserialize_from(body)?)
    }
}

#[test]
fn test() -> Result<(), anyhow::Error> {
    let mut device = NetworkDevice::new()?;
    let response = device.send_message::<GetCalibrationJson>(GetCalibrationJsonRequest)?;
    fs::write("./calibration.json", &response.value.data)?;

    Ok(())
}