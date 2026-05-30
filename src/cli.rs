use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Cli {}

pub fn run() -> Result<()> {
    Cli::parse();
    crate::app::run()
}
