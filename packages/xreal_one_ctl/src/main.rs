use clap::{Parser, Subcommand};

mod commands;
mod usb;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    GetConfig {
        #[command(flatten)]
        args: commands::get_config::Args,
    },
    Info {
        #[command(flatten)]
        args: commands::info::Args,
    },
    EnableCamera {
        #[command(flatten)]
        args: commands::enable_camera::Args,
    },
    Update {
        #[command(flatten)]
        args: commands::update::Args,
    },
    FlashDsp {
        #[command(flatten)]
        args: commands::flash_dsp::Args,
    },
    RecoverMcu {
        #[command(flatten)]
        args: commands::recover_mcu::Args,
    },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::GetConfig { args } => commands::get_config::run(args).await,
        Command::Info { args } => commands::info::run(args).await,
        Command::EnableCamera { args } => commands::enable_camera::run(args).await,
        Command::Update { args } => commands::update::run(args).await,
        Command::FlashDsp { args } => commands::flash_dsp::run(args).await,
        Command::RecoverMcu { args } => commands::recover_mcu::run(args).await,
    }
}
