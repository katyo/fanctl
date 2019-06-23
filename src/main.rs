#[macro_use]
extern crate clap;
extern crate serde;
#[macro_use]
extern crate serde_yaml;

mod clap_ext;
pub mod hwmon;
pub mod config;

use std::collections::HashMap;
use std::env;
use std::fs;
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
    inputs: HashMap<String, Box<hwmon::Sensor>>,
    outputs: HashMap<String, Box<hwmon::Fan>>,
}

impl From<Config> for Context {
    fn from(config: Config) -> Self {
        let mut inputs = HashMap::new();
        let mut outputs = HashMap::new();
        for (name, input_config) in config.inputs.iter() {
            inputs.insert(name.clone(), input_config.initialize());
        }
        for (name, output_config) in config.outputs.iter() {
            outputs.insert(name.clone(), output_config.initialize());
        }
        Context {
            inputs: inputs,
            outputs: outputs
        }
    }
}

fn main() {
    //use hwmon::Fan;
    //let mut fan = AmdgpuFan::new("/sys/class/drm/card0/device/hwmon/hwmon0", "fan1");

    //let mut args = env::args().skip(1);
    //let value: f64 = if let Some(arg) = args.next() {
    //    arg.parse()
    //        .expect("failed to parse value")
    //} else {
    //    1.0
    //};
    //fan.enable()
    //    .expect("failed to enable fan");
    //fan.set_value(value)
    //    .expect("failed to set fan value");
    let matches = app().get_matches();
    let config_file_path = matches.value_of_os(CONFIG_ARG).unwrap();
    let config_file = fs::OpenOptions::new()
        .read(true)
        .open(config_file_path)
        .expect("failed to open config file");
    let config: Config = serde_yaml::from_reader(config_file)
        .expect("failed to parse config file");
    println!("{:?}", &config);
    let ctx = Context::from(config);
    for (name, input) in ctx.inputs.iter() {
        println!("{}: {:?}", name, input.get_value());
    }
}
