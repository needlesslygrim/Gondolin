use std::io::ErrorKind;
use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    fs::{File, OpenOptions},
    io::{prelude::*, BufReader, BufWriter},
    path::{Path, PathBuf},
};

use color_eyre::eyre::{bail, Context, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{FuzzySelect, Input, Password};
use itertools::Itertools;
use serde_derive::{Deserialize, Serialize};
use tabled::{
    settings::Style,
    tables::{PoolTable, TableValue},
    Table, Tabled,
};
use uuid::Uuid;

use crate::errors::GondolinError;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub path: PathBuf,
    #[cfg(feature = "web")]
    pub port: u16,
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

impl Config {
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

    pub(crate) fn init_interactive(path: &Path, db_path: &Path, port: Option<u16>) -> Result<Self> {
        if let Some(port) = port {
            let config = Config {
                path: PathBuf::from(db_path),
                #[cfg(feature = "web")]
                port,
            };
            Self::init(path, &config).wrap_err(
                "Failed to initialise configuration file after interactively getting config",
            )?;

            return Ok(config);
        }

        let theme = ColorfulTheme::default();

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

        Self::init(path, &config).wrap_err(
            "Failed to initialise configuration file after interactively getting config",
        )?;

        Ok(config)
    }

    fn open(path: &Path) -> Result<Self> {
        let f = File::open(path).wrap_err("Failed to open file handle to configuration file")?;
        let mut reader = BufReader::new(f);
        let mut buf = String::with_capacity(
            usize::try_from(
                fs::metadata(path)
                    .wrap_err("Failed to get metadata of configuration file")?
                    .len(),
            )
            .unwrap_or_default(),
        );
        reader
            .read_to_string(&mut buf)
            .wrap_err("Failed to read configuration file from disk")?;

        toml::de::from_str(&buf).wrap_err("Failed to parse configuration file")
    }

    pub(crate) fn open_interactive(path: &Path) -> Result<Self> {
        if !path
            .try_exists()
            .wrap_err("Failed to check whether the database exists")?
        {
            eprintln!("You have not initialised Gondolin yet, please run `gondolin init` to initialise, then run this command again.");
            std::process::exit(0);
        }

        Self::open(path).wrap_err("Failed to load configuration from disk")
    }
}

impl Database {
    pub fn init(path: &Path) -> Result<Self> {
        // Discard the file descriptor because we don't need to actually write to the file on
        // initialisation, we only need to create the file. Ideally there would be an
        // `fs::create_file()`, but there is not.
        if let Err(err) = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
        {
            match err.kind() {
                ErrorKind::AlreadyExists => {
                    bail!(crate::errors::GondolinError::DatabaseAlreadyExistsError)
                }
                _ => bail!("Failed to create a new database file: {err}"),
            };
        }

        Ok(Self {
            logins: HashMap::new(),
            path: PathBuf::from(path),
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        let reader =
            BufReader::new(File::open(path).wrap_err("Failed to open file handle to database")?);
        let is_empty = match fs::metadata(path) {
            Ok(meta) => meta.len(),
            Err(err) => match err.kind() {
                ErrorKind::NotFound => 0,
                _ => Err(err).wrap_err("Failed to get metadata of configuration file")?,
            },
        } == 0;

        let mut db = if is_empty {
            Self::default()
        } else {
            rmp_serde::decode::from_read(reader).wrap_err("Failed to parse database contents")?
        };
        db.path = PathBuf::from(path);

        Ok(db)
    }

    pub fn add_login(&mut self, login: Login) {
        let id = Uuid::new_v4();
        // TODO: However unlikely it is that there will be a collision, do proper things here.
        let old_val = self.logins.insert(id, login);
        assert!(old_val.is_none());
    }

    pub(crate) fn add_login_interactive(&mut self) -> Result<()> {
        let theme = ColorfulTheme::default();

        let name = Input::<String>::with_theme(&theme)
            .with_prompt("Enter the name for the login")
            .allow_empty(true)
            .interact_text()
            .wrap_err("Failed to read name from console")?;

        let username = Input::<String>::with_theme(&theme)
            .with_prompt("Enter the username for this login")
            .allow_empty(true)
            .interact_text()
            .wrap_err("Failed to read username from console")?;

        let password = Password::with_theme(&theme)
            .with_prompt("Enter the password for this login")
            .allow_empty_password(true)
            .interact()
            .wrap_err("Failed to read password from console")?;

        let new_login = Login::new(name, username, password);
        self.add_login(new_login);
        Ok(())
    }

    pub fn append_logins(&mut self, logins: Vec<Login>) {
        for login in logins {
            self.add_login(login);
        }
    }

    pub fn query(&self, name: Option<&str>) -> Vec<(&Uuid, &Login)> {
        use nucleo_matcher::{
            pattern::{CaseMatching, Pattern},
            Matcher,
        };

        if self.logins.is_empty() {
            return Vec::new();
        }
        let Some(name) = name else {
            return self.logins.iter().collect();
        };
        if name.is_empty() {
            return self.logins.iter().collect();
        }

        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
        let logins: Vec<LoginAndId> = self.logins.iter().map_into().collect();

        Pattern::parse(name, CaseMatching::Ignore)
            .match_list(logins, &mut matcher)
            .into_iter()
            .map(|(login, _)| login)
            .map(|login| (login.0, login.1))
            .collect()
    }

    pub(crate) fn query_interactive(&mut self, name: Option<&str>) {
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

    pub(crate) fn remove_interactive(&mut self) -> Result<Option<Login>> {
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
