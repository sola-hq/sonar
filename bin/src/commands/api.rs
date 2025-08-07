use clap::Parser;

use dotenvy::dotenv;
use sonar_api::init_api;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Sonar API server")]
pub struct Command {
    /// Http port to use
    #[arg(short, long, default_value_t = 8080, env = "PORT")]
    port: u16,
}

impl Command {
    /// Execute `api` command
    pub async fn execute(self) -> anyhow::Result<()> {
        dotenv().ok();
        init_api().await?;
        Ok(())
    }
}
