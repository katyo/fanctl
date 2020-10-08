use serde::de::DeserializeOwned;
use std::{
    error::Error,
    fs,
    io,
    path::Path,
};

/// Error that may result from parsing a config file
#[derive(Debug, thiserror::Error)]
pub enum ConfigError<ParseError: Error> {
    #[error("Error reading config")]
    Io(#[from] io::Error),
    #[error("Error parsing config")]
    Parse(ParseError),
}

impl From<serde_yaml::Error> for ConfigError<serde_yaml::Error> {
    #[inline]
    fn from(e: serde_yaml::Error) -> Self {
        ConfigError::Parse(e)
    }
}

pub fn read_config<F, Config, ParseError, P>(p: P, parse: F) -> Result<Config, ConfigError<ParseError>> where
    F: FnOnce(fs::File) -> Result<Config, ParseError>,
    P: AsRef<Path>,
    ParseError: Error,
{
    let file = fs::OpenOptions::new()
        .read(true)
        .open(p)
        .map_err(ConfigError::from)?;
    parse(file)
        .map_err(ConfigError::Parse)
}

pub fn read_config_yaml<Config, P>(p: P) -> Result<Config, ConfigError<serde_yaml::Error>> where
    P: AsRef<Path>,
    Config: DeserializeOwned,
{
    read_config(p, serde_yaml::from_reader)
}
