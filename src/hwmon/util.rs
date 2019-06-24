use std::fs;
use std::io;
use std::path::Path;
use std::str::FromStr;
use std::error;

pub enum ReadFileError<F: FromStr> {
    Io(io::Error),
    Parse(<F as FromStr>::Err),
}

pub trait ReadFileResult<T: Sized> {
    fn into_io_result(self) -> io::Result<T>;
}

impl<T> ReadFileResult<T> for Result<T, ReadFileError<T>> where
    T: Sized,
    T: FromStr,
    <T as FromStr>::Err: error::Error + Sync + Send + 'static,
{
    fn into_io_result(self) -> io::Result<T> {
        use io::ErrorKind;
        self.map_err(|err| match err {
            ReadFileError::Io(e) => e,
            ReadFileError::Parse(e) => io::Error::new(ErrorKind::InvalidData, e),
        })
    }
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
