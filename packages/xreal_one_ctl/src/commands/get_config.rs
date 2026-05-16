use futures::StreamExt;
use xreal_one_driver::ControlNetworkDevice;

#[derive(Debug, clap::Args)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<(), anyhow::Error> {
    let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;
    tokio::spawn(inbound_messages.for_each(|_| async {}));

    let response = device.get_config_raw().await?;
    println!("{}", response);

    Ok(())
}
