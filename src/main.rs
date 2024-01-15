use anyhow::Result;
use clap::Parser;

mod cli;

use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse_from(wild::args());

    pretty_env_logger::formatted_builder()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    cli.execute()
}
