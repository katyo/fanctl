use serde::{Serialize, Deserialize};
use serde_yaml::Value;
use std::collections::{HashMap, LinkedList};
use std::convert::TryFrom;

use super::hwmon;

mod rules;
pub use rules::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub(crate) inputs: HashMap<String, Input>,
    pub(crate) outputs: HashMap<String, Output>,
    pub(crate) rules: LinkedList<RuleBinding>,
    pub(crate) interval: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Input {
    ty: InputType,
    args: Value,
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
    ty: OutputType,
    args: serde_yaml::Value,
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
    pub(crate) path: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmdgpuFan {
    pub(crate) path: String,
    pub(crate) prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwmonSensor {
    pub(crate) path: String,
}
