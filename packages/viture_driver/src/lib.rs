//! VITURE Luma HID pose stream support.
//!
//! The Luma SDK path used by the demo does not locally integrate raw gyro and
//! accelerometer samples. It opens V1 pose mode over HID and forwards `0x0308`
//! pose frames containing roll, pitch, yaw, and quaternion values. This crate
//! follows that path and stores the latest parsed pose on the opened device.

use anyhow::{anyhow, bail, Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::collections::BTreeSet;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const VITURE_VENDOR_ID: u16 = 0x35ca;
pub const LUMA_PRODUCT_ID: u16 = 0x1131;
pub const LUMA_PRO_PRODUCT_ID_1: u16 = 0x1121;
pub const LUMA_PRO_PRODUCT_ID_2: u16 = 0x1141;
pub const LUMA_CYBER_PRODUCT_ID: u16 = 0x1151;

const V1_PACKET_SIZE: usize = 64;
const V1_COMMAND_MAGIC: [u8; 2] = [0xff, 0xfe];
const V1_IMU_EVENT_MESSAGE_ID: u16 = 0x0308;
const V1_SET_IMU_MODE_MESSAGE_ID: u16 = 0x0015;
const V1_SET_IMU_FREQUENCY_MESSAGE_ID: u16 = 0x0018;
const VITURE_IMU_MODE_OFF: u8 = 0;
const VITURE_IMU_MODE_POSE: u8 = 1;
const VITURE_IMU_FREQUENCY_HIGH: u8 = 4;
const CONTROL_INTERFACE_NUMBER: i32 = 1;
const IMU_INTERFACE_NUMBER: i32 = 0;
const READER_OPEN_TIMEOUT: Duration = Duration::from_secs(2);
const READ_TIMEOUT_MS: i32 = 1000;

#[derive(Clone, Copy, Debug, Default)]
pub struct ImuPose {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub quaternion_w: f32,
    pub quaternion_x: f32,
    pub quaternion_y: f32,
    pub quaternion_z: f32,
    pub timestamp_ns: i64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PoseSnapshot {
    pub pose: Option<ImuPose>,
    pub sample_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VitureLumaModelKind {
    Luma,
    LumaPro,
    LumaCyber,
}

impl VitureLumaModelKind {
    pub fn detect(product_id: u16) -> Option<Self> {
        match product_id {
            LUMA_PRODUCT_ID => Some(Self::Luma),
            LUMA_PRO_PRODUCT_ID_1 | LUMA_PRO_PRODUCT_ID_2 => Some(Self::LumaPro),
            LUMA_CYBER_PRODUCT_ID => Some(Self::LumaCyber),
            _ => None,
        }
    }

    pub fn market_name(self) -> &'static str {
        match self {
            Self::Luma => "Luma",
            Self::LumaPro => "Luma Pro",
            Self::LumaCyber => "Luma Cyber",
        }
    }
}

pub struct VitureLumaModel<'a> {
    device: &'a DeviceInfo,
    kind: VitureLumaModelKind,
}

impl VitureLumaModel<'_> {
    pub fn detect<'a>(device: &'a DeviceInfo) -> Option<VitureLumaModel<'a>> {
        if device.vendor_id() != VITURE_VENDOR_ID {
            return None;
        }

        let kind = VitureLumaModelKind::detect(device.product_id())?;
        Some(VitureLumaModel { device, kind })
    }

    pub fn kind(&self) -> VitureLumaModelKind {
        self.kind
    }

    pub fn vendor_id(&self) -> u16 {
        self.device.vendor_id()
    }

    pub fn product_id(&self) -> u16 {
        self.device.product_id()
    }

    pub fn market_name(&self) -> &'static str {
        self.kind.market_name()
    }

    pub fn product_string(&self) -> Option<&str> {
        self.device.product_string()
    }

    pub fn serial_number(&self) -> Option<&str> {
        self.device.serial_number()
    }
}

#[derive(Clone, Debug)]
pub struct DeviceCandidate {
    pub product_id: i32,
    pub market_name: String,
}

#[derive(Clone, Debug)]
struct HidEndpointInfo {
    path: CString,
    interface_number: i32,
    usage_page: u16,
    usage: u16,
}

#[derive(Debug, Default)]
struct PoseStore {
    latest: Option<ImuPose>,
    sample_count: u64,
}

struct VitureEndpoint {
    control_device: HidDevice,
    reader: Option<JoinHandle<()>>,
    reader_running: Arc<AtomicBool>,
    pose_store: Arc<Mutex<PoseStore>>,
}

pub struct VitureDevice {
    endpoint: VitureEndpoint,
    pub product_id: i32,
    pub market_name: String,
}

impl VitureDevice {
    pub fn open(api: &HidApi, model: VitureLumaModel<'_>) -> Result<Self> {
        Self::open_product_id(api, model.product_id())
    }

    pub fn open_product_id(api: &HidApi, product_id: u16) -> Result<Self> {
        let kind = VitureLumaModelKind::detect(product_id).ok_or_else(|| {
            anyhow!("PID 0x{product_id:04X} does not use the Luma Gen1 HID pose stream")
        })?;
        let endpoint = VitureEndpoint::open_product_id(api, product_id)?;

        Ok(Self {
            endpoint,
            product_id: i32::from(product_id),
            market_name: kind.market_name().to_owned(),
        })
    }

    pub fn connect_preferred(requested_pid: Option<i32>) -> Result<Self> {
        let mut api = HidApi::new().context("failed to initialize hidapi")?;
        configure_hid_api(&mut api);

        let product_id = if let Some(pid) = requested_pid {
            let product_id =
                u16::try_from(pid).with_context(|| format!("invalid VITURE PID {pid}"))?;
            if VitureLumaModelKind::detect(product_id).is_none() {
                bail!("PID 0x{product_id:04X} does not use the Luma Gen1 HID pose stream");
            }
            product_id
        } else {
            choose_preferred_product_id(&discover_luma_candidates(&api))
                .ok_or_else(|| anyhow!("no VITURE Luma HID device detected"))?
        };

        Self::open_product_id(&api, product_id)
    }

    pub fn latest_pose(&self) -> PoseSnapshot {
        self.endpoint.latest_pose()
    }
}

impl VitureEndpoint {
    fn open_product_id(api: &HidApi, product_id: u16) -> Result<Self> {
        let endpoints = matching_endpoints(api, product_id);
        if endpoints.is_empty() {
            bail!("PID 0x{product_id:04X} is supported, but no matching HID endpoint is present");
        }

        let controls = sorted_endpoints_for_interface(&endpoints, CONTROL_INTERFACE_NUMBER);
        let imus = sorted_endpoints_for_interface(&endpoints, IMU_INTERFACE_NUMBER);
        let mut errors = Vec::new();

        for control_endpoint in &controls {
            for imu_endpoint in &imus {
                match Self::open_endpoint_pair(api, control_endpoint, imu_endpoint) {
                    Ok(endpoint) => return Ok(endpoint),
                    Err(err) => errors.push(format!(
                        "control {}, imu {}: {err}",
                        control_endpoint.describe(),
                        imu_endpoint.describe()
                    )),
                }
            }
        }

        bail!(
            "failed to open VITURE Luma HID endpoints for PID 0x{product_id:04X}: {}",
            errors.join("; ")
        );
    }

    fn open_endpoint_pair(
        api: &HidApi,
        control_endpoint: &HidEndpointInfo,
        imu_endpoint: &HidEndpointInfo,
    ) -> Result<Self> {
        let control_device = api
            .open_path(control_endpoint.path.as_c_str())
            .with_context(|| {
                format!("failed to open control HID {}", control_endpoint.describe())
            })?;

        let pose_store = Arc::new(Mutex::new(PoseStore::default()));
        let (reader_running, reader) =
            spawn_reader(imu_endpoint.path.clone(), Arc::clone(&pose_store))?;

        if let Err(err) = open_pose_stream(&control_device) {
            reader_running.store(false, Ordering::Relaxed);
            let _ = reader.join();
            return Err(err);
        }

        Ok(Self {
            control_device,
            reader: Some(reader),
            reader_running,
            pose_store,
        })
    }

    fn latest_pose(&self) -> PoseSnapshot {
        match self.pose_store.lock() {
            Ok(store) => PoseSnapshot {
                pose: store.latest,
                sample_count: store.sample_count,
            },
            Err(_) => PoseSnapshot::default(),
        }
    }
}

impl Drop for VitureEndpoint {
    fn drop(&mut self) {
        let _ = send_v1_command(
            &self.control_device,
            V1_SET_IMU_MODE_MESSAGE_ID,
            &[VITURE_IMU_MODE_OFF],
        );
        self.reader_running.store(false, Ordering::Relaxed);
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

pub fn discover_candidates() -> Vec<DeviceCandidate> {
    let mut api = match HidApi::new() {
        Ok(api) => api,
        Err(err) => {
            eprintln!("hidapi init failed: {err}");
            return Vec::new();
        }
    };
    configure_hid_api(&mut api);
    discover_luma_candidates(&api)
}

pub fn discover_luma_candidates(api: &HidApi) -> Vec<DeviceCandidate> {
    let mut product_ids = BTreeSet::new();
    for device in api.device_list() {
        if let Some(model) = VitureLumaModel::detect(device) {
            product_ids.insert(model.product_id());
        }
    }

    product_ids
        .into_iter()
        .map(|product_id| {
            let kind = VitureLumaModelKind::detect(product_id)
                .expect("discover_luma_candidates only inserts Luma products");
            DeviceCandidate {
                product_id: i32::from(product_id),
                market_name: kind.market_name().to_owned(),
            }
        })
        .collect()
}

fn open_pose_stream(control_device: &HidDevice) -> Result<()> {
    send_v1_command(
        control_device,
        V1_SET_IMU_FREQUENCY_MESSAGE_ID,
        &[VITURE_IMU_FREQUENCY_HIGH],
    )
    .context("failed to set VITURE IMU frequency")?;
    send_v1_command(
        control_device,
        V1_SET_IMU_MODE_MESSAGE_ID,
        &[VITURE_IMU_MODE_POSE],
    )
    .context("failed to enable VITURE pose IMU mode")
}

fn send_v1_command(device: &HidDevice, message_id: u16, payload: &[u8]) -> Result<()> {
    let packet = build_v1_command(message_id, payload)?;
    let written = device
        .write(&packet)
        .with_context(|| format!("failed to write VITURE command 0x{message_id:04X}"))?;

    if written == packet.len() {
        Ok(())
    } else {
        bail!(
            "short HID write for VITURE command 0x{message_id:04X}: wrote {written} of {} bytes",
            packet.len()
        );
    }
}

fn build_v1_command(message_id: u16, payload: &[u8]) -> Result<[u8; V1_PACKET_SIZE]> {
    let data_len = 0x0cusize
        .checked_add(payload.len())
        .ok_or_else(|| anyhow!("VITURE command payload is too large"))?;
    if data_len > V1_PACKET_SIZE - 6 {
        bail!(
            "VITURE command 0x{message_id:04X} payload is too large: {} bytes",
            payload.len()
        );
    }

    let mut packet = [0u8; V1_PACKET_SIZE];
    packet[0..2].copy_from_slice(&V1_COMMAND_MAGIC);
    packet[4..6].copy_from_slice(&(data_len as u16).to_le_bytes());
    packet[14..16].copy_from_slice(&message_id.to_le_bytes());
    packet[18..18 + payload.len()].copy_from_slice(payload);

    let crc_end = 4 + data_len + 2;
    let crc = crc16_viture(&packet[4..crc_end]);
    packet[2..4].copy_from_slice(&crc.to_le_bytes());

    Ok(packet)
}

fn spawn_reader(
    path: CString,
    pose_store: Arc<Mutex<PoseStore>>,
) -> Result<(Arc<AtomicBool>, JoinHandle<()>)> {
    let running = Arc::new(AtomicBool::new(true));
    let thread_running = Arc::clone(&running);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);

    let reader = thread::spawn(move || read_pose_loop(path, thread_running, pose_store, ready_tx));

    match ready_rx.recv_timeout(READER_OPEN_TIMEOUT) {
        Ok(Ok(())) => Ok((running, reader)),
        Ok(Err(err)) => {
            running.store(false, Ordering::Relaxed);
            let _ = reader.join();
            Err(anyhow!(err))
        }
        Err(RecvTimeoutError::Timeout) => {
            running.store(false, Ordering::Relaxed);
            let _ = reader.join();
            bail!("timed out opening VITURE IMU HID endpoint")
        }
        Err(RecvTimeoutError::Disconnected) => {
            running.store(false, Ordering::Relaxed);
            let _ = reader.join();
            bail!("VITURE IMU reader exited while opening HID endpoint")
        }
    }
}

fn read_pose_loop(
    path: CString,
    running: Arc<AtomicBool>,
    pose_store: Arc<Mutex<PoseStore>>,
    ready_tx: SyncSender<Result<(), String>>,
) {
    let device = match open_reader_device(&path) {
        Ok(device) => {
            let _ = ready_tx.send(Ok(()));
            device
        }
        Err(err) => {
            let _ = ready_tx.send(Err(err.to_string()));
            return;
        }
    };

    let mut packet = [0u8; V1_PACKET_SIZE];
    while running.load(Ordering::Relaxed) {
        match device.read_timeout(&mut packet, READ_TIMEOUT_MS) {
            Ok(0) => {}
            Ok(read) => {
                if let Some(pose) = parse_v1_pose_packet(&packet[..read]) {
                    update_pose(&pose_store, pose);
                }
            }
            Err(err) => {
                if running.load(Ordering::Relaxed) {
                    eprintln!("VITURE IMU read failed: {err}");
                }
                break;
            }
        }
    }
}

fn open_reader_device(path: &CString) -> Result<HidDevice> {
    let mut api = HidApi::new().context("failed to initialize hidapi for VITURE IMU reader")?;
    configure_hid_api(&mut api);
    api.open_path(path.as_c_str())
        .context("failed to open VITURE IMU HID endpoint")
}

fn parse_v1_pose_packet(packet: &[u8]) -> Option<ImuPose> {
    if packet.len() < 18 || packet[0] != 0xff || (packet[1] & 0xfe) != 0xfc {
        return None;
    }

    let data_len = u16::from_le_bytes(packet[4..6].try_into().ok()?) as usize;
    let payload_len = data_len.checked_sub(0x0c)?;
    let payload_start = 18usize;
    let payload_end = payload_start.checked_add(payload_len)?;
    if packet.len() < payload_end || payload_len < 0x24 {
        return None;
    }

    let message_id = u16::from_le_bytes(packet[14..16].try_into().ok()?);
    if message_id != V1_IMU_EVENT_MESSAGE_ID {
        return None;
    }

    let payload = &packet[payload_start..payload_end];
    let timestamp_ms = u32::from_le_bytes(packet[10..14].try_into().ok()?);

    Some(ImuPose {
        roll: read_be_f32(payload, 0x00)?,
        pitch: read_be_f32(payload, 0x04)?,
        yaw: read_be_f32(payload, 0x08)?,
        quaternion_w: read_be_f32(payload, 0x14)?,
        quaternion_x: read_be_f32(payload, 0x18)?,
        quaternion_y: read_be_f32(payload, 0x1c)?,
        quaternion_z: read_be_f32(payload, 0x20)?,
        timestamp_ns: i64::from(timestamp_ms) * 1_000_000,
    })
}

fn read_be_f32(bytes: &[u8], offset: usize) -> Option<f32> {
    let raw = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(f32::from_bits(u32::from_be_bytes(raw)))
}

fn update_pose(pose_store: &Mutex<PoseStore>, pose: ImuPose) {
    if let Ok(mut store) = pose_store.lock() {
        store.latest = Some(pose);
        store.sample_count = store.sample_count.saturating_add(1);
    }
}

fn matching_endpoints(api: &HidApi, product_id: u16) -> Vec<HidEndpointInfo> {
    api.device_list()
        .filter(|device| {
            device.vendor_id() == VITURE_VENDOR_ID && device.product_id() == product_id
        })
        .map(|device| HidEndpointInfo {
            path: device.path().to_owned(),
            interface_number: device.interface_number(),
            usage_page: device.usage_page(),
            usage: device.usage(),
        })
        .collect()
}

fn sorted_endpoints_for_interface(
    endpoints: &[HidEndpointInfo],
    preferred_interface_number: i32,
) -> Vec<HidEndpointInfo> {
    let mut sorted = endpoints.to_vec();
    sorted.sort_by(|a, b| {
        endpoint_rank(a, preferred_interface_number)
            .cmp(&endpoint_rank(b, preferred_interface_number))
            .then_with(|| a.path.as_bytes().cmp(b.path.as_bytes()))
    });
    sorted
}

fn endpoint_rank(
    endpoint: &HidEndpointInfo,
    preferred_interface_number: i32,
) -> (u8, u8, i32, u16, u16) {
    (
        u8::from(endpoint.interface_number != preferred_interface_number),
        u8::from(endpoint.interface_number < 0),
        endpoint.interface_number,
        endpoint.usage_page,
        endpoint.usage,
    )
}

fn choose_preferred_product_id(candidates: &[DeviceCandidate]) -> Option<u16> {
    candidates
        .iter()
        .find(|candidate| candidate.product_id == i32::from(LUMA_PRODUCT_ID))
        .or_else(|| candidates.first())
        .and_then(|candidate| u16::try_from(candidate.product_id).ok())
}

fn configure_hid_api(api: &mut HidApi) {
    #[cfg(target_os = "macos")]
    {
        api.set_open_exclusive(false);
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = api;
    }
}

fn crc16_viture(bytes: &[u8]) -> u16 {
    let mut crc = 0u16;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

impl HidEndpointInfo {
    fn describe(&self) -> String {
        format!(
            "path {}, interface {}, usage 0x{:04X}:0x{:04X}",
            self.path.as_c_str().to_string_lossy(),
            self.interface_number,
            self.usage_page,
            self.usage
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_luma_family_products_are_detected() {
        assert_eq!(
            VitureLumaModelKind::detect(LUMA_PRODUCT_ID),
            Some(VitureLumaModelKind::Luma)
        );
        assert_eq!(
            VitureLumaModelKind::detect(LUMA_PRO_PRODUCT_ID_1),
            Some(VitureLumaModelKind::LumaPro)
        );
        assert_eq!(
            VitureLumaModelKind::detect(LUMA_PRO_PRODUCT_ID_2),
            Some(VitureLumaModelKind::LumaPro)
        );
        assert_eq!(
            VitureLumaModelKind::detect(LUMA_CYBER_PRODUCT_ID),
            Some(VitureLumaModelKind::LumaCyber)
        );
        assert_eq!(VitureLumaModelKind::detect(0x1101), None);
        assert_eq!(VitureLumaModelKind::detect(0x1201), None);
    }

    #[test]
    fn crc_matches_xmodem_vector() {
        assert_eq!(crc16_viture(b"123456789"), 0x31c3);
    }

    #[test]
    fn build_v1_frequency_command_matches_sdk_packet_shape() {
        let packet = build_v1_command(V1_SET_IMU_FREQUENCY_MESSAGE_ID, &[4]).unwrap();

        assert_sdk_command_prefix(
            &packet,
            &[
                0xff, 0xfe, 0xf8, 0x54, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x18, 0x00, 0x00, 0x00, 0x04,
            ],
        );
    }

    #[test]
    fn build_v1_pose_mode_command_matches_sdk_packet_shape() {
        let packet = build_v1_command(V1_SET_IMU_MODE_MESSAGE_ID, &[VITURE_IMU_MODE_POSE]).unwrap();

        assert_sdk_command_prefix(
            &packet,
            &[
                0xff, 0xfe, 0x27, 0x25, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x15, 0x00, 0x00, 0x00, 0x01,
            ],
        );
    }

    #[test]
    fn build_v1_pose_off_command_matches_sdk_packet_shape() {
        let packet = build_v1_command(V1_SET_IMU_MODE_MESSAGE_ID, &[VITURE_IMU_MODE_OFF]).unwrap();

        assert_sdk_command_prefix(
            &packet,
            &[
                0xff, 0xfe, 0x06, 0x35, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x15, 0x00, 0x00, 0x00, 0x00,
            ],
        );
    }

    #[test]
    fn parse_pose_packet_matches_sdk_parse_0x0308_fields() {
        let packet = sdk_pose_packet_fixture();
        let pose = parse_v1_pose_packet(&packet).unwrap();

        assert_eq!(pose.roll, 12.5);
        assert_eq!(pose.pitch, -3.25);
        assert_eq!(pose.yaw, 90.0);
        assert_eq!(pose.quaternion_w, 0.70710677);
        assert_eq!(pose.quaternion_x, 0.0);
        assert_eq!(pose.quaternion_y, 0.70710677);
        assert_eq!(pose.quaternion_z, 0.0);
        assert_eq!(pose.timestamp_ns, 1_234_000_000);
    }

    fn sdk_pose_packet_fixture() -> [u8; V1_PACKET_SIZE] {
        let mut packet = [0u8; V1_PACKET_SIZE];
        let payload_len = 0x24usize;
        let data_len = 0x0c + payload_len;

        packet[0] = 0xff;
        packet[1] = 0xfc;
        packet[4..6].copy_from_slice(&(data_len as u16).to_le_bytes());
        packet[10..14].copy_from_slice(&1234u32.to_le_bytes());
        packet[14..16].copy_from_slice(&V1_IMU_EVENT_MESSAGE_ID.to_le_bytes());
        write_be_f32(&mut packet, 18, 12.5);
        write_be_f32(&mut packet, 18 + 0x04, -3.25);
        write_be_f32(&mut packet, 18 + 0x08, 90.0);
        write_be_f32(&mut packet, 18 + 0x14, 0.70710677);
        write_be_f32(&mut packet, 18 + 0x18, 0.0);
        write_be_f32(&mut packet, 18 + 0x1c, 0.70710677);
        write_be_f32(&mut packet, 18 + 0x20, 0.0);

        packet
    }

    fn write_be_f32(packet: &mut [u8], offset: usize, value: f32) {
        packet[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
    }

    fn assert_sdk_command_prefix(packet: &[u8; V1_PACKET_SIZE], expected_prefix: &[u8]) {
        assert_eq!(&packet[..expected_prefix.len()], expected_prefix);
        assert!(packet[expected_prefix.len()..]
            .iter()
            .all(|byte| *byte == 0));

        let data_len = u16::from_le_bytes(packet[4..6].try_into().unwrap()) as usize;
        let crc = u16::from_le_bytes(packet[2..4].try_into().unwrap());
        assert_eq!(crc, crc16_viture(&packet[4..4 + data_len + 2]));
    }
}
