use serde::{Serialize, Deserialize};
use std::collections::{HashMap, LinkedList};

use super::hwmon;

mod rules;
pub use rules::*;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    HwmonSensor {
        path: String,
    },
}

impl<'a> Into<Box<dyn hwmon::Sensor>> for &'a Input {
    fn into(self) -> Box<dyn hwmon::Sensor> {
        match self {
            Input::HwmonSensor { path } => Box::new(hwmon::HwmonSensor::new(path.clone())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Output {
    PwmFan {
        path: String,
        name: String,
    },
    AmdgpuFan {
        path: String,
        prefix: String,
    },
}

impl<'a> Into<Box<dyn hwmon::Fan>> for &'a Output {
    fn into(self) -> Box<dyn hwmon::Fan> {
        match self {
            &Output::PwmFan { ref path, ref name } => {
                let fan = hwmon::PwmFan::new(path.clone(), name.clone())
                    .expect("Failed to create PwmFan");
                Box::new(fan)
            },
            &Output::AmdgpuFan { ref path, ref prefix } => {
                Box::new(hwmon::amdgpu::AmdgpuFan::new(path, prefix))
            },
        }
    }
}
