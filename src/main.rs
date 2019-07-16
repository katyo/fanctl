#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde;
extern crate serde_yaml;
extern crate splines;
extern crate ctrlc;
extern crate combination_err;

mod clap_ext;
pub mod hwmon;
pub mod config;
pub mod rules;
pub mod metrics;

use combination_err::combination_err;
use std::cell::RefCell;
use std::collections::{HashMap, LinkedList};
use std::fs;
use std::fmt;
use std::io;
use std::rc::Rc;
use config::Config;
use std::error;
use metrics::OutputMetricsTracker;

const CONFIG_ARG: &'static str = "config";

fn app() -> clap::App<'static, 'static> {
    use clap::Arg;
    let config_arg = Arg::with_name(CONFIG_ARG)
        .short("c")
        .long("config")
        .help("Config filename")
        .takes_value(true)
        .value_name("CONFIG_FILE")
        .required(true);
    clap_ext::crate_app()
        .arg(config_arg)
}

#[derive(Debug)]
struct FanUpdateError {
    description: String,
    error: io::Error,
}

impl FanUpdateError {
    pub fn new<S: AsRef<str>>(output_name: S, error: io::Error) -> Self {
        FanUpdateError {
            description: format!("Error updating fan ({})", output_name.as_ref()),
            error: error,
        }
    }
}

impl fmt::Display for FanUpdateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use error::Error;
        let description = self.description();
        let source = self.source();
        if let Some(source) = source {
            write!(f, "{}: {}", description, source)
        } else {
            write!(f, "{}", description)
        }
    }
}

impl error::Error for FanUpdateError {
    fn description(&self) -> &str {
        self.description.as_ref()
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.error)
    }
}

#[combination_err(
    "Error updating fanctl graph",
    "Error updating rule",
    "Error updating fan"
)]
#[derive(Debug)]
enum UpdateError {
    Rule(io::Error),
    Fan(FanUpdateError),
}

struct Context {
    _inputs: HashMap<String, Rc<hwmon::Sensor>>,
    outputs: RefCell<HashMap<String, Box<hwmon::Fan>>>,
    rules: LinkedList<(LinkedList<String>, RefCell<OutputMetricsTracker>, Box<rules::Rule>)>,
    config: Config,
}

impl Context {
    #[inline(always)]
    fn interval(&self) -> std::time::Duration {
        use std::time::Duration;
        Duration::from_millis(self.config.interval)
    }

    fn run_once(&mut self) -> Result<(), UpdateError> {
        let mut idx: usize = 0;
        for &(ref output_names, ref tracker, ref rule) in &self.rules {
            let value = rule.get_value()
                .map_err(UpdateError::Rule)?;
            {
                let mut tracker = tracker.borrow_mut();
                tracker.update(value);
                if tracker.count() >= self.config.log_iterations {
                    info!("Average value for rule ({}): {}", idx, tracker.average());
                    tracker.reset();
                }
                idx += 1;
            }
            output_names.iter()
                .map(|output_name| {
                    self.outputs.borrow_mut().get_mut(output_name)
                        .map(Ok)
                        .unwrap_or_else(|| Err(io::Error::new(io::ErrorKind::Other, format!("Unknown output name: {}", output_name))))
                        .and_then(|output| {
                            output.enable()?;
                            output.set_value(value)?;
                            Ok(())
                        })
                        .map_err(|e| FanUpdateError::new(output_name, e))
                        .map_err(UpdateError::from)
                })
                .fold(Ok(()), Result::and)
                .map(|_| ())?
        }
        Ok(())
    }

    fn disable_outputs(&mut self) -> io::Result<()> {
        for output in self.outputs.borrow_mut().values_mut() {
            output.disable()?;
        }
        Ok(())
    }
}

impl From<Config> for Context {
    fn from(config: Config) -> Self {
        let mut inputs = HashMap::new();
        let mut outputs = HashMap::new();
        for (name, input_config) in config.inputs.iter() {
            inputs.insert(name.clone(), Rc::from(input_config.initialize()));
        }
        for (name, output_config) in config.outputs.iter() {
            outputs.insert(name.clone(), output_config.initialize());
        }
        let mut rules = LinkedList::new();
        for rule_binding in config.rules.iter() {
            let rule: Box<rules::Rule> = rules::instantiate_rule(&rule_binding.rule, &inputs)
                .expect("failed to parse rule");
            let tracker = RefCell::new(OutputMetricsTracker::new());
            rules.push_back((rule_binding.outputs.clone(), tracker, rule));
        }
        Context {
            _inputs: inputs,
            outputs: RefCell::new(outputs),
            rules: rules,
            config: config,
        }
    }
}

#[inline(never)]
fn print_license_info<Sp: AsRef<str>, Sa: AsRef<str>>(program_name: Sp, year: &str, author: Sa) {
    println!("{} Copyright (C) {} {}", program_name.as_ref(), year, author.as_ref());
}

fn on_fan_update_error<E: error::Error>(e: E) {
    error!("{}", e);
}

fn main() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    env_logger::init();

    let matches = app().get_matches();
    let config_file_path = matches.value_of_os(CONFIG_ARG).unwrap();
    let config_file = fs::OpenOptions::new()
        .read(true)
        .open(config_file_path)
        .expect("failed to open config file");
    let config: Config = serde_yaml::from_reader(config_file)
        .expect("failed to parse config file");

    let mut ctx = Context::from(config);

    print_license_info(crate_name!(), "2019", crate_authors!());

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error settting SIGTERM handler");
    while running.load(Ordering::SeqCst) {
        use std::thread;

        if let Err(e) = ctx.run_once() {
            on_fan_update_error(e);
            break;
        }
        thread::sleep(ctx.interval());
    }
    info!("Shutting down, disabling control on all outputs.");
    ctx.disable_outputs()
        .expect("failed to shutdown outputs");
    info!("Shutdown successful.");
}
