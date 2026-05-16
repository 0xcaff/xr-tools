use crate::commands::target;
use futures::StreamExt;
use xreal_one_driver::ControlNetworkDevice;

pub async fn run() -> Result<(), anyhow::Error> {
    let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;
    tokio::spawn(inbound_messages.for_each(|_| async {}));

    let response = device.get_config_raw().await?;
    println!("{}", response);

    target::print_statuses(&mut device).await?;

    Ok(())
}
