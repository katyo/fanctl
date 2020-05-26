use std::io;
use std::path::{ PathBuf, Path };
use super::util;
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
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        use crate::path_ext::PathExt;
        let path = path.as_ref();
        HwmonSensor {
            path: path.expand_wildcards().expect("Failed to expand wildcards in path"),
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
