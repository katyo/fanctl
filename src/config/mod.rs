use serde::{Serialize, Deserialize};
use serde_yaml::Value;
use std::collections::{HashMap, LinkedList};
use std::convert::TryFrom;

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
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Input {
    pub ty: InputType,
    pub args: Value,
}

impl Input {
    pub fn initialize(&self) -> Box<hwmon::Sensor> {
        match self.ty {
            InputType::HwmonSensor => {
                let sensor = hwmon::HwmonSensor::try_from(self.args.clone())
                    .expect(&format!("failed to parse configuration for {:?}", self.ty));
                Box::new(sensor)
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputType {
    HwmonSensor
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Output {
    pub ty: OutputType,
    pub args: serde_yaml::Value,
}

impl Output {
    pub fn initialize(&self) -> Box<hwmon::Fan> {
        match self.ty {
            OutputType::AmdgpuFan => {
                let fan = hwmon::amdgpu::AmdgpuFan::try_from(self.args.clone())
                    .expect(&format!("failed to parse configuration for {:?}", self.ty));
                Box::new(fan)
            },
            OutputType::PwmFan => {
                let fan = hwmon::PwmFan::try_from(self.args.clone())
                    .expect(&format!("failed to parse configuation for {:?}", self.ty));
                Box::new(fan)
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputType {
    AmdgpuFan,
    PwmFan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwmFan {
    /// Path to the containing `hwmon` directory.
    ///
    /// EX: `/sys/class/hwmon/hwmon0`
    pub path: String,
    /// Prefix for this fan
    ///
    /// EX: `pwm1` to use `pwm`, and `pwm1_enable` files.
    pub name: String,
}

/// Fan type to use the on-board can speed controller from the
/// `amdgpu` kernel driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmdgpuFan {
    /// Path to the containing `hwmon` directory.
    ///
    /// EX: `/sys/class/drm/card0/device/hwmon/hwmon0`
    pub path: String,
    /// Prefix for this fan instance
    ///
    /// EX: `fan1` to use `fan1_enable`, `fan1_crit`, etc.
    pub prefix: String,
}

/// Configuration arguments for the `HwmonSensor` type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwmonSensor {
    /// Path to the input file (ex: `/sys/class/hwmon/hwmon0/temp1_input`)
    pub path: String,
}
