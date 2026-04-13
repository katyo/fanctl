use crate::{Sensor, SensorResult};
use log::*;
use nvml::{
    Device, Nvml,
    enum_wrappers::device::{TemperatureSensor, TemperatureThreshold},
    error::NvmlError,
};
use std::sync::OnceLock;

static NVML: OnceLock<Result<Nvml, NvmlError>> = OnceLock::new();

pub fn nvml() -> Result<&'static Nvml, NvmlError> {
    NVML.get_or_init(Nvml::init)
        .as_ref()
        .inspect_err(|error| error!("NvError: {error}"))
        .map_err(|_error| NvmlError::NotFound)
}

pub struct NvidiaSensor {
    device: Device<'static>,
}

impl Drop for NvidiaSensor {
    fn drop(&mut self) {
        info!(
            "Remove nvidia sensor: {}",
            self.device.uuid().unwrap_or_default()
        );
    }
}

impl NvidiaSensor {
    pub fn new(device: Device<'static>) -> Self {
        info!(
            "Create nvidia sensor: {}",
            device.uuid().unwrap_or_default()
        );
        Self { device }
    }
}

impl Sensor for NvidiaSensor {
    fn get_value(&self) -> SensorResult<f64> {
        Ok(self
            .device
            .temperature(TemperatureSensor::Gpu)
            .map(|v| v as f64)?)
    }

    fn get_critical(&self) -> SensorResult<f64> {
        Ok(self
            .device
            .temperature_threshold(TemperatureThreshold::Shutdown)
            .map(|v| v as f64)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_nvml() {
        let nvml = nvml().unwrap();

        let num = nvml.device_count().unwrap();

        let dev = nvml.device_by_index(0).unwrap();

        eprintln!("# {num} {dev:?}");
    }
}
