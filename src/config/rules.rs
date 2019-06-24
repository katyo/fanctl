use serde::{Serialize, Deserialize};
use serde_yaml::Value;
use std::collections::LinkedList;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleBinding {
    pub outputs: LinkedList<String>,
    pub rule: Rule,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub ty: RuleType,
    pub config: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RuleType {
    Static,
    Maximum,
    GateCritical,
    GateStatic,
    Curve,
}

/// Rule type for gating on a static value from an input.
///
/// This rule will output 0.0 if the input is below the threshold, but `value` if it is not
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GateStatic {
    /// Input name to source data from
    pub input: String,
    /// Threshold to use when checking whether to output `value` or not
    pub threshold: f64,
    /// Value to output when the input exceeds the threshold
    pub value: f64,
}

/// Rule for generating an interpolated fan curve from a set of input anchors
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    /// Input name to source data from
    pub input: String,
    /// List of input/output value pairs for generating the fan curve
    pub keys: Vec<CurvePoint>,
    /// Default value to output if the input is outside the range of `keys`
    ///
    /// default: `0.0`
    pub out_of_bounds_value: Option<f64>,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct CurvePoint {
    pub input: f64,
    pub output: f64,
}
