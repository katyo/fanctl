use std::io;
use std::path::{ PathBuf, Path };
use super::util;
use std::fs::read_to_string;
use util::ReadFileError;

fn convert_raw_value(raw_value: u64) -> f64 {
    (raw_value as f64) / 1000.0
}

pub trait Sensor {
    fn get_raw_value(&self) -> io::Result<u64>;
    fn get_raw_critical(&self) -> io::Result<u64>;

    fn get_value(&self) -> io::Result<f64> {
        self.get_raw_value().map(convert_raw_value)
    }

    fn get_critical(&self) -> io::Result<f64> {
        self.get_raw_critical().map(convert_raw_value)
    }
}

pub struct HwmonSensor<P: AsRef<Path>> {
    path: P,
}

impl HwmonSensor<PathBuf> {
    #[inline(always)]
    pub fn new<P: Into<PathBuf>>(p: P) -> Self {
        HwmonSensor {
            path: p.into(),
        }
    }
}

impl<P: AsRef<Path>> HwmonSensor<P> {
    fn base_path(&self) -> Option<(PathBuf, String)> {
        let mut path = PathBuf::from(self.path.as_ref());
        path.file_name()
            .map(|s| s.to_string_lossy())
            .and_then(|file_name| file_name.split("_").next().map(|s| s.to_string()))
            .map(move |base_name| {
                path.pop();
                (path, base_name.to_string())
            })
    }

    pub fn read(&self) -> Result<u64, ReadFileError<u64>> {
        util::read_file_value(self.path.as_ref(), 8)
    }
}

impl<P: AsRef<Path>> Sensor for HwmonSensor<P> {
    fn get_raw_value(&self) -> io::Result<u64> {
        self.read().map_err(|e| match e {
            ReadFileError::Io(e) => e,
            ReadFileError::Parse(e) => io::Error::new(io::ErrorKind::Other, e),
        })
    }

    fn get_raw_critical(&self) -> io::Result<u64> {
        if let Some((mut path, base_name)) = self.base_path() {
            path.push(format!("{}_crit", base_name));
            util::read_file_value(path, 8)
                .map_err(|e| match e {
                    ReadFileError::Io(e) => e,
                    ReadFileError::Parse(e) => io::Error::new(io::ErrorKind::Other, e),
                })
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "failed to get critical file path"))
        }
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
            return Ok(Some(path))
        }
    }
    Ok(None)
}

/// Search for a hwmon input by name and label
pub fn search_input(name: &str, label: &str) -> io::Result<Option<PathBuf>> {
    if let Some(hwmon) = search_hwmon(name)? {
        for file in hwmon.read_dir()?.filter_map(|r| r.ok()) {
            let path = file.path();
            if let Some(path_str) = path.as_os_str().to_str() {
                if path_str.ends_with("label") {
                    if read_to_string(&path)?.trim() == label {
                        let input = format!("{}input", path_str.trim_end_matches("label"));
                        debug!("found hwmon input {}/{} at {}", name, label, input);
                        return Ok(Some(input.into()));
                    }
                }
            }
        }
    }
    Ok(None)
}
