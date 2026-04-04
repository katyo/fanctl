use crate::{
    Sensor, SensorResult,
    config::{CurvePoint, Rule as RuleConfig},
};
use core::cmp::PartialOrd;
use splines::Spline;
use std::{cell::RefCell, collections::LinkedList, io, rc::Rc};

pub trait Rule {
    fn get_value(&self) -> SensorResult<f64>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleConfigError {
    #[error("Unknown input: {0}")]
    UnknownInput(String),
    #[error("Unknown output: {0}")]
    UnknownOutput(String),
}

pub fn rule_from_config<F>(
    config: &RuleConfig,
    get_input: F,
) -> Result<Box<dyn Rule>, RuleConfigError>
where
    F: Fn(&String) -> Option<Rc<dyn Sensor>> + Copy,
{
    let ret: Box<dyn Rule> = match config {
        &RuleConfig::Static(v) => Box::new(Static(v)),
        RuleConfig::Maximum(rules) => {
            let mut rs = LinkedList::new();
            for rule in rules.iter() {
                let r = rule_from_config(rule, get_input)?;
                rs.push_back(r);
            }
            Box::new(Maximum { rules: rs })
        }
        &RuleConfig::GateCritical { ref input, value } => {
            let input = get_input(input)
                .map(Ok)
                .unwrap_or_else(|| Err(RuleConfigError::UnknownInput(input.clone())))?;
            Box::new(GateCritical::new(input, value))
        }
        &RuleConfig::GateStatic {
            ref input,
            threshold,
            value,
        } => {
            let input = get_input(input)
                .map(Ok)
                .unwrap_or_else(|| Err(RuleConfigError::UnknownInput(input.clone())))?;
            Box::new(GateStatic::new(input, threshold, value))
        }
        &RuleConfig::Curve {
            ref input,
            ref keys,
            out_of_bounds_value,
        } => {
            let input = get_input(input)
                .map(Ok)
                .unwrap_or_else(|| Err(RuleConfigError::UnknownInput(input.clone())))?;
            Box::new(Curve::new(input, keys.iter().copied(), out_of_bounds_value))
        }
        &RuleConfig::Smooth { ref rule, samples } => {
            let r = rule_from_config(rule, get_input)?;
            Box::new(Smooth::new(r, samples))
        }
    };
    Ok(ret)
}

pub struct Static(f64);

impl Rule for Static {
    fn get_value(&self) -> SensorResult<f64> {
        Ok(self.0)
    }
}

pub struct Maximum {
    rules: LinkedList<Box<dyn Rule>>,
}

fn partial_max<V: PartialOrd + Copy>(fst: V, snd: V) -> V {
    if fst.ge(&snd) { fst } else { snd }
}

impl Rule for Maximum {
    fn get_value(&self) -> SensorResult<f64> {
        let mut max = None;
        for rule in &self.rules {
            let value = rule.get_value()?;
            if let Some(current_max) = max {
                max = Some(partial_max(current_max, value));
            } else {
                max = Some(value);
            }
        }
        Ok(max.ok_or(io::Error::other("No inputs available for \"Maximum\" rule"))?)
    }
}

pub struct Smooth {
    rule: Box<dyn Rule>,
    samples: usize,
    buffer: RefCell<Vec<f64>>,
}

impl Smooth {
    #[inline]
    pub fn new(rule: Box<dyn Rule>, samples: usize) -> Self {
        Smooth {
            rule,
            samples,
            buffer: RefCell::new(Vec::with_capacity(samples)),
        }
    }

    fn add_value(&self, value: f64) {
        let mut buffer = self.buffer.borrow_mut();
        buffer.push(value);
        if buffer.len() > self.samples {
            buffer.remove(0);
        }
    }

    fn get_smoothed(&self) -> Option<f64> {
        use std::cmp::Ordering;
        let mut values = self.buffer.borrow().clone();
        let len = values.len();
        if len == 0 {
            return None;
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let mut cut = len / 5;
        if len > 2 {
            cut = std::cmp::max(1, cut);
        }
        let cut_values = &values[cut..len - cut];
        let mut avg = 0.;
        for i in cut_values {
            avg += i;
        }
        avg /= (len - 2 * cut) as f64;
        Some(avg)
    }
}

impl Rule for Smooth {
    fn get_value(&self) -> SensorResult<f64> {
        let value = self.rule.get_value()?;
        self.add_value(value);

        match self.get_smoothed() {
            Some(value) => Ok(value),
            None => Err(io::Error::other("No input data"))?,
        }
    }
}

pub struct GateStatic<S: AsRef<dyn Sensor>> {
    input: S,
    threshold: f64,
    value: f64,
}

impl<S: AsRef<dyn Sensor>> GateStatic<S> {
    #[inline]
    pub fn new(input: S, threshold: f64, value: f64) -> Self {
        GateStatic {
            input,
            threshold,
            value,
        }
    }
}

impl<S: AsRef<dyn Sensor>> Rule for GateStatic<S> {
    fn get_value(&self) -> SensorResult<f64> {
        let input = self.input.as_ref();
        let value = input.get_value()?;
        if value > self.threshold {
            Ok(self.value)
        } else {
            Ok(0.0)
        }
    }
}

pub struct GateCritical<S: AsRef<dyn Sensor>> {
    input: S,
    value: f64,
}

impl<S: AsRef<dyn Sensor>> GateCritical<S> {
    #[inline]
    pub fn new(input: S, value: f64) -> Self {
        GateCritical { input, value }
    }
}

impl<S: AsRef<dyn Sensor>> Rule for GateCritical<S> {
    fn get_value(&self) -> SensorResult<f64> {
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

pub struct Curve<S: AsRef<dyn Sensor>> {
    input: S,
    spline: Spline<f64, f64>,
    out_of_bounds_value: f64,
}

impl<S: AsRef<dyn Sensor>> Curve<S> {
    pub fn new<It>(input: S, points: It, out_of_bounds_value: Option<f64>) -> Self
    where
        It: Iterator<Item = CurvePoint>,
    {
        use splines::{Interpolation, Key};
        let mut is_first = true;
        let keys = points.map(|point| {
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
            input,
            spline,
            out_of_bounds_value: out_of_bounds_value.unwrap_or(1.0),
        }
    }
}

impl<S: AsRef<dyn Sensor>> Rule for Curve<S> {
    fn get_value(&self) -> SensorResult<f64> {
        let input = self.input.as_ref();
        let value = input.get_value()?;
        let ret = self
            .spline
            .sample(value)
            .unwrap_or(self.out_of_bounds_value);
        Ok(ret)
    }
}
