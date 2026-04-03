use log::*;
use regex::Regex;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const WILDCARD_CHAR: char = '*';

pub trait PathExt {
    fn expand_wildcards(&self) -> io::Result<PathBuf>;
}

impl PathExt for Path {
    fn expand_wildcards(&self) -> io::Result<PathBuf> {
        let mut real_path = PathBuf::from("/");
        for component in self.iter() {
            let component_s = component.to_str().unwrap();
            let mut wildcards = component_s.matches(WILDCARD_CHAR).peekable();
            if wildcards.peek().is_some() {
                trace!("Found wildcard component: {:?}", component_s);
                let r = {
                    let pattern = format!("^{}$", component_s.replace(WILDCARD_CHAR, ".*"));
                    Regex::new(&pattern)
                        .unwrap_or_else(|_| panic!("Failed to compile regular expression while evaluating wildcard path component: \"{:?}\"", component))
                };
                let entries = fs::read_dir(&real_path)?;
                let found_entry = entries
                    .filter_map(|entry| {
                        if let Ok(entry) = entry {
                            let entry_s = entry.file_name().to_str().map(String::from).unwrap();
                            if r.is_match(&entry_s) {
                                Some(entry_s)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .next();
                if let Some(found_entry) = found_entry {
                    real_path.push(found_entry);
                } else {
                    // Fallback case
                    // TODO: is this the desired behavior?
                    real_path.push(component);
                }
            } else {
                trace!(
                    "There were no wildcards in path component: \"{}\"",
                    component_s
                );
                // There were no wildcards, so just append the component directly
                real_path.push(component);
            }
        }
        Ok(real_path)
    }
}
