use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read},
};

use color_eyre::eyre::{bail, Context, Result};
use dialoguer::{Input, Password};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use ron::ser::PrettyConfig;
use serde_derive::{Deserialize, Serialize};
use tabled::{
    builder,
    settings::Style,
    tables::{PoolTable, TableValue},
    Table, Tabled,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub logins: Vec<Login>,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct Login {
    pub name: String,
    pub username: String,
    pub password: String,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let mut f = File::open(path).map_err(|err| err.kind());

        if let Err(err) = &f {
            if *err == io::ErrorKind::NotFound {
                return Self::init(path)
                    .wrap_err("Failed to initialise new database in `Database::open`");
            } else {
                bail!("Failed to open existing database: {err}")
            }
        }

        let mut reader = BufReader::new(f.as_ref().unwrap());
        let mut contents = String::new();
        reader
            .read_to_string(&mut contents)
            .wrap_err("Failed to read existing database")?;

        // ugly.
        let db = if contents.is_empty() {
            Self { logins: Vec::new() }
        } else {
            ron::from_str::<Database>(&contents).wrap_err("Failed to parse existing database")?
        };

        Ok(db)
    }

    pub fn init(path: &str) -> Result<Self> {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|err| err.kind());

        if let Err(io::ErrorKind::AlreadyExists) = f {
            bail!("Failed to create new database, as one already exists");
        } else if let Err(err) = f {
            bail!("Failed to initialise new database file: {}", err)
        }

        Ok(Self { logins: Vec::new() })
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

        self.logins.push(Login {
            name,
            username,
            password,
        });

        Ok(())
    }

    pub fn query(&self, name: Option<&String>) {
        if self.logins.is_empty() {
            let data = TableValue::Cell(String::from("No records"));

            println!(
                "{table}",
                table = PoolTable::from(data).with(Style::rounded())
            );
            return;
        }

        if let Some(name) = name {
            let matcher = SkimMatcherV2::default();

            let logins = self
                .logins
                .iter()
                .map(|login| (login, matcher.fuzzy_match(&login.name, name)))
                .filter(|login| login.1.is_some())
                .sorted_by_key(|login| login.1)
                .rev()
                .map(|login| login.0);
            if logins.len() == 0 {
                let data = TableValue::Cell(String::from("No records"));

                println!(
                    "{table}",
                    table = PoolTable::from(data).with(Style::rounded())
                );
                return;
            }
            println!("{}", Table::new(logins).with(Style::rounded()))
        } else {
            println!("{}", Table::new(self.logins.iter()).with(Style::rounded()));
        }
    }

    pub fn sync(self, path: &str) -> Result<()> {
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
