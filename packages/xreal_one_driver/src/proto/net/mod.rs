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
mod key_submit_state;

use crate::proto::net::get_config::GetConfig;
use crate::proto::usb::RequestArgs;
use protobuf::Message;
use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs, io};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::proto::net::key_submit_state::KeySubmitState;

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

        Ok(Self {
            magic,
            length,
        })
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

pub struct NetworkDevice {
    write: tokio::net::tcp::OwnedWriteHalf,
    pending_requests: Arc<Mutex<HashMap<(u32, [u8; 2]), tokio::sync::oneshot::Sender<Vec<u8>>>>>,
}

impl NetworkDevice {
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
                    println!("{:#?}", header);

                    match header.magic {
                        KeySubmitState::MAGIC => {
                            let mut body = vec![0u8; (header.length) as usize];
                            read.read_exact(&mut body).await?;

                            let from = KeySubmitState::deserialize_from(body)?;
                            println!("{:#?}", from);
                        }
                        _ => {
                            let transaction_id = read.read_u32_le().await?;
                            let mut body = vec![0u8; (header.length - 4) as usize];
                            read.read_exact(&mut body).await?;

                            let Some(pending_request) = pending_requests
                                .lock()
                                .unwrap()
                                .remove(&(transaction_id, header.magic))
                            else {
                                // panic!(
                                //     "no pending request for transaction id: {}",
                                //     transaction_id
                                // );
                                continue;
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

#[tokio::test]
async fn test() -> Result<(), anyhow::Error> {
    let (mut device, worker) = NetworkDevice::new().await?;
    let handle = tokio::spawn(worker);

    let response = device
        .send_message::<GetConfig>(protos::get_config::Request {
            ..Default::default()
        })
        .await?;

    fs::write("./calibration.json", &response.value.data)?;

    tokio::time::sleep(Duration::from_secs(10)).await;

    Ok(())
}
