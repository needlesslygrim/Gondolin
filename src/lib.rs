use std::fs::File;
use std::io::BufWriter;

use color_eyre::{eyre::Context, Result};
use ron::ser::PrettyConfig;

pub mod args;
mod models;

use args::{Cli, Subcommands::*};
use models::Database;

pub fn run(args: Cli) -> Result<()> {
    let mut database: Database =
        Database::open("db.ron").wrap_err("Failed to open the database")?;

    match args.subcommand {
        New => database
            .add_new_interactive()
            .wrap_err("Failed to add a new login to the database")?,
        Query(name) => database.query(name.name.as_ref()),
    }

    database
        .sync("db.ron")
        .wrap_err("Failed to sync database to disk")?;

    Ok(())
}
