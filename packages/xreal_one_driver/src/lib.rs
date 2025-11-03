//! # xreal one interfaces
//!
//! the xreal one devices have three distinct interfaces to talk to the device, each serving a
//! different purpose. there is some overlap in functionality between these
//! - usbhid control channel for low-level, direct device management
//! - control (tcp) for configuration and command/response style operations
//! - reports (tcp) for continuous sensor/telemetry streams (e.g., imu)
//!
//! ## usbhid
//! used for lower-level functions like reading firmware info, firmware updates, and usb-config
//! queries (enable mtp or camera). see the methods on [`UsbDevice`]
//!
//! ### usage
//! ```rust
//! use xreal_one_driver::{XrealOneModel, UsbDevice};
//! use xreal_one_driver::UsbConfigList;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Discover and open the device over HID
//! let api = hidapi::HidApi::new()?;
//!     let model = api
//!         .device_list()
//!         .find_map(XrealOneModel::detect)
//!         .expect("XREAL One not found");
//!     let usb = UsbDevice::open(&api, model)?;
//!     usb.set_usb_config(UsbConfigList::new().with_uvc0(1).with_enable(1))?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## controls (tcp)
//! command/response protocol over tcp used for runtime configuration and control of the device.
//! create a [`ControlNetworkDevice`] to send commands; it returns a stream of inbound
//! control messages (e.g., key state changes) that you should poll concurrently.
//! common operations include querying config, setting scene/input modes, brightness, dimmer level,
//! and reading versions/ids.
//!
//! ### usage
//! ```rust
//! # #[tokio::main] async fn main() -> anyhow::Result<()> {
//! use futures::StreamExt;
//! use xreal_one_driver::ControlNetworkDevice;
//!
//! // Establish the control connection and start draining inbound messages
//! let (mut control, inbound) = ControlNetworkDevice::new().await?;
//! tokio::spawn(inbound.for_each(|_msg| async {
//!     // Handle unsolicited control messages such as key state changes here
//! }));
//!
//! // Send control commands
//! control.set_display_brightness(5).await?;
//! let config = control.get_config().await?;
//! // ... use `config` (e.g., calibration, transforms)
//! # Ok(()) }
//! ```
//!
//! ## reports (tcp)
//! continuous telemetry stream (e.g., imu and magnetometer reports). use [`net::reports::listen()`]
//! to obtain a `stream` of typed report messages. imu data provides gyro/accel (and magnetometer)
//! values in a consistent coordinate system, suitable for ahrs/orientation estimation.
//!
//! ### usage
//! ```rust
//! # #[tokio::main] async fn main() -> anyhow::Result<()> {
//! use futures::StreamExt;
//! use xreal_one_driver::proto::net::reports;
//! use xreal_one_driver::ReportType;
//!
//! let reports = reports::listen().await?;
//! futures::pin_mut!(reports);
//!
//! while let Some(msg) = reports.next().await.transpose()? {
//!     if let reports::ReportsInboundMessageType::Report(r) = msg {
//!         if r.report_type == ReportType::IMU {
//!             // r.gx, r.gy, r.gz (rad/s), r.ax, r.ay, r.az (m/s^2), etc.
//!             // Use alongside calibration/config from the control interface as needed
//!         }
//!     }
//! }
//! # Ok(()) }
//! ```
pub mod proto;

pub use proto::net::config;
pub use proto::net::control::*;
pub use proto::net::reports::*;
pub use proto::usb::{UsbConfigList, UsbDevice, XrealOneModel};
