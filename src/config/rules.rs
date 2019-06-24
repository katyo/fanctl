use serde::{Serialize, Deserialize};
use serde_yaml::Value;
use std::collections::LinkedList;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleBinding {
    pub(crate) outputs: LinkedList<String>,
    pub(crate) rule: Rule,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub(crate) ty: RuleType,
    pub(crate) config: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RuleType {
    Static,
    Maximum,
    GateCritical,
    GateStatic,
    Curve,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GateStatic {
    pub(crate) input: String,
    pub(crate) threshold: f64,
    pub(crate) value: f64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub(crate) input: String,
    pub(crate) keys: Vec<CurvePoint>,
    pub(crate) out_of_bounds_value: Option<f64>,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct CurvePoint {
    pub(crate) input: f64,
    pub(crate) output: f64,
}
