use std::hint::unreachable_unchecked;

use color_eyre::{eyre::Context, Result};

pub mod args;
mod models;

use args::Cli;
use models::Database;

pub fn run(args: Cli) -> Result<()> {
    // Alias it to `C` (Command)
    use args::Subcommands as C;
    // FIXME: Impl `Eq`?
    if let C::Init = args.subcommand {
        Database::init("db.ron").wrap_err("Failed to initialise database")?;
        return Ok(());
    }

    let mut db = Database::open("db.ron").wrap_err("Failed to open the existing database")?;
    match args.subcommand {
        // Hopefully thiss isn't a bad idea :)
        C::Init => unsafe { unreachable_unchecked() },
        C::New => db
            .add_new_interactive()
            .wrap_err("Failed to add a new login to the database")?,
        C::Query(name) => db.query(name.name.as_ref()),
        C::Remove => {
            db.remove_interactive()
                .wrap_err("Failed to remove a login from the database interactively")?;
        }
    };

    db.sync("db.ron")
        .wrap_err("Failed to sync database to disk")?;

    Ok(())
}
