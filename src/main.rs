use anyhow::Result;
use clap::Parser;

mod cli;

use cli::Cli;

fn main() -> Result<()> {
    Cli::parse_from(wild::args()).execute()
}
