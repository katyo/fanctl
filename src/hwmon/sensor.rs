use super::util;
use crate::{Sensor, SensorResult};
use log::*;
use std::{
    fs::read_to_string,
    io,
    path::{Path, PathBuf},
};
use util::ReadFileError;

pub struct HwmonSensor {
    path_val: PathBuf,
    path_crit: Option<PathBuf>,
}

impl Drop for HwmonSensor {
    fn drop(&mut self) {
        info!("Remove hwmon sensor: {}", self.path_val.display());
    }
}

impl HwmonSensor {
    #[inline(always)]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path_val = path.into();
        let path_crit = with_spec(&path_val, "_crit");
        info!("Create hwmon sensor: {}", path_val.display());
        HwmonSensor {
            path_val,
            path_crit,
        }
    }
}

fn with_spec(path: impl AsRef<Path>, spec: impl AsRef<str>) -> Option<PathBuf> {
    let path = path.as_ref();
    path.parent().and_then(|dir_name| {
        path.file_name()
            .map(|s| s.to_string_lossy())
            .and_then(|file_name| file_name.split("_").next().map(|s| s.to_string()))
            .map(move |base_name| dir_name.join(format!("{}_{}", base_name, spec.as_ref())))
    })
}

impl HwmonSensor {
    pub fn read_val_raw(&self) -> Result<u64, ReadFileError<u64>> {
        util::read_file_value(&self.path_val, 8)
    }

    pub fn read_crit_raw(&self) -> Result<Option<u64>, ReadFileError<u64>> {
        self.path_crit
            .as_ref()
            .map(|path| util::read_file_value(path, 8))
            .transpose()
    }
}

impl Sensor for HwmonSensor {
    fn get_value(&self) -> SensorResult<f64> {
        Ok(self.read_val_raw().map_err(|e| match e {
            ReadFileError::Io(e) => e,
            ReadFileError::Parse(e) => io::Error::other(e),
        })? as f64
            * 1e-3)
    }

    fn get_critical(&self) -> SensorResult<f64> {
        Ok(self
            .read_crit_raw()
            .map_err(|e| match e {
                ReadFileError::Io(e) => e,
                ReadFileError::Parse(e) => io::Error::other(e),
            })?
            .ok_or_else(|| io::Error::other("failed to get critical file path"))? as f64
            * 1e-3)
    }
}

/// Search for a hwmon device by name
pub fn search_hwmon(name: &str) -> io::Result<Option<PathBuf>> {
    for hwmon in PathBuf::from("/sys/class/hwmon")
        .read_dir()?
        .filter_map(|r| r.ok())
    {
        let path = hwmon.path();
        if read_to_string(path.join("name"))?.trim() == name {
            debug!("found hwmon {} at {}", name, path.to_string_lossy());
            return Ok(Some(path));
        }
    }
    Ok(None)
}

pub enum SearchInput<'s> {
    ByName(&'s str),
    ByLabel(&'s str),
}

/// Search for a hwmon input by name and label
pub fn search_input(name: &str, predicate: SearchInput) -> io::Result<Option<PathBuf>> {
    if let Some(hwmon) = search_hwmon(name)? {
        for file in hwmon.read_dir()?.filter_map(|r| r.ok()) {
            let path = file.path();
            if let Some(path_str) = path.as_os_str().to_str() {
                match predicate {
                    SearchInput::ByName(in_name)
                        if path_str.ends_with("_input")
                            && path_str
                                .rsplit_once('/')
                                .map(|(_, name_str)| name_str.trim_end_matches("_input") == in_name)
                                .unwrap_or_default() =>
                    {
                        let input = path_str;
                        debug!("found hwmon input {}/{} at {}", name, in_name, input);
                        return Ok(Some(input.into()));
                    }
                    SearchInput::ByLabel(label)
                        if path_str.ends_with("_label")
                            && read_to_string(&path)?.trim() == label =>
                    {
                        let input = format!("{}_input", path_str.trim_end_matches("_label"));
                        debug!("found hwmon input {}/{} at {}", name, label, input);
                        return Ok(Some(input.into()));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}
