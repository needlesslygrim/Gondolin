#![warn(clippy::all)]
#![warn(clippy::pedantic)]

use std::{env, fs, fs::OpenOptions, hint::unreachable_unchecked, io::ErrorKind};

use color_eyre::eyre::bail;
use color_eyre::{eyre::Context, Result};

pub mod args;
mod errors;
mod models;
#[cfg(feature = "web")]
mod net;

use crate::args::InitArgs;
use crate::models::Config;
use args::Cli;
use models::Database;

static DATABASE_FILE_NAME: &str = "gondolin.db";
static CONFIG_FILE_NAME: &str = "gondolin.toml";
static LCK_FILE_NAME: &str = "gondolin.lck";

// TODO: Extract the logic of opening and closing the config, database, and lockfile into either a set of functions, or an empty struct called
// `Program` or something, which is responsible for all of this stuff. That would also improve the shutdown logic in `net::serve()`, and would
// ensure that both functions stayed up to date. This is not especially urgent since it's just another abstraction which would overcomplicate
// this project even more, but at some point this should be done.
pub fn run(args: Cli) -> Result<()> {
    let Some(proj_dirs) =
        directories::ProjectDirs::from("com.github", "needlesslygrim", "Gondolin")
    else {
        bail!("Failed to get project directories")
    };

    let conf_dir = proj_dirs.config_dir();
    let data_dir = proj_dirs.data_dir();

    if !conf_dir
        .try_exists()
        .wrap_err("Failed to check if configuration dir exists")?
        || !data_dir
            .try_exists()
            .wrap_err("Failed to check if data dir exists")?
    {
        fs::create_dir_all(conf_dir).wrap_err("Failed to create configuration dir")?;
        fs::create_dir_all(data_dir).wrap_err("Failed to create data dir")?;
    }

    let conf_path = conf_dir.join(CONFIG_FILE_NAME);
    let db_path = data_dir.join(DATABASE_FILE_NAME);

    // Alias it to `C` (Command)
    use args::Subcommands as C;
    if let C::Init(InitArgs { port }) = args.subcommand {
        Config::init_interactive(&conf_path, &db_path, port)
            .wrap_err("Failed to initialise configuration file")?;
        Database::init(&db_path).wrap_err("Failed to initialise database")?;

        println!("Successfully initialised a database and configuration file");
        return Ok(());
    }

    let config =
        Config::open_interactive(&conf_path).wrap_err("Failed to open config interactively")?;

    let mut db = Database::open(&config.path).wrap_err("Failed to open the existing database")?;

    let mut lck_path = env::temp_dir();
    lck_path.push(LCK_FILE_NAME);

    if let Err(err) = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&lck_path)
    {
        match err.kind() {
            ErrorKind::AlreadyExists => {
                eprintln!("An instance of Gondolin is already running, please kill it or wait for it to quit before trying to run another instance");
                std::process::exit(1);
            }
            _ => bail!("Failed to open the lockfile: {}", err),
        }
    };

    match args.subcommand {
        // Hopefully this isn't a bad idea :)
        C::Init(_) => unsafe { unreachable_unchecked() },
        C::New => db
            .add_login_interactive()
            .wrap_err("Failed to add a new login to the database")?,
        C::Query(name) => db.query_interactive(name.name.as_deref()),
        C::Remove => {
            db.remove_interactive()
                .wrap_err("Failed to remove a login from the database interactively")?;
        }
        #[cfg(feature = "web")]
        C::Serve => {
            net::serve(&mut db, config.port, &lck_path).wrap_err("Failed to serve webpage")?;
        }
    };

    db.sync().wrap_err("Failed to sync database to disk")?;
    if let Err(err) = fs::remove_file(lck_path) {
        match err.kind() {
            ErrorKind::NotFound => {
                // TODO: Improve this message.
                eprintln!("Tried to remove the lockfile, but it was already gone");
                std::process::exit(1);
            }
            _ => bail!("Failed to remove the lockfile: {}", err),
        }
    };
    Ok(())
}
