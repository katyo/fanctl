#[macro_use]
extern crate clap;
extern crate serde;
extern crate serde_yaml;

mod clap_ext;
pub mod hwmon;
pub mod config;
pub mod rules;

use std::cell::RefCell;
use std::collections::{HashMap, LinkedList};
use std::fs;
use std::io;
use std::rc::Rc;
use config::Config;

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

struct Context {
    _inputs: HashMap<String, Rc<hwmon::Sensor>>,
    outputs: RefCell<HashMap<String, Box<hwmon::Fan>>>,
    rules: LinkedList<(LinkedList<String>, Box<rules::Rule>)>,
    config: Config,
}

impl Context {
    #[inline(always)]
    fn interval(&self) -> std::time::Duration {
        use std::time::Duration;
        Duration::from_millis(self.config.interval)
    }

    fn run_once(&mut self) -> io::Result<()> {
        for &(ref output_names, ref rule) in &self.rules {
            let value = rule.get_value()?;
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
                })
                .fold(Ok(()), Result::and)
                .map(|_| ())?
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
            rules.push_back((rule_binding.outputs.clone(), rule));
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

#[allow(unused_must_use)]
fn on_fan_update_error(e: io::Error) {
    use io::Write;

    let mut stderr = io::stderr();
    writeln!(&mut stderr, "Error updating fans: {:?}", e);
}

fn main() {
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

    loop {
        use std::thread;

        if let Err(e) = ctx.run_once() {
            on_fan_update_error(e);
            break;
        }
        thread::sleep(ctx.interval());
    }
}
