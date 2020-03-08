use serde_yaml::Value;
use std::convert::TryFrom;
use std::io;
use super::{config, hwmon};
use std::collections::{HashMap, LinkedList};
use std::cmp::PartialOrd;
use std::rc::Rc;
use splines::Spline;
use crate::config::{
    RuleBinding,
    Rule as RuleConfig,
    CurvePoint,
};

pub trait Rule {
    fn get_value(&self) -> io::Result<f64>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleConfigError {
    UnknownInput(String),
    UnknownOutput(String),
}

pub fn rule_from_config<F>(config: &RuleConfig, get_input: F) -> Result<Box<dyn Rule>, RuleConfigError>
where
    F: Fn(&String) -> Option<Rc<hwmon::Sensor>> + Copy,
{
    let ret: Box<dyn Rule> = match config {
        &RuleConfig::Static(v) => Box::new(Static(v)),
        &RuleConfig::Maximum(ref rules) => {
            let mut rs = LinkedList::new();
            for rule in rules.iter() {
                let r = rule_from_config(rule, get_input)?;
                rs.push_back(r);
            }
            Box::new(Maximum {
                rules: rs,
            })
        },
        &RuleConfig::GateCritical { ref input, value } => {
            let input = get_input(input)
                .map(Ok)
                .unwrap_or_else(|| Err(RuleConfigError::UnknownInput(input.clone())))?;
            Box::new(GateCritical::new(input, value))
        },
        &RuleConfig::GateStatic { ref input, threshold, value } => {
            let input = get_input(input)
                .map(Ok)
                .unwrap_or_else(|| Err(RuleConfigError::UnknownInput(input.clone())))?;
            Box::new(GateStatic::new(input, threshold, value))
        },
        &RuleConfig::Curve { ref input, ref keys, out_of_bounds_value } => {
            let input = get_input(input)
                .map(Ok)
                .unwrap_or_else(|| Err(RuleConfigError::UnknownInput(input.clone())))?;
            Box::new(Curve::new(input, keys.iter().map(|&p| p), out_of_bounds_value))
        },
    };
    Ok(ret)
}

pub struct Static(f64);

impl Rule for Static {
    fn get_value(&self) -> io::Result<f64> {
        Ok(self.0)
    }
}

pub struct Maximum {
    rules: LinkedList<Box<Rule>>,
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

pub struct GateStatic<S: AsRef<hwmon::Sensor>> {
    input: S,
    threshold: f64,
    value: f64,
}

impl<S: AsRef<hwmon::Sensor>> GateStatic<S> {
    #[inline]
    pub fn new(input: S, threshold: f64, value: f64) -> Self {
        GateStatic {
            input: input,
            threshold: threshold,
            value: value,
        }
    }
}

impl<S: AsRef<hwmon::Sensor>> Rule for GateStatic<S> {
    fn get_value(&self) -> io::Result<f64> {
        let input = self.input.as_ref();
        let value = input.get_value()?;
        if value > self.threshold {
            Ok(self.value)
        } else {
            Ok(0.0)
        }
    }
}

pub struct GateCritical<S: AsRef<hwmon::Sensor>> {
    input: S,
    value: f64,
}

impl<S: AsRef<hwmon::Sensor>> GateCritical<S> {
    #[inline]
    pub fn new(input: S, value: f64) -> Self {
        GateCritical {
            input: input,
            value: value,
        }
    }
}

impl<S: AsRef<hwmon::Sensor>> Rule for GateCritical<S> {
    fn get_value(&self) -> io::Result<f64> {
        let input = self.input.as_ref();
        let threshold = input.get_critical()?;
        let value = input.get_value()?;
        if value >= threshold {
            Ok(self.value)
        } else {
            Ok(0.0)
        }
    }
}

pub struct Curve<S: AsRef<hwmon::Sensor>> {
    input: S,
    spline: Spline<f64, f64>,
    out_of_bounds_value: f64,
}

impl<S: AsRef<hwmon::Sensor>> Curve<S> {
    pub fn new<It>(input: S, points: It, out_of_bounds_value: Option<f64>) -> Self
    where
        It: Iterator<Item=CurvePoint>,
    {
        use splines::{
            Interpolation,
            Key,
        };
        let mut is_first = true;
        let keys = points
            .map(|point| {
                let interpolation = if is_first {
                    Interpolation::Linear
                } else {
                    Interpolation::default()
                };
                is_first = true;
                Key::new(point.input, point.output, interpolation)
            });
        let spline = Spline::from_iter(keys);
        Curve {
            input: input,
            spline: spline,
            out_of_bounds_value: out_of_bounds_value.unwrap_or(1.0),
        }
    }
}

impl<S: AsRef<hwmon::Sensor>> Rule for Curve<S> {
    fn get_value(&self) -> io::Result<f64> {
        let input = self.input.as_ref();
        let value = input.get_value()?;
        let ret = self.spline.sample(value)
            .unwrap_or(self.out_of_bounds_value);
        Ok(ret)
    }
}
