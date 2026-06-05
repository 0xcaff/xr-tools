use std::collections::BTreeSet;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};
use xreal_one_driver::{RecoveryUsbDevice, UsbDevice, XrealOneModel, XrealOneRecoveryModel};

pub type UsbRunnerHandle = JoinHandle<Result<(), anyhow::Error>>;

fn describe_model(model: &XrealOneModel<'_>) -> String {
    format!(
        "{:?} {:04x}:{:04x} {} serial={}",
        model.kind,
        model.device.vendor_id(),
        model.device.product_id(),
        model.device.product_string().unwrap_or("unknown product"),
        model.device.serial_number().unwrap_or("unknown")
    )
}

fn describe_recovery_model(model: &XrealOneRecoveryModel<'_>) -> String {
    format!(
        "{:?} recovery {:04x}:{:04x} {} serial={}",
        model.kind,
        model.device.vendor_id(),
        model.device.product_id(),
        model.device.product_string().unwrap_or("unknown product"),
        model.device.serial_number().unwrap_or("unknown")
    )
}

fn model_identity_key(model: &XrealOneModel<'_>) -> String {
    format!(
        "{:?}:{:04x}:{:04x}:{}",
        model.kind,
        model.device.vendor_id(),
        model.device.product_id(),
        model.device.serial_number().unwrap_or("")
    )
}

fn recovery_model_identity_key(model: &XrealOneRecoveryModel<'_>) -> String {
    format!(
        "{:?}:{:04x}:{:04x}:{}",
        model.kind,
        model.device.vendor_id(),
        model.device.product_id(),
        model.device.serial_number().unwrap_or("")
    )
}

pub fn open_normal_usb_device(
    api: &hidapi::HidApi,
) -> Result<(UsbDevice, UsbRunnerHandle), anyhow::Error> {
    try_open_normal_usb_device(api)?
        .ok_or_else(|| anyhow::anyhow!("no normal XREAL HID device found"))
}

pub fn try_open_normal_usb_device(
    api: &hidapi::HidApi,
) -> Result<Option<(UsbDevice, UsbRunnerHandle)>, anyhow::Error> {
    let mut matches = api
        .device_list()
        .filter_map(XrealOneModel::detect)
        .collect::<Vec<_>>();

    let identities = matches
        .iter()
        .map(model_identity_key)
        .collect::<BTreeSet<_>>();
    if identities.len() > 1 {
        let descriptions = matches
            .iter()
            .map(describe_model)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "multiple normal XREAL HID devices found; disconnect extras first: {}",
            descriptions
        );
    }

    let Some(model) = matches.pop() else {
        return Ok(None);
    };

    let (device, usb_runner) = UsbDevice::open(api, model)?;
    Ok(Some((device, tokio::spawn(usb_runner))))
}

pub fn try_open_recovery_usb_device(
    api: &hidapi::HidApi,
) -> Result<Option<(RecoveryUsbDevice, UsbRunnerHandle)>, anyhow::Error> {
    let mut matches = api
        .device_list()
        .filter_map(XrealOneRecoveryModel::detect)
        .collect::<Vec<_>>();

    let identities = matches
        .iter()
        .map(recovery_model_identity_key)
        .collect::<BTreeSet<_>>();
    if identities.len() > 1 {
        let descriptions = matches
            .iter()
            .map(describe_recovery_model)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "multiple recovery XREAL HID devices found; disconnect extras first: {}",
            descriptions
        );
    }

    let Some(model) = matches.pop() else {
        return Ok(None);
    };

    let (device, usb_runner) = RecoveryUsbDevice::open(api, model)?;
    Ok(Some((device, tokio::spawn(usb_runner))))
}

pub async fn wait_for_recovery_usb_device(
    api: &mut hidapi::HidApi,
    timeout: Duration,
) -> Result<(RecoveryUsbDevice, UsbRunnerHandle), anyhow::Error> {
    let deadline = Instant::now() + timeout;

    loop {
        api.refresh_devices()?;
        if let Some(device) = try_open_recovery_usb_device(api)? {
            return Ok(device);
        }

        let now = Instant::now();
        if now >= deadline {
            anyhow::bail!(
                "timed out after {}s waiting for recovery XREAL HID device",
                timeout.as_secs()
            );
        }

        sleep(std::cmp::min(
            Duration::from_millis(250),
            deadline.saturating_duration_since(now),
        ))
        .await;
    }
}
