use clap::ValueEnum;
use futures::StreamExt;
use xreal_one_driver::ControlNetworkDevice;

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Target {
    Proximity,
    #[value(alias = "stab")]
    Stabilizer,
}

impl Target {
    const ALL: [Self; 2] = [Self::Proximity, Self::Stabilizer];

    fn name(self) -> &'static str {
        match self {
            Self::Proximity => "proximity",
            Self::Stabilizer => "stabilizer",
        }
    }
}

pub async fn run(target: Target, enabled: bool) -> Result<(), anyhow::Error> {
    let (mut device, inbound_messages) = ControlNetworkDevice::new().await?;
    tokio::spawn(inbound_messages.for_each(|_| async {}));

    set_enabled(&mut device, target, enabled).await?;

    let status = if enabled { "enabled" } else { "disabled" };
    println!("{} {}", status, target.name());

    Ok(())
}

pub async fn print_statuses(device: &mut ControlNetworkDevice) -> Result<(), anyhow::Error> {
    for target in Target::ALL {
        let enabled = get_enabled(device, target).await?;
        let status = if enabled { "enabled" } else { "disabled" };
        println!("{}: {}", target.name(), status);
    }

    Ok(())
}

async fn set_enabled(
    device: &mut ControlNetworkDevice,
    target: Target,
    enabled: bool,
) -> Result<(), anyhow::Error> {
    match target {
        Target::Proximity => device.set_proximity_enable(enabled).await,
        Target::Stabilizer => device.set_space_screen_eis_enable(enabled).await,
    }
}

async fn get_enabled(
    device: &mut ControlNetworkDevice,
    target: Target,
) -> Result<bool, anyhow::Error> {
    match target {
        Target::Proximity => device.get_proximity_enable().await,
        Target::Stabilizer => device.get_space_screen_eis_enable().await,
    }
}
