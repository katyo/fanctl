use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::io;
use super::{util, Fan};
use std::fs;

#[derive(Debug, Copy, Clone)]
pub enum PwmEnableState {
    Disabled,
    Manual,
    Automatic(u8),
}

impl From<u8> for PwmEnableState {
    fn from(value: u8) -> Self {
        match value {
            0x0 => PwmEnableState::Disabled,
            0x1 => PwmEnableState::Manual,
            v => PwmEnableState::Automatic(v),
        }
    }
}

impl Into<u8> for PwmEnableState {
    fn into(self) -> u8 {
        use PwmEnableState::*;
        match self {
            Disabled => 0x0,
            Manual => 0x1,
            Automatic(v) => v,
        }
    }
}

#[inline(always)]
fn pwm_enable_state(enabled: bool) -> PwmEnableState {
    if enabled {
        PwmEnableState::Manual
    } else {
        PwmEnableState::Disabled
    }
}

const ENABLE_PATH: &'static str = "enable";

pub struct PwmFan<P: AsRef<Path>> {
    base_path: P,
    name: String,
    initial_state: PwmEnableState,
}

impl<P: AsRef<Path>> PwmFan<P> {
    pub fn new(base_path: P, name: String) -> io::Result<Self> {
        let mut ret = PwmFan {
            base_path: base_path,
            name: name,
            initial_state: PwmEnableState::Disabled,
        };
        ret.initial_state = ret.enabled()?;
        Ok(ret)
    }

    fn get_path(&self, component: Option<&str>) -> PathBuf {
        let mut path = PathBuf::from(self.base_path.as_ref());
        let component = component.filter(|s| s.len() > 0);
        let filename = component
            .map(|c| Cow::Owned(format!("{}_{}", &self.name, c)))
            .unwrap_or_else(|| Cow::Borrowed(&self.name));
        let filename: &str = filename.as_ref();
        path.push(filename);
        path
    }

    pub fn enabled(&self) -> io::Result<PwmEnableState> {
        use util::ReadFileResult;
        let enabled_path = self.get_path(Some(ENABLE_PATH));
        let value: u8 = util::read_file_value(&enabled_path, 2)
            .into_io_result()?;
        Ok(PwmEnableState::from(value))
    }

    pub fn set_enabled_pwm(&mut self, state: PwmEnableState) -> io::Result<()> {
        use io::Write;
        let enabled_path = self.get_path(Some(ENABLE_PATH));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&enabled_path)?;
        let value: u8 = state.into();
        write!(&mut file, "{}", value)
    }

    pub fn set_value_pwm(&mut self, value: u8) -> io::Result<()> {
        use io::Write;
        let value_path = self.get_path(None);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&value_path)?;
        write!(&mut file, "{}", value)
    }
}

impl<P: AsRef<Path>> Fan for PwmFan<P> {
    fn set_enabled(&mut self, enabled: bool) -> io::Result<()> {
        self.set_enabled_pwm(pwm_enable_state(enabled))
    }

    fn set_value(&mut self, value: f64) -> io::Result<()> {
        let raw_value: u8 = (value * 255.0) as u8;
        self.set_value_pwm(raw_value)
    }

    fn close(&mut self) -> io::Result<()> {
        self.set_enabled_pwm(self.initial_state)
    }
}
