use pyo3::exceptions::PyIOError;

use super::*;

fn test_part_uppercase(part: &str) -> bool {
    part.chars()
        .filter(|c| c.is_alphabetic())
        .all(|c| c.is_uppercase())
}

fn compute_name_size_hint(
    parts: &Vec<&str>,
    first_name_first: bool,
) -> Option<std::num::NonZeroUsize> {
    if first_name_first {
        let mut count = 0usize;
        for part in parts.iter().rev() {
            let upper_case = test_part_uppercase(*part);
            if upper_case {
                count += 1
            } else {
                break;
            }
        }
        if count == 0 {
            return None;
        }
        let pos = parts.len() - count;
        std::num::NonZeroUsize::new(pos)
    } else {
        let mut count = 0usize;
        for part in parts {
            let upper_case = test_part_uppercase(*part);
            if upper_case {
                count += 1
            } else {
                break;
            }
        }
        if count == parts.len() {
            return None;
        }
        std::num::NonZeroUsize::new(count)
    }
}

fn extract_name_parts_with_hint(
    name_parts: &Vec<&str>,
    firstname_first: bool,
    hint: std::num::NonZeroUsize,
) -> (String, String) {
    let first_name;
    let last_name;
    if firstname_first {
        last_name = name_parts[hint.get()..].join(" ");
        first_name = name_parts[0..hint.get()].join(" ");
    } else {
        last_name = name_parts[0..hint.get()].join(" ");
        first_name = name_parts[hint.get()..].join(" ");
    }
    (first_name, last_name)
}

fn extract_name_parts_no_hint(name: &String, firstname_first: bool) -> (String, String) {
    let first_name;
    let last_name;

    if firstname_first {
        match name.rsplit_once(' ') {
            None => {
                last_name = name.clone();
                first_name = String::from("");
            }
            Some((firstname, surname)) => {
                last_name = surname.to_string();
                first_name = firstname.to_string();
            }
        }
    } else {
        match name.split_once(' ') {
            None => {
                last_name = name.clone();
                first_name = String::from("");
            }
            Some((surname, firstname)) => {
                last_name = surname.to_string();
                first_name = firstname.to_string();
            }
        }
    }

    (first_name, last_name)
}

#[pyfunction]
#[pyo3(signature = (name, firstname_first = true))]
pub fn extract_name_parts(name: String, firstname_first: bool) -> (String, String) {
    let parts = name.split(' ').collect();

    match compute_name_size_hint(&parts, firstname_first) {
        None => extract_name_parts_no_hint(&name, firstname_first),
        Some(hint) => extract_name_parts_with_hint(&parts, firstname_first, hint),
    }
}

#[pyfunction]
#[pyo3(signature = (filename, has_headers = false, delimiter = String::from(";")))]
pub fn load_csv(
    filename: String,
    has_headers: bool,
    delimiter: String,
) -> PyResult<super::csv_file::CsvFile> {
    let path = PathBuf::from(&filename);

    use crate::frontend::csv::{Content, Error, Params};
    let csv_content = Content::from_csv_file(&path).map_err(|e| match e {
        Error::CSV(csv_error) => PyIOError::new_err(csv_error.to_string()),
        Error::IO(io_error) => PyIOError::new_err(io_error.to_string()),
    })?;

    let delimiter_bytes = delimiter.as_bytes();
    if delimiter_bytes.len() != 1 {
        return Err(PyValueError::new_err("delimiter must have a single byte"));
    };

    let params = Params {
        has_headers,
        delimiter: delimiter_bytes[0],
    };

    let csv_extract = csv_content.extract(&params).map_err(|e| match e {
        Error::CSV(csv_error) => PyIOError::new_err(csv_error.to_string()),
        Error::IO(io_error) => PyIOError::new_err(io_error.to_string()),
    })?;

    Ok(super::csv_file::CsvFile::from_extract(csv_extract))
}
