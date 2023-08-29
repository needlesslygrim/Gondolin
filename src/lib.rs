use std::fs::File;
use std::io::BufWriter;

use color_eyre::{eyre::Context, Result};
use ron::ser::PrettyConfig;

pub mod args;
mod models;

use args::{Cli, Subcommands::*};
use models::Database;

pub fn run(args: Cli) -> Result<()> {
    match args.subcommand {
        Init => {
            Database::init("db.ron").wrap_err("Failed to initialise database")?;
        }
        New => Database::open("db.ron")
            .wrap_err("Failed to open database to add a login")?
            .add_new_interactive()
            .wrap_err("Failed to add a new login to the database")?
            .sync("db.ron")
            .wrap_err("Failed to sync database to disk")?,
        Query(name) => Database::open("db.ron")
            .wrap_err("Failed to open database to query logins")?
            .query(name.name.as_ref())
            .sync("db.ron")
            .wrap_err("Failed to sync database to disk")?,
    };

    Ok(())
}
