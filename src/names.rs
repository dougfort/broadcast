use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use thiserror::Error;

pub struct Names {
    names: Vec<String>,
}

impl Names {
    /// create a new names object
    pub fn new(path: &str) -> Result<Self, NamesError> {
        let mut n = Names {
            names: Vec::<String>::new(),
        };
        n.load(path)?;
        Ok(n)
    }

    /// load the names object from a file
    fn load(&mut self, path: &str) -> Result<(), NamesError> {
        let lines = read_lines(path)?;
        for line in lines {
            self.names.push(line?);
        }
        Ok(())
    }

    /// choose a random name
    pub fn choose(&self) -> Result<String, NamesError> {
        let mut rng = thread_rng();
        match self.names.choose(&mut rng) {
            Some(name) => Ok(name.clone()),
            None => Err(NamesError::NodDataError),
        }
    }
}

impl Default for Names {
    /// return a new, empty Names object
    fn default() -> Self {
        Names {
            names: Vec::<String>::new(),
        }
    }
}

/// returned Error for name failures
#[derive(Error, Debug)]
pub enum NamesError {
    /// Error parsing a value
    #[error(transparent)]
    LoadError(#[from] std::io::Error),

    /// No data in Names
    #[error("No Names found")]
    NodDataError,
}

// pasted from Rust By Example
// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_load_names() {
        let n = Names::new("data/names.txt").unwrap();
        assert!(!n.names.is_empty());

        n.choose().unwrap();
    }
}
