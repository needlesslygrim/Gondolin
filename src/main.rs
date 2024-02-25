use std::env;

use clap::Parser;
use color_eyre::eyre::{eyre, Context};
use log::error;

// TODO: Find a way to exit properly, instead of using `std::process::exit()`.
// This might give us a way to run destructors automagically. However, it may
// not work with color_eyre, so research is needed :^).
fn main() -> color_eyre::Result<()> {
    let args = locket::args::Cli::parse();
    color_eyre::install()?;
    match env::vars().find(|(var, _)| var == "LOCKET_LOG") {
        Some((_, value)) => {
            match pretty_env_logger::formatted_builder()
                .parse_env(value)
                .try_init()
            {
                Ok(_) => (),
                Err(e) => {
                    pretty_env_logger::formatted_builder()
                        .filter_level(args.verbosity.log_level_filter())
                        .try_init()
                        .wrap_err_with(|| eyre!("Failed to initialise pretty_env_logger, already failed to parse LOCKET_LOG with error: {e}"))?;

                    error!("Failed to parse LOCKET_LOG with error: {e}");
                }
            };
        }
        None => {
            pretty_env_logger::formatted_builder()
                .filter_level(args.verbosity.log_level_filter())
                .try_init()
                .wrap_err("Failed to initialise pretty_env_logger")?;
        }
    }

    locket::run(args)
}
