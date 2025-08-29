use super::*;
use crate::frontend::csv;
use std::collections::BTreeMap;

#[pyclass]
pub struct CsvFile {
    #[pyo3(get)]
    headers: Option<Vec<String>>,
    #[pyo3(get)]
    content: Vec<Vec<String>>,
    #[pyo3(get)]
    map: Option<Vec<BTreeMap<String, Vec<String>>>>,
}

impl CsvFile {
    fn complete_line(mut line: Vec<String>, len: usize) -> Vec<String> {
        assert!(line.len() <= len);

        line.resize(len, String::new());

        line
    }

    pub fn from_extract(csv_extract: csv::Extract) -> Self {
        let max_len = csv_extract
            .lines
            .iter()
            .chain(csv_extract.headers.iter())
            .map(|x| x.len())
            .max()
            .unwrap_or(0);

        let headers = csv_extract.headers.map(|x| Self::complete_line(x, max_len));
        let content = csv_extract
            .lines
            .into_iter()
            .map(|x| Self::complete_line(x, max_len))
            .collect();
        let map = Self::build_map_from_completed_lines(&headers, &content);

        CsvFile {
            headers,
            content,
            map,
        }
    }

    fn build_map_from_completed_lines(
        headers: &Option<Vec<String>>,
        content: &Vec<Vec<String>>,
    ) -> Option<Vec<BTreeMap<String, Vec<String>>>> {
        if headers.is_none() {
            return None;
        }

        let headers = headers.as_ref().unwrap();

        Some(
            content
                .iter()
                .map(|line| {
                    let mut line_map = BTreeMap::<String, Vec<String>>::new();

                    for (header, cell) in headers.iter().zip(line.iter()) {
                        match line_map.get_mut(header) {
                            Some(list) => {
                                list.push(cell.clone());
                            }
                            None => {
                                line_map.insert(header.clone(), vec![cell.clone()]);
                            }
                        }
                    }

                    line_map
                })
                .collect(),
        )
    }
}
