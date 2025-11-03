use crate::proto::net::control::{NetworkMessageHeader, UnknownMessage};
use crate::proto::net::{InboundMessage, Response};
use futures::Stream;
use strum::FromRepr;
use tokio::io::AsyncReadExt;

#[derive(Debug)]
pub enum ReportsInboundMessageType {
    Report(ReportMessage),
    Unknown(UnknownMessage),
}

pub async fn listen(
) -> Result<impl Stream<Item = Result<ReportsInboundMessageType, anyhow::Error>>, anyhow::Error> {
    let connection = tokio::net::TcpStream::connect("169.254.2.1:52998").await?;

    let header = [0u8; 6];
    Ok(futures::stream::unfold(
        (connection, header),
        |(mut connection, mut header)| async move {
            let result = (async || -> Result<ReportsInboundMessageType, anyhow::Error> {
                connection.read_exact(&mut header).await?;

                let header = NetworkMessageHeader::from_bytes(&header)?;

                let mut body = vec![0u8; header.length as usize];
                connection.read_exact(&mut body).await?;

                match header.magic {
                    ReportMessage::MAGIC => {
                        let body = ReportMessage::deserialize_from(body)?;
                        Ok(ReportsInboundMessageType::Report(body))
                    }
                    _ => Ok(ReportsInboundMessageType::Unknown(UnknownMessage {
                        magic: header.magic,
                        bytes: body,
                    })),
                }
            })()
            .await;

            Some((result, (connection, header)))
        },
    ))
}

#[derive(FromRepr, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReportType {
    IMU = 0x0000000B,
    Magnometer = 0x00000004,
}

/// A report message containing IMU (Inertial Measurement Unit) data related to the position and
/// orientation of the glasses. Values are expressed in a global coordinate system where:
/// - Positive X points to the left,
/// - Positive Y points upward,
/// - Positive Z points backwards.
/// These directions are defined relative to the glasses, from the perspective of the wearer when
/// the glasses are worn on their head.
///
/// gx, gy, gz, ax, ay, az are only valid when report_type is [ReportType::IMU]
/// mx, my, mz are only valid when report_type is [ReportType::Magnometer]
#[derive(Debug)]
pub struct ReportMessage {
    pub device_id: u64,
    pub hmd_time_nanos_device: u64,
    pub report_type: ReportType,
    pub gx: f32,
    pub gy: f32,
    pub gz: f32,
    pub ax: f32,
    pub ay: f32,
    pub az: f32,
    pub mx: f32,
    pub my: f32,
    pub mz: f32,

    /// Temperature in degrees Celsius
    pub temperature: f32,
    pub imu_id: u8,

    /// An incrementing counter
    pub frame_id: [u8; 3],
}

impl Response for ReportMessage {
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

impl InboundMessage for ReportMessage {
    const MAGIC: [u8; 2] = [0x28, 0x36];
}
