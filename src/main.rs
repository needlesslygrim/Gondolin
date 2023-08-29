use clap::Parser;
use safe::args::Cli;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    safe::run(Cli::parse())
}
