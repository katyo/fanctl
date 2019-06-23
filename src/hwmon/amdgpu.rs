use std::fs;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::Fan;

#[derive(Debug, Clone)]
pub struct AmdgpuFan {
    enable_path: PathBuf,
    target_path: PathBuf,
    min_path: PathBuf,
    max_path: PathBuf,
}

enum ReadFileError<P> {
    Io(io::Error),
    Parse(P),
}

fn read_file_value<F, P>(path: P) -> Result<F, ReadFileError<<F as FromStr>::Err>> where
    F: FromStr,
    P: AsRef<Path>,
{
    use io::BufRead;

    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(path.as_ref())
        .map(BufReader::new)
        .map_err(ReadFileError::Io)?;
    let mut contents = String::with_capacity(8);
    file.read_line(&mut contents)
        .map_err(ReadFileError::Io)?;
    let contents = contents.trim_end_matches("\n");
    contents.trim().parse()
        .map_err(ReadFileError::Parse)
}

impl AmdgpuFan {
    pub fn new<P: AsRef<Path>, S: AsRef<str>>(base_path: P, name: S) -> Self {
        let base_path = base_path.as_ref();
        let name = name.as_ref();

        let make_path = |filename: &str| {
            let mut path = base_path.to_owned();
            path.push(format!("{}_{}", name, filename));
            path
        };

        AmdgpuFan {
            enable_path: make_path("enable"),
            target_path: make_path("target"),
            min_path: make_path("min"),
            max_path: make_path("max")
        }
    }

    pub fn min(&self) -> io::Result<u64> {
        read_file_value(&self.min_path).map_err(|e| match e {
            ReadFileError::Io(e) => e,
            ReadFileError::Parse(e) => panic!("{:?}", e),
        })
    }

    pub fn max(&self) -> io::Result<u64> {
        read_file_value(&self.max_path).map_err(|e| match e {
            ReadFileError::Io(e) => e,
            ReadFileError::Parse(e) => panic!("{:?}", e),
        })
    }

    pub fn enabled(&self) -> io::Result<bool> {
        read_file_value::<u8, _>(&self.enable_path)
            .map(|v| v > 0)
            .map_err(|e| match e {
                ReadFileError::Io(e) => e,
                ReadFileError::Parse(e) => panic!("{:?}", e),
            })
    }

    fn set_target(&mut self, target: u64) -> io::Result<()> {
        use io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&self.target_path)?;
        write!(file, "{}", target)
    }
}

impl Fan for AmdgpuFan {
    fn set_enabled(&mut self, enabled: bool) -> io::Result<()> {
        use io::Write;

        let value = if enabled {
            "1"
        } else {
            "0"
        };
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(&self.enable_path)?;
        write!(file, "{}", value)
    }

    fn set_value(&mut self, value: f64) -> io::Result<()> {
        let min = self.min()?;
        let max = self.max()?;
        let target = if value >= 1.0 {
            max
        } else if value <= 0.0 {
            min
        } else {
            let mut target = (value * ((max - min) as f64)).round() as u64;
            target += min;
            target
        };
        self.set_target(target)
    }
}
