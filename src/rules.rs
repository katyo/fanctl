use serde_yaml::Value;
use std::convert::TryFrom;
use std::io;
use super::{config, hwmon};
use std::collections::{HashMap, LinkedList};
use std::cmp::PartialOrd;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum RuleParseError {
    MissingArg(&'static str),
    WrongType(&'static str, Value),
    UnknownRuleType(String),
    UnknownInput(String),
}

pub trait Rule {
    fn get_value(&self) -> io::Result<f64>;
}

pub fn instantiate_rule(rule: &config::Rule, inputs: &HashMap<String, Rc<hwmon::Sensor>>) -> Result<Box<Rule>, RuleParseError> {
    use config::RuleType;
    let ret = match rule.ty {
        RuleType::Static => Static::try_from(rule.config.as_ref().unwrap()).map(|r| Box::new(r) as Box<Rule>),
        RuleType::Maximum => Maximum::try_from((rule.config.as_ref().unwrap(), inputs)).map(|r| Box::new(r) as Box<Rule>),
        RuleType::GateCritical => GateCritical::new(rule.config.as_ref().unwrap(), inputs).map(|r| Box::new(r) as Box<Rule>),
    };
    ret
}

fn parse_rule_type<S: AsRef<str>>(ty: S) -> Result<config::RuleType, RuleParseError> {
    use config::RuleType;
    let ty = ty.as_ref();
    match ty {
        "Static" => Ok(RuleType::Static),
        "Maximum" => Ok(RuleType::Maximum),
        "GateCritical" => Ok(RuleType::GateCritical),
        other => Err(RuleParseError::UnknownRuleType(other.to_string())),
    }
}

pub struct Static(f64);

impl TryFrom<&Value> for Static {
    type Error = RuleParseError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        if let &Value::Number(ref num) = value {
            num.as_f64()
                .map(|v| Ok(Static(v)))
                .unwrap_or(Err(RuleParseError::WrongType("Number", value.clone())))
        } else {
            Err(RuleParseError::WrongType("Number", value.clone()))
        }
    }
}

impl Rule for Static {
    fn get_value(&self) -> io::Result<f64> {
        Ok(self.0)
    }
}

pub struct Maximum {
    rules: LinkedList<Box<Rule>>,
}

impl TryFrom<(&Value, &HashMap<String, Rc<hwmon::Sensor>>)> for Maximum {
    type Error = RuleParseError;
    fn try_from(value: (&Value, &HashMap<String, Rc<hwmon::Sensor>>)) -> Result<Self, Self::Error> {
        let (value, inputs) = value;
        if let &Value::Sequence(ref arr) = value {
            let mut rules = LinkedList::new();
            let ty_key = Value::String("ty".to_string());
            let config_key = Value::String("config".to_string());
            for rule_value in arr {
                if let &Value::Mapping(ref m) = rule_value {
                    let ty = m.get(&ty_key)
                        .map(|v| match v {
                            &Value::String(ref s) => parse_rule_type(s),
                            v => Err(RuleParseError::WrongType("String", v.clone())),
                        })
                        .unwrap_or(Err(RuleParseError::MissingArg("ty")))?;
                    let config = m.get(&config_key).map(Clone::clone);
                    let rule = instantiate_rule(&config::Rule {
                        ty: ty,
                        config: config,
                    }, inputs)?;
                    rules.push_back(rule);
                } else {
                    return Err(RuleParseError::WrongType("Mapping", rule_value.clone()))
                }
            }
            Ok(Maximum {
                rules: rules
            })
        } else {
            Err(RuleParseError::WrongType("Sequence", value.clone()))
        }
    }
}

fn partial_max<V: PartialOrd + Copy>(fst: V, snd: V) -> V {
    if fst.ge(&snd) {
        fst
    } else {
        snd
    }
}

impl Rule for Maximum {
    fn get_value(&self) -> io::Result<f64> {
        let mut max = None;
        for rule in &self.rules {
            let value = rule.get_value()?;
            if let Some(current_max) = max {
                max = Some(partial_max(current_max, value));
            } else {
                max = Some(value);
            }
        }
        max.map(Ok)
            .unwrap_or(Err(io::Error::new(io::ErrorKind::Other, "No inputs available for \"Maximum\" rule")))
    }
}

trait MappingExt {
    fn get_string(&self, key: &Value) -> Option<String>;
    fn get_f64(&self, key: &Value) -> Option<f64>;
}

impl MappingExt for serde_yaml::Mapping {
    fn get_string(&self, key: &Value) -> Option<String> {
        self.get(key)
            .and_then(|v| match v {
                &Value::String(ref s) => Some(s.clone()),
                _ => None,
            })
    }

    fn get_f64(&self, key: &Value) -> Option<f64> {
        self.get(key)
            .and_then(|v| match v {
                &Value::Number(ref n) => n.as_f64(),
                _ => None,
            })
    }
}

pub struct GateCritical {
    input: Rc<hwmon::Sensor>,
    value: f64,
}

impl GateCritical {
    fn new(config: &Value, inputs: &HashMap<String, Rc<hwmon::Sensor>>) -> Result<GateCritical, RuleParseError> {
        if let &Value::Mapping(ref m) = config {
            let input_key = Value::String("input".to_string());
            let value_key = Value::String("value".to_string());
            let input = m.get_string(&input_key)
                .map(Ok)
                .unwrap_or(Err(RuleParseError::MissingArg("input")))
                .and_then(|input_name| {
                    inputs.get(&input_name)
                        .map(Ok)
                        .unwrap_or_else(move || Err(RuleParseError::UnknownInput(input_name)))
                })
                .map(Clone::clone)?;
            let value = m.get_f64(&value_key)
                .map(Ok)
                .unwrap_or(Err(RuleParseError::MissingArg("value")))?;
            Ok(GateCritical {
                input: input,
                value: value,
            })
        } else {
            Err(RuleParseError::WrongType("Mapping", config.clone()))
        }
    }
}

impl Rule for GateCritical {
    fn get_value(&self) -> io::Result<f64> {
        let threshold = self.input.get_critical()?;
        let value = self.input.get_value()?;
        if value >= threshold {
            Ok(self.value)
        } else {
            Ok(0.0)
        }
    }
}
