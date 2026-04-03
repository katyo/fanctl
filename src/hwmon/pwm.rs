use super::util;
use crate::Fan;
use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
};

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

impl From<PwmEnableState> for u8 {
    fn from(val: PwmEnableState) -> Self {
        use PwmEnableState::*;
        match val {
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

const ENABLE_PATH: &str = "enable";

pub struct PwmFan<P: AsRef<Path>> {
    name: String,
    initial_state: PwmEnableState,
    real_path: P,
}

impl<P: AsRef<Path>> PwmFan<P> {
    #[inline(always)]
    pub fn real_path(&self) -> &Path {
        self.real_path.as_ref()
    }

    pub fn new(base_path: P, name: String) -> io::Result<Self> {
        let mut ret = PwmFan {
            name,
            initial_state: PwmEnableState::Disabled,
            real_path: base_path,
        };
        ret.initial_state = ret.enabled()?;
        Ok(ret)
    }

    fn get_path(&self, component: Option<&str>) -> PathBuf {
        let mut path = self.real_path().to_owned();
        let component = component.filter(|s| !s.is_empty());
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
        let value: u8 = util::read_file_value(&enabled_path, 2).into_io_result()?;
        Ok(PwmEnableState::from(value))
    }

    pub fn set_enabled_pwm(&mut self, state: PwmEnableState) -> io::Result<()> {
        use io::Write;
        let enabled_path = self.get_path(Some(ENABLE_PATH));
        let mut file = fs::OpenOptions::new().write(true).open(&enabled_path)?;
        let value: u8 = state.into();
        write!(&mut file, "{}", value)
    }

    pub fn set_value_pwm(&mut self, value: u8) -> io::Result<()> {
        use io::Write;
        let value_path = self.get_path(None);
        let mut file = fs::OpenOptions::new().write(true).open(&value_path)?;
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
