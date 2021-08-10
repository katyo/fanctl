extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde;
extern crate serde_yaml;
extern crate splines;
extern crate signal_hook;
extern crate regex;
extern crate thiserror;

mod logging;
pub mod hwmon;
pub mod config;
pub mod rules;
pub mod metrics;
pub(crate) mod path_ext;

use clap::{
    Clap,
    crate_version,
    crate_authors,
    crate_name,
};
use std:: {
    collections::HashMap,
    convert::{
        TryFrom,
        TryInto,
    },
    error::Error,
    io,
    path::{
        Path,
        PathBuf,
    },
    rc::Rc,
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    error,
};
use config::{
    Config,
    ConfigError,
};
use rules::Rule;
use serde_yaml::Error as YamlError;

#[derive(Debug, Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Options {
    #[clap(short, long, value_name = "CONFIG_FILE", about = "Config file path", parse(from_os_str))]
    config: PathBuf,
}

impl Options {
    #[inline(always)]
    pub fn config_path(&self) -> &Path {
        self.config.as_ref()
    }

    pub fn config(&self) -> Result<Config, ConfigError<YamlError>> {
        config::read_config_yaml(&self.config)
    }
}

#[derive(Debug, thiserror::Error)]
enum UpdateError {
    #[error("Error updating rule: {0}")]
    Rule(io::Error),
    #[error("Error updating fan: {0}")]
    FanIo(io::Error),
}

struct BoundRule {
    outputs: Vec<Rc<Mutex<Box<dyn hwmon::Fan>>>>,
    rule: Box<dyn Rule>,
}

impl BoundRule {
    #[inline]
    fn new(outputs: Vec<Rc<Mutex<Box<dyn hwmon::Fan>>>>, rule: Box<dyn Rule>) -> BoundRule {
        BoundRule {
            outputs: outputs,
            rule: rule,
        }
    }

    fn enable_and_set_all(&mut self, value: f64) -> io::Result<f64> {
        for output in &self.outputs {
            let mut output = output.lock().unwrap();
            output.enable()?;
            output.set_value(value)?;
        }
        Ok(value)
    }

    fn update(&mut self) -> Result<f64, UpdateError> {
        let value = self.rule.get_value()
            .map_err(UpdateError::Rule)?;
        self.enable_and_set_all(value)
            .map_err(UpdateError::FanIo)?;
        Ok(value)
    }
}

#[derive(Debug, thiserror::Error)]
enum ProgramError {
    #[error("Error finding sensor {0}: {1}")]
    FindSensor(String, config::FindHwmonError),
    #[error("Error finding fan {0}: {1}")]
    FindFan(String, config::FindHwmonError),
    #[error("Error in rule configuration: {0}")]
    RuleConfig(#[from] rules::RuleConfigError),
}

struct FanControlProgram {
    rules: Vec<BoundRule>,
    config: Config,
}

impl TryFrom<Config> for FanControlProgram {
    type Error = ProgramError;

    fn try_from(config: Config) -> Result<FanControlProgram, Self::Error> {
        let mut inputs: HashMap<String, Rc<dyn hwmon::Sensor>> = HashMap::new();
        for (name, input_config) in config.inputs.iter() {
            let name = name.clone();
            let (name, sensor): (String, Box<dyn hwmon::Sensor>) = match input_config.try_into() {
                Ok(sensor) => Ok((name, sensor)),
                Err(e) => Err(ProgramError::FindSensor(name, e)),
            }?;
            inputs.insert(name, Rc::from(sensor));
        }
        let mut outputs: HashMap<String, Rc<Mutex<Box<dyn hwmon::Fan>>>> = HashMap::new();
        for (name, output_config) in config.outputs.iter() {
            let name = name.clone();
            let (name, fan): (String, Box<dyn hwmon::Fan>) = match output_config.try_into() {
                Ok(fan) => Ok((name, fan)),
                Err(e) => Err(ProgramError::FindFan(name, e)),
            }?;
            outputs.insert(name, Rc::new(Mutex::new(fan)));
        }
        let mut rules: Vec<BoundRule> = Vec::with_capacity(config.rules.len());
        for rule_binding in config.rules.iter() {
            let rule = rules::rule_from_config(&rule_binding.rule, |name| inputs.get(name).map(Clone::clone))?;
            let mut os: Vec<Rc<Mutex<Box<dyn hwmon::Fan>>>> = Vec::with_capacity(rule_binding.outputs.len());
            for output_name in rule_binding.outputs.iter() {
                let output = outputs.get(output_name)
                    .map(Clone::clone)
                    .map(Ok)
                    .unwrap_or_else(|| Err(rules::RuleConfigError::UnknownOutput(output_name.clone())))?;
                os.push(output);
            }
            rules.push(BoundRule::new(os, rule));
        }
        Ok(FanControlProgram {
            rules: rules,
            config: config,
        })
    }
}

impl FanControlProgram {
    #[inline]
    fn interval(&self) -> std::time::Duration {
        use std::time::Duration;
        Duration::from_millis(self.config.interval)
    }

    fn run_once(&mut self) -> Result<(), UpdateError> {
        self.rules.iter_mut()
            .fold(Ok(()), |r, rule| r.and_then(move |_| rule.update().map(|_| ())))
    }

    fn disable_outputs(&mut self) -> io::Result<()> {
        self.rules.iter_mut()
            .map(|r| {
                r.outputs.iter_mut()
                    .map(|o| {
                        use io::ErrorKind;
                        o.try_lock()
                            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to lock mutex for output!"))
                            .and_then(|mut o| o.close())
                    })
                    .fold(Ok(()), Result::and)
            })
            .fold(Ok(()), Result::and)
    }
}

fn on_fan_update_error<E: error::Error>(e: E) {
    error!("{}", e);
}

fn setup_exit_handlers(flag: &Arc<AtomicBool>) -> Result<(), io::Error> {
    let signals = [
        signal_hook::SIGINT,
        signal_hook::SIGTERM,
    ];
    for &s in &signals {
        signal_hook::flag::register(s, flag.clone())?;
    }
    Ok(())
}

fn real_main(options: &Options) -> Result<(), Box<dyn Error>> {
    let config: Config = options.config()?;
    let mut program = FanControlProgram::try_from(config)?;

    let running = Arc::new(AtomicBool::new(false));
    setup_exit_handlers(&running)?;
    while !running.load(Ordering::SeqCst) {
        use std::thread;

        if let Err(e) = program.run_once() {
            on_fan_update_error(e);
            break;
        }
        thread::sleep(program.interval());
    }
    info!("Shutting down, disabling control on all outputs.");
    program.disable_outputs()?;
    info!("Shutdown successful.");
    Ok(())
}

fn main() {
    logging::init();
    let options = Options::parse();
    if let Err(e) = real_main(&options) {
        error!("{}", &e);
    }
}
