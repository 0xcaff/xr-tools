pub mod display_stop_osd_render;
pub mod dp_get_current_edid_dsp;
pub mod dp_set_current_edid_dsp;
pub mod dp_set_input_mode;
pub mod get_config;
pub mod glasses_get_dsp_version;
pub mod glasses_get_id;
pub mod glasses_get_sw_version;
pub mod key_submit_state;
mod props;
mod reports;
pub mod set_display_brightness;
pub mod set_electrochromic_dimmer;

use crate::proto::net::key_submit_state::KeyStateChangeMessage;
use crate::proto::net::props::{SetNumericProperty, SetPropertyRequest};
use crate::proto::net::set_display_brightness::{DisplayBrightness, SetDisplayBrightness};
use crate::proto::usb::RequestArgs;
use anyhow::Error;
use futures::{pin_mut, Stream, StreamExt, TryStream, TryStreamExt};
use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::pin::pin;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};

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

trait InboundMessage: Response {
    const MAGIC: [u8; 2];
}

pub trait Response: Sized {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error>;
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

#[derive(Debug)]
pub enum InboundMessageType {
    KeyStateChange(KeyStateChangeMessage),
    Unknown(UnknownMessage),
}

#[derive(Debug)]
pub struct UnknownMessage {
    pub magic: [u8; 2],
    pub bytes: Vec<u8>,
}

impl ControlNetworkDevice {
    pub async fn new() -> Result<
        (
            Self,
            impl Stream<Item = Result<InboundMessageType, anyhow::Error>>,
        ),
        anyhow::Error,
    > {
        let connection = tokio::net::TcpStream::connect("169.254.2.1:52999").await?;
        let (read, write) = connection.into_split();
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));

        let header = [0u8; 6];

        Ok((
            Self {
                write,
                pending_requests: pending_requests.clone(),
            },
            futures::stream::unfold((header, read), move |(header, mut read)| {
                let pending_requests = pending_requests.clone();

                async move {
                    let result = (async || -> Result<InboundMessageType, anyhow::Error> {
                        loop {
                            let header = NetworkMessageHeader::from_bytes(&header)?;

                            match header.magic {
                                KeyStateChangeMessage::MAGIC => {
                                    let mut body = vec![0u8; (header.length) as usize];
                                    read.read_exact(&mut body).await?;

                                    let from = KeyStateChangeMessage::deserialize_from(body)?;
                                    return Ok(InboundMessageType::KeyStateChange(from));
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
                                        let mut body_new = transaction_id.to_be_bytes().to_vec();
                                        body_new.append(&mut body);

                                        return Ok(InboundMessageType::Unknown(UnknownMessage {
                                            bytes: body_new,
                                            magic: header.magic,
                                        }));
                                    };

                                    let _ = pending_request.send(body);
                                }
                            }
                        }
                    })()
                    .await;

                    Some((result, (header, read)))
                }
            }),
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
    let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;

    tokio::spawn(async move {
        pin_mut!(inbound_messages);

        while let Some(message) = inbound_messages.try_next().await.unwrap() {
            println!("{:#?}", message);
        }
    });

    let response = device
        .send_message::<SetDisplayBrightness>(SetPropertyRequest {
            value: SetNumericProperty(DisplayBrightness(1)),
        })
        .await?;

    println!("{:#?}", response);

    Ok(())
}
