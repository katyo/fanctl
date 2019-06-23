use serde::{Serialize, Deserialize};
use serde_yaml::Value;
use std::collections::HashMap;

use super::hwmon;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub inputs: HashMap<String, Input>,
    pub outputs: HashMap<String, Output>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Input {
    ty: InputType,
    args: HashMap<String, Value>,
}

impl Input {
    fn get_required(&self, arg: &str) -> &Value {
        self.args.get(arg)
            .expect(&format!("failed to get required arg: {}", arg))
    }

    pub fn initialize(&self) -> Box<hwmon::Sensor> {
        match self.ty {
            InputType::HwmonSensor => {
                let path = if let Value::String(ref s) = self.get_required("path") {
                    s.clone()
                } else {
                    panic!("expected arg \"path\" to be a String");
                };
                Box::new(hwmon::HwmonSensor::new(path))
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
    args: HashMap<String, Value>,
}

impl Output {
    fn get_required(&self, arg: &str) -> &Value {
        self.args.get(arg)
            .expect(&format!("failed to get required arg: {}", arg))
    }

    pub fn initialize(&self) -> Box<hwmon::Fan> {
        match self.ty {
            OutputType::AmdgpuFan => {
                let path = if let Value::String(ref s) = self.get_required("path") {
                    s
                } else {
                    panic!("expected argument \"path\" to be a String");
                };
                let prefix = if let Value::String(ref s) = self.get_required("prefix") {
                    s
                } else {
                    panic!("expected argument \"prefix\" to be a String");
                };
                Box::new(hwmon::amdgpu::AmdgpuFan::new(path, prefix))
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputType {
    AmdgpuFan
}
