use serde::{Serialize, Deserialize};
use std::{
    collections::LinkedList,
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleBinding {
    pub outputs: LinkedList<String>,
    pub rule: Rule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Rule {
    Static(f64),
    Maximum(Vec<Box<Rule>>),
    GateCritical {
        input: String,
        value: f64,
    },
    GateStatic {
        input: String,
        threshold: f64,
        value: f64,
    },
    Curve {
        input: String,
        keys: Vec<CurvePoint>,
        out_of_bounds_value: Option<f64>,
    },
    Smooth {
        rule: Box<Rule>,
        samples: usize
    },
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct CurvePoint {
    pub input: f64,
    pub output: f64,
}
