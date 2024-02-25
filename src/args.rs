use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "Safe")]
#[command(author = "needlesslygrim")]
#[command(version = "1.0.0")]
#[command(about = "A simple password manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Subcommands,

    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Subcommand, Debug)]
pub enum Subcommands {
    #[command(about = "Initialise a database and configuration")]
    Init(InitArgs),
    New,
    Query(QueryArgs),
    Remove,
    #[cfg(feature = "web")]
    Serve,
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    #[arg(short, long)]
    pub port: Option<u16>,
}

#[derive(Parser, Debug)]
pub struct QueryArgs {
    pub name: Option<String>,
}
