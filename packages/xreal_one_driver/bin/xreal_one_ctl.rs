use std::path::PathBuf;
use clap::{Parser, Subcommand};
use xreal_one_driver::UsbDevice;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Debug, Subcommand)]
#[command(author, version, about, long_about = None)]
enum Commands {
    Update {
        mcu_path: PathBuf,
        pilot_path: PathBuf,
    }
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    match args.command {
        Commands::Update { mcu_path, pilot_path } => {
            let api = hidapi::HidApi::new()?;
            let device = UsbDevice::open(&api)?;

            {
                let mcu_bytes = std::fs::read(mcu_path)?;
                device.update_mcu(&mcu_bytes)?;
            }

            {
                let pilot_bytes = std::fs::read(pilot_path)?;
                device.update_pilot(&pilot_bytes)?;
            }

            Ok(())
        }
    }
}
