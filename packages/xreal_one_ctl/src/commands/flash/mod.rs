use clap::Subcommand;

// pub mod dsp;
pub mod mcu;
pub mod mcu_recovery;
// pub mod pilot;

#[derive(Debug, Subcommand)]
pub enum Command {
    Mcu {
        #[command(flatten)]
        args: mcu::Args,
    },
    // Dsp {
    //     #[command(flatten)]
    //     args: dsp::Args,
    // },
    // Pilot {
    //     #[command(flatten)]
    //     args: pilot::Args,
    // },
    McuRecovery {
        #[command(flatten)]
        args: mcu_recovery::Args,
    },
}

pub async fn run(command: Command) -> Result<(), anyhow::Error> {
    match command {
        Command::Mcu { args } => mcu::run(args).await,
        // Command::Dsp { args } => dsp::run(args).await,
        // Command::Pilot { args } => pilot::run(args).await,
        Command::McuRecovery { args } => mcu_recovery::run(args).await,
    }
}
