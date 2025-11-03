use crate::config::Config;
pub use crate::proto::net::display_set_scene_mode::SceneMode;
use crate::proto::net::display_set_scene_mode::SetSceneMode;
pub use crate::proto::net::dp_get_current_edid_dsp::DisplayConfiguration;
use crate::proto::net::dp_set_current_edid_dsp::DpSetCurrentEdidDsp;
use crate::proto::net::dp_set_input_mode::DpSetInputMode;
pub use crate::proto::net::dp_set_input_mode::InputMode;
use crate::proto::net::get_config::GetConfig;
use crate::proto::net::glasses_get_dsp_version::GlassesGetDspVersion;
use crate::proto::net::glasses_get_id::GlassesGetId;
use crate::proto::net::glasses_get_sw_version::GlassesGetFwVersion;
pub use crate::proto::net::key_submit_state::KeyStateChangeMessage;
use crate::proto::net::props::{GetPropertyRequest, SetNumericProperty, SetPropertyRequest};
pub use crate::proto::net::set_display_brightness::DisplayBrightness;
use crate::proto::net::set_display_brightness::SetDisplayBrightness;
pub use crate::proto::net::set_elechromic_dimmer::ElectricDimmerLevel;
use crate::proto::net::set_elechromic_dimmer::SetElechromicDimmer;
use crate::proto::net::{InboundMessage, NetworkTransaction, Response};
use crate::proto::usb::RequestArgs;
use futures::Stream;
use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct ControlNetworkDevice {
    write: tokio::net::tcp::OwnedWriteHalf,
    pending_requests: Arc<Mutex<HashMap<(u32, [u8; 2]), tokio::sync::oneshot::Sender<Vec<u8>>>>>,
}

#[derive(Debug)]
pub enum ControlInboundMessageType {
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
            impl Stream<Item = Result<ControlInboundMessageType, anyhow::Error>>,
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
            futures::stream::unfold((header, read), move |(mut header, mut read)| {
                let pending_requests = pending_requests.clone();

                async move {
                    let result = (async || -> Result<ControlInboundMessageType, anyhow::Error> {
                        loop {
                            read.read_exact(&mut header).await?;
                            let header = NetworkMessageHeader::from_bytes(&header)?;

                            match header.magic {
                                KeyStateChangeMessage::MAGIC => {
                                    let mut body = vec![0u8; header.length as usize];
                                    read.read_exact(&mut body).await?;

                                    let from = KeyStateChangeMessage::deserialize_from(body)?;
                                    return Ok(ControlInboundMessageType::KeyStateChange(from));
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

                                        return Ok(ControlInboundMessageType::Unknown(
                                            UnknownMessage {
                                                bytes: body_new,
                                                magic: header.magic,
                                            },
                                        ));
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

    pub(crate) async fn send_message<'a, T: NetworkTransaction<'a>>(
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

#[derive(Debug)]
pub(crate) struct NetworkMessageHeader {
    pub magic: [u8; 2],
    pub length: u32,
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

impl ControlNetworkDevice {
    pub async fn set_scene_mode(&mut self, scene_mode: SceneMode) -> Result<(), anyhow::Error> {
        self.send_message::<SetSceneMode>(SetPropertyRequest {
            value: SetNumericProperty(scene_mode),
        })
        .await?;

        Ok(())
    }

    pub async fn set_input_mode(&mut self, input_mode: InputMode) -> Result<(), anyhow::Error> {
        self.send_message::<DpSetInputMode>(SetPropertyRequest {
            value: SetNumericProperty(input_mode),
        })
        .await?;

        Ok(())
    }

    /// [display_brightness]: A value between 0 and 9
    pub async fn set_display_brightness(
        &mut self,
        display_brightness: u8,
    ) -> Result<(), anyhow::Error> {
        self.send_message::<SetDisplayBrightness>(SetPropertyRequest {
            value: SetNumericProperty(DisplayBrightness(display_brightness)),
        })
        .await?;

        Ok(())
    }

    pub async fn set_elechromic_dimmer(
        &mut self,
        dimmer: ElectricDimmerLevel,
    ) -> Result<(), anyhow::Error> {
        self.send_message::<SetElechromicDimmer>(SetPropertyRequest {
            value: SetNumericProperty(dimmer),
        })
        .await?;

        Ok(())
    }

    // async fn get_display_configuration(&mut self) -> Result<DisplayConfiguration, anyhow::Error> {
    //     Ok(self
    //         .send_message::<DpGetCurrentEdidDsp>(GetPropertyRequest)
    //         .await?
    //         .value
    //         .0)
    // }

    pub async fn set_display_configuration(
        &mut self,
        display_configuration: DisplayConfiguration,
    ) -> Result<(), anyhow::Error> {
        self.send_message::<DpSetCurrentEdidDsp>(SetPropertyRequest {
            value: SetNumericProperty(display_configuration),
        })
        .await?;

        Ok(())
    }

    pub async fn get_dsp_version(&mut self) -> Result<String, anyhow::Error> {
        Ok(self
            .send_message::<GlassesGetDspVersion>(GetPropertyRequest)
            .await?
            .value)
    }

    pub async fn get_sw_version(&mut self) -> Result<String, anyhow::Error> {
        Ok(self
            .send_message::<GlassesGetFwVersion>(GetPropertyRequest)
            .await?
            .value)
    }

    pub async fn get_id(&mut self) -> Result<String, anyhow::Error> {
        Ok(self
            .send_message::<GlassesGetId>(GetPropertyRequest)
            .await?
            .value)
    }

    pub async fn get_config_raw(&mut self) -> Result<String, anyhow::Error> {
        Ok(self
            .send_message::<GetConfig>(GetPropertyRequest)
            .await?
            .value)
    }

    pub async fn get_config(&mut self) -> Result<Config, anyhow::Error> {
        Ok(Config::parse(self.get_config_raw().await?.as_bytes())?)
    }
}
