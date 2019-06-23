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
    GateCritical
}
