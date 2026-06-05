use crate::{
    ImuFrequency, ImuMode, ImuPose, RequestArgs, Response, SetImuFrequency, SetImuMode,
    UsbTransaction,
};
use anyhow::{anyhow, bail, Context, Result};
use futures::channel::mpsc;
use futures::{stream, Stream};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::pin::Pin;
use std::thread;

#[derive(Clone, Debug)]
struct VitureInboundMessage {
    command: [u8; 2],
    timestamp_ms: u32,
    payload: Vec<u8>,
}

pub struct UsbEndpoint {
    control_device: HidDevice,
    imu_device: HidDevice,
}

impl UsbEndpoint {
    pub fn open_device_info(
        api: &HidApi,
        control_device: &DeviceInfo,
        imu_device: &DeviceInfo,
    ) -> Result<Self, anyhow::Error> {
        let control_device = control_device
            .open_device(api)
            .context("failed to open VITURE control HID endpoint")?;
        let imu_device = imu_device
            .open_device(api)
            .context("failed to open VITURE IMU HID endpoint")?;

        Ok(Self {
            control_device,
            imu_device,
        })
    }

    pub fn start_imu(self) -> Result<impl Stream<Item = ImuPose> + Send + 'static, anyhow::Error> {
        send_message::<SetImuFrequency>(&self.control_device, ImuFrequency::High)?;
        send_message::<SetImuMode>(&self.control_device, ImuMode::Pose)?;

        let stop_imu = scopeguard::guard(self.control_device, |control_device| {
            let _ = send_message::<SetImuMode>(&control_device, ImuMode::Off);
        });
        let (tx, mut rx) = mpsc::unbounded();
        thread::spawn(move || read_pose_loop(self.imu_device, tx));

        Ok(stream::poll_fn(move |cx| {
            let _stop_imu = &stop_imu;
            Pin::new(&mut rx).poll_next(cx)
        }))
    }
}

fn send_message<'req, Txn: UsbTransaction<'req>>(
    device: &HidDevice,
    request: Txn::RequestArgs,
) -> Result<Txn::Response, anyhow::Error> {
    let mut payload = [0u8; 64];
    let payload_len = request.serialize_into(&mut payload)?;
    let bytes = viture_control_bytes(Txn::COMMAND_ID, &payload[..payload_len])?;

    let written = device.write(&bytes).with_context(|| {
        format!(
            "failed to write VITURE command {}",
            format_command(Txn::COMMAND_ID)
        )
    })?;

    if written != bytes.len() && written != bytes.len() - 1 {
        bail!(
            "short HID write for VITURE command {}: wrote {written} of {} bytes",
            format_command(Txn::COMMAND_ID),
            bytes.len()
        );
    }

    Txn::Response::deserialize_from(&[])
}

fn format_command(command: [u8; 2]) -> String {
    format!("0x{:04x}", u16::from_le_bytes(command))
}

fn viture_control_bytes(command: [u8; 2], payload: &[u8]) -> Result<[u8; 65], anyhow::Error> {
    let data_len = 0x0cusize
        .checked_add(payload.len())
        .ok_or_else(|| anyhow!("VITURE command payload is too large"))?;
    if data_len > 64 - 6 {
        bail!(
            "VITURE command {} payload is too large: {} bytes",
            format_command(command),
            payload.len()
        );
    }

    let mut body = [0u8; 65];
    let packet = &mut body[1..];
    packet[0..2].copy_from_slice(&[0xff, 0xfe]);
    packet[4..6].copy_from_slice(&(data_len as u16).to_le_bytes());
    packet[14..16].copy_from_slice(&command);
    packet[18..18 + payload.len()].copy_from_slice(payload);

    let crc_end = 4 + data_len + 2;
    let crc = crc::Crc::<u16>::new(&crc::CRC_16_XMODEM).checksum(&packet[4..crc_end]);
    packet[2..4].copy_from_slice(&crc.to_le_bytes());

    Ok(body)
}

fn read_pose_loop(imu_device: HidDevice, tx: mpsc::UnboundedSender<ImuPose>) {
    let mut packet = [0u8; 65];
    while !tx.is_closed() {
        match imu_device.read(&mut packet) {
            Ok(0) => {}
            Ok(read) => {
                if let Ok(message) = VitureInboundMessage::parse(&packet[..read]) {
                    if message.command == [0x08, 0x03] {
                        let Ok(pose) =
                            ImuPose::deserialize_from(message.timestamp_ms, &message.payload)
                        else {
                            continue;
                        };

                        if tx.unbounded_send(pose).is_err() {
                            break;
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
}

impl VitureInboundMessage {
    fn parse(body: &[u8]) -> Result<Self, anyhow::Error> {
        let body = if body.len() > 64 && body[0] == 0 {
            &body[1..]
        } else {
            body
        };

        if body.len() < 18 {
            bail!(
                "failed to read VITURE message, only read {} bytes, expected at least 18",
                body.len()
            );
        }

        if body[0] != 0xff || (body[1] & 0xfe) != 0xfc {
            bail!(
                "invalid VITURE message magic: 0x{:02x}{:02x}",
                body[0],
                body[1]
            );
        }

        let data_len = u16::from_le_bytes(body[4..6].try_into()?) as usize;
        let payload_len = data_len
            .checked_sub(0x0c)
            .ok_or_else(|| anyhow!("invalid VITURE message length: {data_len}"))?;
        let payload_start = 18usize;
        let payload_end = payload_start
            .checked_add(payload_len)
            .ok_or_else(|| anyhow!("VITURE message payload is too large"))?;
        if body.len() < payload_end {
            bail!(
                "failed to read complete VITURE message, only read {} bytes, expected {}",
                body.len(),
                payload_end
            );
        }

        let checksum = u16::from_le_bytes(body[2..4].try_into()?);
        let checksum_end = 4 + data_len + 2;
        if body.len() < checksum_end {
            bail!(
                "failed to read complete VITURE checksum body, only read {} bytes, expected {}",
                body.len(),
                checksum_end
            );
        }

        let expected_checksum =
            crc::Crc::<u16>::new(&crc::CRC_16_XMODEM).checksum(&body[4..checksum_end]);
        if expected_checksum != checksum {
            bail!("invalid VITURE message checksum: {expected_checksum}");
        }

        Ok(Self {
            command: body[14..16].try_into()?,
            timestamp_ms: u32::from_le_bytes(body[10..14].try_into()?),
            payload: body[payload_start..payload_end].to_vec(),
        })
    }
}
