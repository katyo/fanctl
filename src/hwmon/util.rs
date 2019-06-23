use std::fs;
use std::io;
use std::path::Path;
use std::str::FromStr;

pub enum ReadFileError<F: FromStr> {
    Io(io::Error),
    Parse(<F as FromStr>::Err),
}

pub fn read_file_value<F, P>(path: P, capacity: usize) -> Result<F, ReadFileError<F>> where
    F: FromStr,
    P: AsRef<Path>,
{
    use io::{BufRead, BufReader};
    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(path.as_ref())
        .map(BufReader::new)
        .map_err(ReadFileError::Io)?;
    let mut contents = String::with_capacity(capacity);
    file.read_line(&mut contents)
        .map_err(ReadFileError::Io)?;
    let contents = contents.trim_end_matches("\n");
    contents.trim()
        .parse()
        .map_err(ReadFileError::Parse)
}
