use thiserror::Error;

#[derive(Debug, Copy, Clone, Error)]
pub enum GondolinError {
    #[error("Tried to initialise a configuration file where one already exists")]
    ConfigAlreadyExistsError,
    #[error("Tried to initialise a database file where one already exists")]
    DatabaseAlreadyExistsError,
}
