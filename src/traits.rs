#[derive(Debug, thiserror::Error)]
pub enum SensorError {
    /// Input/output error
    #[error("Input/output error: {0}")]
    Io(#[from] std::io::Error),
    /// Nvidia error
    #[cfg(feature = "nvidia")]
    #[error("Nvidia error: {0}")]
    Nvml(#[from] nvml::error::NvmlError),
}

pub type SensorResult<T> = Result<T, SensorError>;

pub trait Sensor {
    fn get_value(&self) -> SensorResult<f64>;
    fn get_critical(&self) -> SensorResult<f64>;
}

fn from_milli(raw_value: u64) -> f64 {
    (raw_value as f64) / 1000.0
}

pub type FanError = std::io::Error;
pub type FanResult<T> = std::io::Result<T>;

pub trait Fan {
    fn set_enabled(&mut self, enabled: bool) -> FanResult<()>;
    fn set_value(&mut self, value: f64) -> FanResult<()>;
    fn close(&mut self) -> FanResult<()>;

    #[inline(always)]
    fn enable(&mut self) -> FanResult<()> {
        self.set_enabled(true)
    }

    #[inline(always)]
    fn disable(&mut self) -> FanResult<()> {
        self.set_enabled(false)
    }
}
