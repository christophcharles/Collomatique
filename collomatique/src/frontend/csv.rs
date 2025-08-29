use std::fs::File;
use std::io::Read;

use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid CSV structure")]
    CSV(#[from] ::csv::Error),
    #[error("Error while reading file")]
    IO(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Content {
    content: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Extract {
    pub headers: Option<Vec<String>>,
    pub lines: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Params {
    pub has_headers: bool,
    pub delimiter: u8,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            has_headers: true,
            delimiter: b';',
        }
    }
}

impl Extract {
    fn line_from_string_record(record: &::csv::StringRecord) -> Vec<String> {
        let mut line = Vec::new();
        for field in record {
            line.push(field.to_string());
        }
        line
    }

    pub fn get_column_count(&self) -> usize {
        let mut max_column_count = 0usize;
        for line in &self.lines {
            if line.len() > max_column_count {
                max_column_count = line.len();
            }
        }
        if let Some(headers) = &self.headers {
            if headers.len() > max_column_count {
                max_column_count = headers.len();
            }
        }
        max_column_count
    }
}

impl Content {
    pub fn new() -> Self {
        Content {
            content: Vec::new(),
        }
    }

    pub fn from_raw(content: &[u8]) -> Self {
        Content {
            content: Vec::from(content),
        }
    }

    pub fn from_csv_file(path: &std::path::Path) -> Result<Self> {
        let mut csv_file = Content::new();

        let mut file = File::open(path)?;
        file.read_to_end(&mut csv_file.content)?;

        Ok(csv_file)
    }

    pub fn extract(&self, params: &Params) -> Result<Extract> {
        let mut rdr = ::csv::ReaderBuilder::new()
            .has_headers(params.has_headers)
            .delimiter(params.delimiter)
            .flexible(true)
            .from_reader(self.content.as_slice());

        let headers = if params.has_headers {
            Some(Extract::line_from_string_record(rdr.headers()?))
        } else {
            None
        };
        let mut lines = Vec::new();
        for line in rdr.records() {
            lines.push(Extract::line_from_string_record(&line?));
        }

        Ok(Extract { headers, lines })
    }
}
