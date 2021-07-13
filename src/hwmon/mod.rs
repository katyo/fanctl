use std::io;

pub mod amdgpu;

mod pwm;
mod sensor;

pub mod util;

pub use sensor::{search_input, search_hwmon, HwmonSensor, Sensor};
pub use pwm::*;

pub trait Fan {
    fn set_enabled(&mut self, enabled: bool) -> io::Result<()>;
    fn set_value(&mut self, value: f64) -> io::Result<()>;
    fn close(&mut self) -> io::Result<()>;

    #[inline(always)]
    fn enable(&mut self) -> io::Result<()> {
        self.set_enabled(true)
    }

    #[inline(always)]
    fn disable(&mut self) -> io::Result<()> {
        self.set_enabled(false)
    }
}
