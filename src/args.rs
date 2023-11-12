use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "Safe")]
#[command(author = "Erick, Anabeth, Toto, Felix")]
#[command(version = "0.1")]
#[command(about = "A simple password manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Subcommands,
}

#[derive(Subcommand, Debug)]
pub enum Subcommands {
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
