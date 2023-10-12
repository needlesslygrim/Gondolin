use clap::Parser;

// TODO: Find a way to exit properly, instead of using `std::process::exit()`.
// This might give us a way to run destructors automagically. However, it may
// not work with color_eyre, so research is needed :^).
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    gondolin::run(gondolin::args::Cli::parse())
}
