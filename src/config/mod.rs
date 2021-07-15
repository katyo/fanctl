use serde::{Serialize, Deserialize};

use super::hwmon;

mod rules;
pub mod error;
pub use error::{
    ConfigError,
    read_config,
    read_config_yaml,
};
pub use rules::*;
use std::{
    collections::{
        HashMap,
        LinkedList,
    },
    convert::{
        TryInto,
    },
    io,
    path::PathBuf,
};

/// Root config struct created from the config file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Map of input names to input info
    pub inputs: HashMap<String, Input>,
    /// Map of output names to output info
    pub outputs: HashMap<String, Output>,
    /// List of rules, and the outputs that they should apply to
    pub rules: LinkedList<RuleBinding>,
    /// Interval, in milliseconds, to wait between iterations
    pub interval: u64,
    /// Log interval, in number of `iterations`
    pub log_iterations: Option<usize>,
}

/// Defines an input sensor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    /// Standard hwmon input sensor
    HwmonSensor(HwmonSensor),
}

#[derive(Debug, thiserror::Error)]
pub enum FindHwmonError {
    #[error("Error searching for hwmon component: {0}")]
    Io(#[from] io::Error),
    #[error("No hwmon component found for search: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HwmonSensor {
    Path { path: PathBuf },
    Search { hwmon: String, label: String },
}

impl HwmonSensor {
    pub fn path(&self) -> Result<PathBuf, FindHwmonError> {
        match self {
            HwmonSensor::Path { path } => {
                use crate::path_ext::PathExt;
                path.expand_wildcards().map_err(FindHwmonError::from)
            },
            HwmonSensor::Search { hwmon, label } => hwmon::search_input(hwmon, label)
                .map_err(FindHwmonError::from)
                .and_then(|v| v.map(Ok).unwrap_or_else(|| Err(FindHwmonError::NotFound(format!("{}/{}", hwmon, label))))),
        }
    }
}

impl<'a> TryInto<Box<dyn hwmon::Sensor>> for &'a Input {
    type Error = FindHwmonError;

    fn try_into(self) -> Result<Box<dyn hwmon::Sensor>, Self::Error> {
        match self {
            Input::HwmonSensor(sensor) => {
                let ret = sensor.path()
                    .map(hwmon::HwmonSensor::new)?;
                Ok(Box::new(ret))
            },
        }
    }
}

/// Defines an output fan
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Output {
    /// Standard `hwmon` pwm fan
    PwmFan {
        #[serde(flatten)]
        hwmon: FanHwmon,
        name: String,
    },
    /// AMDGPU fuzzy fan
    AmdgpuFan {
        #[serde(flatten)]
        hwmon: FanHwmon,
        prefix: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FanHwmon {
    Path { path: PathBuf },
    Search { hwmon: String },
}

impl FanHwmon {
    pub fn path(&self) -> PathBuf {
        match self {
            FanHwmon::Path { path } => path.clone(),
            FanHwmon::Search { hwmon } => hwmon::search_hwmon(hwmon)
                .expect("Error while searching hwmon")
                .expect(&format!{"No hwmon match found for '{}'", hwmon}),
        }
    }
}

impl<'a> Into<Box<dyn hwmon::Fan>> for &'a Output {
    fn into(self) -> Box<dyn hwmon::Fan> {
        match self {
            &Output::PwmFan { ref hwmon, ref name } => {
                let fan = hwmon::PwmFan::new(hwmon.path(), name.clone())
                    .expect("Failed to create PwmFan");
                Box::new(fan)
            },
            &Output::AmdgpuFan { ref hwmon, ref prefix } => {
                Box::new(hwmon::amdgpu::AmdgpuFan::new(hwmon.path(), prefix))
            },
        }
    }
}
