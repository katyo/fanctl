use std::io;
use std::path::Path;
use super::util;
use util::ReadFileError;

pub trait Sensor {
    fn get_raw_value(&self) -> io::Result<u64>;

    fn get_value(&self) -> io::Result<f64> {
        self.get_raw_value().map(|v| (v as f64) / 1000.0)
    }
}

pub struct HwmonSensor<P: AsRef<Path>> {
    path: P
}

impl<P: AsRef<Path>> HwmonSensor<P> {
    pub fn new(path: P) -> Self {
        HwmonSensor {
            path: path,
        }
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
}
