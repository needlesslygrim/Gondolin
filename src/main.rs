use clap::Parser;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    gondolin::run(gondolin::args::Cli::parse())
}
