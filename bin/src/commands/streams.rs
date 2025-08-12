use clap::Parser;

use dotenvy::dotenv;
use sonar_streams::app::App;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Sonar streams")]
pub struct Command {}

impl Command {
    /// Execute `api` command
    pub async fn execute(self) -> anyhow::Result<()> {
        dotenv().ok();
        let app = App::new();
        app.run().await?;
        Ok(())
    }
}
