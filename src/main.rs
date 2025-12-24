use clap::Parser;

mod api;
mod cli;
mod commands;
mod config;
mod models;
mod neo4j;
mod utils;

use cli::Cli;
use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting BinaryX-Graph...");

    let cli = Cli::parse();
    let config = Config::load_from_path(cli.config.as_deref())?;
    cli.execute(config).await?;

    Ok(())
}
