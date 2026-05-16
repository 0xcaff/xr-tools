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
    GetConfig,
    Info,
    Enable {
        target: commands::target::Target,
    },
    Disable {
        target: commands::target::Target,
    },
    EnableCamera,
    Flash {
        #[command(subcommand)]
        command: commands::flash::Command,
    },
    RestartRecovery,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::GetConfig => commands::get_config::run().await,
        Command::Info => commands::info::run().await,
        Command::Enable { target } => commands::target::run(target, true).await,
        Command::Disable { target } => commands::target::run(target, false).await,
        Command::EnableCamera => commands::enable_camera::run().await,
        Command::Flash { command } => commands::flash::run(command).await,
        Command::RestartRecovery => commands::restart_recovery::run().await,
    }
}
