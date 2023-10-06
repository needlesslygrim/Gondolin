use std::collections::HashMap;
use std::fmt::Display;
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read},
};

use color_eyre::eyre::{bail, Context, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{FuzzySelect, Input, Password};
use ron::ser::PrettyConfig;
use serde_derive::{Deserialize, Serialize};
use tabled::{
    settings::Style,
    tables::{PoolTable, TableValue},
    Table, Tabled,
};
use uuid::Uuid;

#[cfg(feature = "paralell_queries")]
use rayon::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub logins: HashMap<Uuid, Login>,
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

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let mut f = File::open(path).map_err(|err| err.kind());

        if let Err(err) = &f {
            if *err == io::ErrorKind::NotFound {
                return Self::init(path)
                    .wrap_err("Failed to initialise new database in `Database::open`");
            }
            bail!("Failed to open existing database: {err}")
        }

        let mut reader = BufReader::new(f.as_ref().unwrap());
        let mut contents = String::new();
        reader
            .read_to_string(&mut contents)
            .wrap_err("Failed to read existing database")?;

        // less ugly than before.
        let db = if contents.is_empty() {
            Self {
                logins: HashMap::new(),
            }
        } else {
            ron::from_str::<Database>(&contents).wrap_err("Failed to parse existing database")?
        };

        Ok(db)
    }

    pub fn init(path: &str) -> Result<Self> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
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
        })
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

    pub fn sync(&self, path: &str) -> Result<()> {
        let f = OpenOptions::new()
            .write(true)
            .truncate(true)
            .read(false)
            .open(path)
            .wrap_err("Failed to open the database file for sync")?;
        let writer = BufWriter::new(f);

        ron::ser::to_writer_pretty(
            writer,
            &self,
            PrettyConfig::default()
                .indentor("\t".to_string())
                .struct_names(true),
        )
        .wrap_err("Failed to sync the database to disk")?;

        Ok(())
    }
}
