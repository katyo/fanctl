use serde::{Serialize, Deserialize};
use std::collections::{HashMap, LinkedList};

use super::hwmon;

mod rules;
pub mod error;
pub use error::{
    ConfigError,
    read_config,
    read_config_yaml,
};
pub use rules::*;
use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HwmonSensor {
    Path { path: PathBuf },
    Search { hwmon: String, label: String },
}

impl HwmonSensor {
    pub fn path(&self) -> PathBuf {
        match self {
            HwmonSensor::Path { path } => path.clone(),
            HwmonSensor::Search { hwmon, label } => hwmon::search_input(hwmon, label)
                .expect("Error while searching hwmon input")
                .expect(&format!{"No hwmon match found for '{}/{}'", hwmon, label}),
        }
    }
}

impl<'a> Into<Box<dyn hwmon::Sensor>> for &'a Input {
    fn into(self) -> Box<dyn hwmon::Sensor> {
        match self {
            Input::HwmonSensor(sensor) => Box::new(hwmon::HwmonSensor::new(sensor.path())),
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
