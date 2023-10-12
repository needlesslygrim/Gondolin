use std::collections::HashMap;
use std::fmt::Display;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::{
    fs,
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read},
};

use color_eyre::eyre::{bail, Context, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{FuzzySelect, Input, Password};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use tabled::{
    settings::Style,
    tables::{PoolTable, TableValue},
    Table, Tabled,
};
use uuid::Uuid;

#[cfg(feature = "paralell_queries")]
use rayon::prelude::*;

use crate::errors::GondolinError;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub path: PathBuf,
    #[cfg(feature = "web")]
    pub port: u16,
}

static DATABASE_FILE_NAME: &'static str = "gondolin.db";
static CONFIG_FILE_NAME: &'static str = "gondolin.toml";

impl Config {
    fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let f = File::open(&path).wrap_err("Failed to open configuration file")?;
        let mut reader = BufReader::new(f);
        let mut buf = String::with_capacity(
            fs::metadata(&path)
                .wrap_err("Failed to get metadata of configuration file")?
                .len() as usize,
        );
        reader
            .read_to_string(&mut buf)
            .wrap_err("Failed to read configuration file")?;

        toml::de::from_str(&buf).wrap_err("Failed to parse configuration file")
    }

    pub(crate) fn open_interactive(path: &Path) -> Result<Self> {
        let theme = dialoguer::theme::ColorfulTheme::default();
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {
                    // Not doing this for now.
                    eprintln!("You have not initialised Gondolin yet, please run `gondolin init` to initialise, then run this command again.");
                    std::process::exit(0);
                    /*                    let init = dialoguer::Confirm::with_theme(&theme).with_prompt("Do you want to initialise a database and a configuration file?").interact_opt().wrap_err("Failed to read choice of whether to initialise a configuration and database")?;
                    if init.is_some_and(|init| init == false) || init.is_none() {
                        bail!(GondolinError::RefuseInitialisationError);
                    }

                    return Self::init_interactive(Path::new(path)).wrap_err(
                        "Failed to initialise interactively after user chose to initialise ",
                    );*/
                }
                _ => {
                    bail!(err)
                }
            },
        };

        let mut buf = String::with_capacity(metadata.len() as usize);
        let mut reader =
            BufReader::new(File::open(&path).wrap_err("Failed to open configuration file")?);
        reader
            .read_to_string(&mut buf)
            .wrap_err("Failed to read configuration file")?;

        toml::de::from_str(&buf).wrap_err("Failed to parse configuration file")
    }

    pub fn init(path: &Path, config: &Config) -> Result<()> {
        let exists = path
            .try_exists()
            .wrap_err("Failed to check whether the configuration file already exists")?;

        if exists {
            bail!(GondolinError::ConfigAlreadyExistsError);
        }

        let mut writer =
            BufWriter::new(File::create(path).wrap_err("Failed to create configuration file")?);
        let buf = toml::ser::to_string_pretty(config)
            .wrap_err("Failed to serialise configuration file")?;
        writer
            .write_all(buf.as_bytes())
            .wrap_err("Failed to write configuration file")?;

        Ok(())
    }

    pub(crate) fn init_interactive(path: &Path, db_path: &Path) -> Result<Self> {
        let theme = ColorfulTheme::default();

        // Removed for now since it's a bit stupid.
        /*        let database_dir = Input::<String>::with_theme(&theme)
        .with_prompt("Enter the directory to store the database in")
        .default(
            default_database_dir.parent()
                .ok_or(eyre!("Default config directory has no parent"))?
                // TODO: Not to do this :^).
                .to_string_lossy()
                .to_string(),
        )
        .allow_empty(false)
        .validate_with(|dir: &String| -> std::result::Result<(), &str> {
            let dir = Path::new(dir);

            if dir.try_exists().is_ok_and(|exists| exists == false) {
                return Err("Directory does not exist")
            }
            if !dir.is_dir() {
                return Err("Entered path is not a directory");
            }

            match fs::read_dir(&dir).map_err(|err| err.kind()) {
                Ok(_) => (),
                Err(err) => return match err {
                    ErrorKind::PermissionDenied => Err("The current process does not have permission to read from this directory"),
                    _ => Err("There was a problem when checking this directory")
                }
            }


           Ok(())
        })
        .interact_text()
        .wrap_err("Failed to get path")?;*/

        #[cfg(feature = "web")]
        let port = dialoguer::Input::<u16>::with_theme(&theme)
            .with_prompt("Enter the port number for the server")
            .default(56423)
            .validate_with(|port: &u16| {
                if 0 < *port && *port < u16::MAX {
                    Ok(())
                } else {
                    Err("Not a valid port number")
                }
            })
            .allow_empty(false)
            .interact_text()
            .wrap_err("Failed to get port number")?;

        let config = Config {
            path: PathBuf::from(db_path),
            #[cfg(feature = "web")]
            port,
        };

        Self::init(&path, &config).wrap_err(
            "Failed to initialise configuration file after interactively getting config",
        )?;

        Ok(config)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Database {
    pub logins: HashMap<Uuid, Login>,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct Login {
    pub name: String,
    pub username: String,
    pub password: String,
}

impl Display for Login {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Login")
            .field("name", &self.name)
            .field("username", &self.username)
            .finish()
    }
}

impl AsRef<str> for Login {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl Login {
    pub fn new(name: String, username: String, password: String) -> Self {
        Self {
            name,
            username,
            password,
        }
    }
}

// A tuple struct which simply allows us to have custom `Deref` behaviour on a `(&Uuid, &Login)`.
// We need this because of how nucleo works.
struct LoginAndId<'a>(&'a Uuid, &'a Login);

impl<'a> From<(&'a Uuid, &'a Login)> for LoginAndId<'a> {
    fn from(value: (&'a Uuid, &'a Login)) -> Self {
        Self(value.0, value.1)
    }
}

impl<'a> AsRef<str> for LoginAndId<'a> {
    fn as_ref(&self) -> &str {
        &self.1.name
    }
}

// FIXME: Don't `clone()` the path when it's passed to `open()` or `init()`
impl Database {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        PathBuf: From<P>,
    {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|err| err.kind());

        if let Err(io::ErrorKind::AlreadyExists) = f {
            // TODO: Colour output.
            eprintln!("[-] ERROR: A database already exists in the target location, so you cannot initialise a new one there");
            std::process::exit(1);
        } else if let Err(err) = f {
            bail!("Failed to initialise new database file: {}", err)
        }

        Ok(Self {
            logins: HashMap::new(),
            path: PathBuf::from(path),
        })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        PathBuf: From<P>,
    {
        let mut f = File::open(&path).map_err(|err| err.kind());

        if let Err(err) = &f {
            // TODO, look at whether this is actually a good idea or not.
            if *err == io::ErrorKind::NotFound {
                return Self::init(path)
                    .wrap_err("Failed to initialise new database in `Database::open`");
            }
            bail!("Failed to open existing database: {err}")
        }

        let reader = BufReader::new(f.as_ref().unwrap());
        let is_empty = fs::metadata(&path)
            .wrap_err("Failed to get metadata of existing database")?
            .len()
            == 0;
        let mut db = if is_empty {
            Self::default()
        } else {
            rmp_serde::decode::from_read(reader).wrap_err("Failed to parse existing database")?
        };
        db.path = PathBuf::from(path);

        Ok(db)
    }

    pub fn add(&mut self, login: Login) {
        let id = Uuid::new_v4();
        // TODO: However unlikely it is that there will be a collision, do proper things here.
        let old_val = self.logins.insert(id, login);
        assert!(old_val.is_none());
    }

    pub fn add_new_interactive(&mut self) -> Result<()> {
        let theme = dialoguer::theme::ColorfulTheme::default();

        let name = Input::<String>::with_theme(&theme)
            .with_prompt("Enter the name for the login")
            .interact_text()
            .wrap_err("Failed to read name from console")?;

        let username = Input::<String>::with_theme(&theme)
            .with_prompt("Enter the username for this login")
            .interact_text()
            .wrap_err("Failed to read username from console")?;

        let password = Password::with_theme(&theme)
            .with_prompt("Enter the password for this login")
            .interact()
            .wrap_err("Failed to read password from console")?;

        let new_login = Login::new(name, username, password);
        self.add(new_login);
        Ok(())
    }

    // TODO: Find a way to use a slice here.
    pub fn append(&mut self, logins: Vec<Login>) {
        for login in logins {
            self.add(login);
        }
    }

    #[cfg(feature = "paralell_queries")]
    pub fn query(&self, name: Option<&str>) -> Vec<(&Uuid, &Login)> {
        use nucleo_matcher::{
            pattern::{CaseMatching, Pattern},
            Matcher,
        };

        if self.logins.is_empty() {
            return Vec::new();
        }

        let Some(name) = name else {
            // TODO: Find out if this requires allocation.
            return self.logins.iter().collect();
        };
        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
        let iter: Vec<LoginAndId> = self.logins.iter().map(|tuple| tuple.into()).collect();
        let matches: Vec<(&Uuid, &Login)> = Pattern::parse(name, CaseMatching::Ignore)
            .match_list(iter, &mut matcher)
            .par_iter()
            .map(|(login, _)| login)
            .map(|login| (login.0, login.1))
            .collect();

        if !matches.is_empty() {
            return matches;
        }

        Vec::new()
    }

    #[cfg(not(feature = "paralell_queries"))]
    pub fn query(&self, name: &str) -> Vec<&Login> {
        if self.logins.is_empty() {
            return Vec::new();
        }
        let matcher = SkimMatcherV2::default();

        let matches = self
            .logins
            .iter()
            .map(|login| (login, matcher.fuzzy_match(&login.name, name)))
            .filter(|login| login.1.is_some())
            .sorted_by_key(|login| login.1)
            .rev()
            .map(|login| login.0);
        if matches.len() != 0 {
            return matches.collect::<Vec<&Login>>();
        }

        Vec::new()
    }

    pub fn query_interactive(&mut self, name: Option<&str>) {
        if self.logins.is_empty() {
            let data = TableValue::Cell(String::from("No records"));

            println!(
                "{table}",
                table = PoolTable::from(data).with(Style::rounded())
            );
            return;
        }

        if let Some(name) = name {
            // Fix?
            let matches: Vec<&Login> = self
                .query(Some(name))
                .iter()
                .map(|(_, login)| *login)
                .collect();
            if matches.is_empty() {
                let data = TableValue::Cell(String::from("No records"));

                println!(
                    "{table}",
                    table = PoolTable::from(data).with(Style::rounded())
                );
                return;
            }
            println!("{}", Table::new(matches).with(Style::rounded()));
        } else {
            println!(
                "{}",
                Table::new(self.logins.values()).with(Style::rounded())
            );
        }
    }

    pub fn remove(&mut self, id: Uuid) -> Option<Login> {
        self.logins.remove(&id)
    }

    pub fn remove_interactive(&mut self) -> Result<Option<Login>> {
        let options: Vec<_> = self.logins.iter().collect();
        let choice = FuzzySelect::with_theme(&ColorfulTheme::default())
            .items(
                options
                    .iter()
                    .map(|(_, login)| login)
                    .collect::<Vec<&&Login>>()
                    .as_slice(),
            )
            .interact_opt()
            .wrap_err("Failed to read choice of login to be removed from console")?;

        if let Some(index) = choice {
            let id = *options[index].0;
            return Ok(self.logins.remove(&id));
        }

        Ok(None)
    }

    pub fn sync(&self) -> Result<()> {
        let f = OpenOptions::new()
            .write(true)
            .truncate(true)
            .read(false)
            .open(&self.path)
            .wrap_err("Failed to open the database file for sync")?;
        let mut writer = BufWriter::new(f);

        let doc = rmp_serde::encode::to_vec(&self).wrap_err("Failed to serialise the database")?;
        writer
            .write_all(&doc)
            .wrap_err("Failed to write the database to disk")?;

        Ok(())
    }
}
