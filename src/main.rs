use clap::Parser;
use log::error;

mod cli;

use cli::Cli;

fn main() {
    let cli = Cli::parse_from(wild::args());

    pretty_env_logger::formatted_builder()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    if let Err(e) = cli.execute() {
        error!("{e:#}")
    }
}
