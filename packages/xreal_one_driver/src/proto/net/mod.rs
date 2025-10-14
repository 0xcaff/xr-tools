pub mod display_stop_osd_render;
pub mod dp_get_current_edid_dsp;
pub mod dp_set_current_edid_dsp;
pub mod dp_set_input_mode;
pub mod get_config;
pub mod glasses_get_dsp_version;
pub mod glasses_get_id;
pub mod glasses_get_sw_version;
pub mod key_submit_state;
pub mod protos;
pub mod set_display_brightness;
pub mod set_electrochromic_dimmer;

use crate::proto::net::key_submit_state::KeySubmitState;
use crate::proto::usb::RequestArgs;
use protobuf::Message;
use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::io;
use strum::FromRepr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

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

pub trait NetworkTransaction<'request> {
    const MAGIC: [u8; 2];
    type RequestArgs: RequestArgs<'request>;
    type Response: Response;
}

trait InboundRequest: Response {
    const MAGIC: [u8; 2];
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

#[derive(Debug)]
struct NetworkMessageHeader {
    magic: [u8; 2],
    length: u32,
}

impl NetworkMessageHeader {
    pub fn from_bytes(buffer: &[u8]) -> Result<Self, anyhow::Error> {
        let mut magic = [0u8; 2];
        magic.copy_from_slice(&buffer[0..2]);
        let length = u32::from_be_bytes([buffer[2], buffer[3], buffer[4], buffer[5]]);

        Ok(Self { magic, length })
    }

    pub async fn write(&self, mut writer: impl AsyncWrite + Unpin) -> Result<(), io::Error> {
        writer.write_all(&self.magic).await?;
        writer.write_all(&self.length.to_be_bytes()).await?;

        Ok(())
    }
}

#[derive(Debug)]
struct NetworkTransactionMessageHeader {
    header: NetworkMessageHeader,
    transaction_id: u32,
}

impl NetworkTransactionMessageHeader {
    pub async fn write(&self, mut writer: impl AsyncWrite + Unpin) -> Result<(), io::Error> {
        self.header.write(&mut writer).await?;
        writer.write_all(&self.transaction_id.to_be_bytes()).await?;

        Ok(())
    }
}

pub struct ControlNetworkDevice {
    write: tokio::net::tcp::OwnedWriteHalf,
    pending_requests: Arc<Mutex<HashMap<(u32, [u8; 2]), tokio::sync::oneshot::Sender<Vec<u8>>>>>,
}

impl ControlNetworkDevice {
    pub async fn new(
    ) -> Result<(Self, impl Future<Output = Result<(), anyhow::Error>>), anyhow::Error> {
        let connection = tokio::net::TcpStream::connect("169.254.2.1:52999").await?;
        let (mut read, write) = connection.into_split();
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));

        Ok((
            Self {
                write,
                pending_requests: pending_requests.clone(),
            },
            async move {
                let mut header = [0u8; 6];

                loop {
                    read.read_exact(&mut header).await?;

                    let header = NetworkMessageHeader::from_bytes(&header)?;

                    match header.magic {
                        KeySubmitState::MAGIC => {
                            let mut body = vec![0u8; (header.length) as usize];
                            read.read_exact(&mut body).await?;

                            let from = KeySubmitState::deserialize_from(body)?;
                            println!("{:#?}", from);
                        }
                        _ => {
                            let transaction_id = read.read_u32().await?;
                            let mut body = vec![0u8; (header.length - 4) as usize];
                            read.read_exact(&mut body).await?;

                            let Some(pending_request) = pending_requests
                                .lock()
                                .unwrap()
                                .remove(&(transaction_id, header.magic))
                            else {
                                panic!(
                                    "no pending request for transaction id: {:x}",
                                    transaction_id
                                );
                            };

                            let _ = pending_request.send(body);
                        }
                    }
                }
            },
        ))
    }

    pub async fn send_message<'a, T: NetworkTransaction<'a>>(
        &mut self,
        request: T::RequestArgs,
    ) -> Result<T::Response, anyhow::Error> {
        let body = request.as_bytes()?;

        let tx_id = 1;

        let header = NetworkTransactionMessageHeader {
            header: NetworkMessageHeader {
                magic: T::MAGIC,
                length: body.len() as u32 + 4,
            },
            transaction_id: tx_id | 0x80000000,
        };

        let (tx, rx) = tokio::sync::oneshot::channel();

        self.pending_requests
            .lock()
            .unwrap()
            .insert((tx_id, T::MAGIC), tx);

        header.write(&mut self.write).await?;
        self.write.write_all(&body).await?;

        let body = rx.await?;

        Ok(T::Response::deserialize_from(body)?)
    }
}

pub struct ReportsNetworkDevice {
    connection: tokio::net::TcpStream,
}

impl ReportsNetworkDevice {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let mut connection = tokio::net::TcpStream::connect("169.254.2.1:52998").await?;

        loop {
            let mut header = [0u8; 6];

            connection.read_exact(&mut header).await?;

            let header = NetworkMessageHeader::from_bytes(&header)?;

            let mut body = vec![0u8; header.length as usize];
            connection.read_exact(&mut body).await?;

            match header.magic {
                ReportPacket::MAGIC => {
                    let body = ReportPacket::deserialize_from(body)?;
                    println!("{:#?}", body);
                }
                _ => {
                    panic!("{:?}", header.magic);
                    continue;
                }
            }
        }
    }
}

#[derive(FromRepr, Debug)]
#[repr(u32)]
enum ReportType {
    IMU = 0x0000000B,
    Magnometer = 0x00000004,
}

#[derive(Debug)]
struct ReportPacket {
    device_id: u64,
    hmd_time_nanos_device: u64,
    report_type: ReportType,
    gx: f32,
    gy: f32,
    gz: f32,
    ax: f32,
    ay: f32,
    az: f32,
    mx: f32,
    my: f32,
    mz: f32,
    temperature: f32,
    imu_id: u8,
    frame_id: [u8; 3],
}

impl Response for ReportPacket {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error> {
        assert_eq!(buffer.len(), 128);

        let device_id = u64::from_le_bytes(buffer[0..0x8].try_into()?);
        let hmd_time_nanos_device = u64::from_le_bytes(buffer[0x8..0x10].try_into()?);
        let report_type = u32::from_le_bytes(buffer[0x18..0x1c].try_into()?);
        let report_type = ReportType::from_repr(report_type)
            .ok_or_else(|| anyhow::anyhow!("unknown report type: {:x}", report_type))?;

        let gx = f32::from_le_bytes(buffer[0x1c..0x20].try_into()?);
        let gy = f32::from_le_bytes(buffer[0x20..0x24].try_into()?);
        let gz = f32::from_le_bytes(buffer[0x24..0x28].try_into()?);
        let ax = f32::from_le_bytes(buffer[0x28..0x2c].try_into()?);
        let ay = f32::from_le_bytes(buffer[0x2c..0x30].try_into()?);
        let az = f32::from_le_bytes(buffer[0x30..0x34].try_into()?);
        let mx = f32::from_le_bytes(buffer[0x34..0x38].try_into()?);
        let my = f32::from_le_bytes(buffer[0x38..0x3c].try_into()?);
        let mz = f32::from_le_bytes(buffer[0x3c..0x40].try_into()?);
        let temperature = f32::from_le_bytes(buffer[0x40..0x44].try_into()?);

        let imu_id = buffer[0x44];
        let frame_id = buffer[0x45..0x48].try_into()?;

        Ok(Self {
            device_id,
            hmd_time_nanos_device,
            report_type,
            gx,
            gy,
            gz,
            ax,
            ay,
            az,
            mx,
            my,
            mz,
            temperature,
            imu_id,
            frame_id,
        })
    }
}

impl InboundRequest for ReportPacket {
    const MAGIC: [u8; 2] = [0x28, 0x36];
}

#[tokio::test]
async fn test() -> Result<(), anyhow::Error> {
    ReportsNetworkDevice::new().await?;

    Ok(())
}
