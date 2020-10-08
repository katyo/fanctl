#[macro_use]
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

pub mod hwmon;
pub mod config;
pub mod rules;
pub mod metrics;
pub(crate) mod path_ext;

use clap::Clap;
use std:: {
    collections::HashMap,
    convert::TryFrom,
    fmt,
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
        let source = self.source();
        if let Some(source) = source {
            write!(f, "{}: {}", &self.description, source)
        } else {
            write!(f, "{}", &self.description)
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

#[derive(Debug, thiserror::Error)]
enum UpdateError {
    #[error("Error updating rule")]
    Rule(#[from] io::Error),
    #[error("Error updating fan")]
    Fan(#[from] FanUpdateError),
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

    fn update(&mut self) -> io::Result<f64> {
        let value = self.rule.get_value()?;
        for output in &self.outputs {
            let mut output = output.lock().unwrap();
            output.enable()?;
            output.set_value(value)?;
        }
        Ok(value)
    }
}

struct FanControlProgram {
    rules: Vec<BoundRule>,
    config: Config,
}

impl TryFrom<Config> for FanControlProgram {
    type Error = String;

    fn try_from(config: Config) -> Result<FanControlProgram, String> {
        let inputs: HashMap<String, Rc<dyn hwmon::Sensor>> = config.inputs.iter()
            .map(|(name, input_config)| {
                let input: Box<dyn hwmon::Sensor> = input_config.into();
                (name.clone(), Rc::from(input))
            })
            .collect();
        let outputs: HashMap<String, Rc<Mutex<Box<dyn hwmon::Fan>>>> = config.outputs.iter()
            .map(|(name, output_config)| {
                let output: Box<dyn hwmon::Fan> = output_config.into();
                (name.clone(), Mutex::new(output))
            })
            .map(|(name, output)| (name, Rc::new(output)))
            .collect();
        let mut rules: Vec<BoundRule> = Vec::with_capacity(config.rules.len());
        for rule_binding in config.rules.iter() {
            let rule = rules::rule_from_config(&rule_binding.rule, |name| inputs.get(name).map(Clone::clone))
                .map_err(|e| format!("{:?}", e))?;
            let mut os: Vec<Rc<Mutex<Box<dyn hwmon::Fan>>>> = Vec::with_capacity(rule_binding.outputs.len());
            for output_name in rule_binding.outputs.iter() {
                let output = outputs.get(output_name)
                    .map(Clone::clone)
                    .map(Ok)
                    .unwrap_or_else(|| Err(rules::RuleConfigError::UnknownOutput(output_name.clone())))
                    .map_err(|e| format!("{:?}", e))?;
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
        for rule in self.rules.iter_mut() {
            rule.update()
                .map_err(|e| FanUpdateError::new("", e))
                .map_err(UpdateError::from)?;
        }
        Ok(())
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

#[inline(never)]
fn print_license_info<Sp: AsRef<str>, Sa: AsRef<str>>(program_name: Sp, year: &str, author: Sa) {
    println!("{} Copyright (C) {} {}", program_name.as_ref(), year, author.as_ref());
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

fn main() {
    env_logger::init();

    let options = Options::parse();
    let config: Config = options.config()
        .expect("failed to get config");

    let mut program = FanControlProgram::try_from(config)
        .expect("Failed to initialize");

    print_license_info(crate_name!(), "2019", crate_authors!());

    let running = Arc::new(AtomicBool::new(false));
    setup_exit_handlers(&running)
        .expect("Failed to set up exit handlers");
    while !running.load(Ordering::SeqCst) {
        use std::thread;

        if let Err(e) = program.run_once() {
            on_fan_update_error(e);
            break;
        }
        thread::sleep(program.interval());
    }
    info!("Shutting down, disabling control on all outputs.");
    program.disable_outputs()
        .expect("failed to shutdown outputs");
    info!("Shutdown successful.");
}
