use crate::proto::usb::{Empty, UsbDevice, UsbTransaction};

pub struct SetUbootUpgradeFlag;

impl UsbTransaction<'static> for SetUbootUpgradeFlag {
    const COMMAND_ID: [u8; 2] = [0x3e, 0x00];
    type RequestArgs = Empty;
    type Response = ();
}

pub struct SystemReboot;

impl UsbTransaction<'static> for SystemReboot {
    const COMMAND_ID: [u8; 2] = [0x44, 0x00];
    type RequestArgs = Empty;
    type Response = ();
}

impl UsbDevice {
    pub fn set_uboot_upgrade_flag(&self) -> Result<(), anyhow::Error> {
        self.send_message::<SetUbootUpgradeFlag>(Empty)?;
        Ok(())
    }

    pub fn system_reboot(&self) -> Result<(), anyhow::Error> {
        self.send_message::<SystemReboot>(Empty)?;
        Ok(())
    }
}
