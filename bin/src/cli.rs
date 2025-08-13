use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use tracing_otel_extra::Logger;

use crate::commands::{api, node, scheduler, streams};

#[derive(Parser)]
#[clap(version, about, propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn from_env_and_args() -> Self {
        dotenv().ok();
        Self::parse()
    }
}

/// Work seamlessly with Sola from the command line.
///
/// See `sola --help` for more information.
#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(name = "api", about = "Start the API server")]
    Api(api::Command),
    #[command(name = "node", about = "Start the Node server")]
    Node(node::Command),
    #[command(name = "scheduler", about = "Start the Scheduler server")]
    Scheduler(scheduler::Command),
    #[command(name = "streams", about = "Start the Streams server")]
    Streams(streams::Command),
}

/// Parse CLI options, set up logging and run the chosen command.
pub async fn run() -> anyhow::Result<()> {
    let opt = Cli::from_env_and_args();
    let guard = Logger::from_env(None)?.init().expect("Failed to initialize logging");

    match opt.command {
        Commands::Api(command) => command.execute().await?,
        Commands::Node(command) => command.execute().await?,
        Commands::Scheduler(command) => command.execute().await?,
        Commands::Streams(command) => command.execute().await?,
    }
    drop(guard);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// Tests that the help message is parsed correctly. This ensures that clap args are configured
    /// correctly and no conflicts are introduced via attributes that would result in a panic at
    /// runtime
    #[test]
    fn test_parse_help_all_subcommands() {
        let cli: clap::Command = Cli::command();
        for sub_command in cli.get_subcommands() {
            let err = Cli::try_parse_from(["key", sub_command.get_name(), "--help"])
                .err()
                .unwrap_or_else(|| {
                    panic!("Failed to parse help message {}", sub_command.get_name())
                });

            // --help is treated as error, but
            // > Not a true "error" as it means --help or similar was used. The help message will be sent to stdout.
            assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
        }
    }
}
