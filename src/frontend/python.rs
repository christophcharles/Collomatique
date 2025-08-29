use std::{collections::BTreeSet, path::PathBuf};

use pyo3::{prelude::*, types::IntoPyDict};

mod csv_file;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonCode {
    code: String,
    file: PathBuf,
}

fn extract_function_arguments(py: Python, func: &Py<PyAny>) -> PyResult<Vec<String>> {
    use pyo3::types::{PyString, PyTuple};

    let code_attr = func.getattr(py, "__code__")?;
    let varnames_any = code_attr.getattr(py, "co_varnames")?;
    let varnames: &Bound<PyTuple> = varnames_any.downcast_bound(py)?;

    let mut output = vec![];

    let len = varnames.to_list().len();
    for i in 0..len {
        let item_any = varnames.get_item(i)?;
        let item: &Bound<PyString> = item_any.downcast()?;

        output.push(String::from(item.to_str()?));
    }

    Ok(output)
}

impl PythonCode {
    pub fn from_code(code: &str) -> Self {
        PythonCode {
            code: code.to_string(),
            file: PathBuf::new(),
        }
    }

    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        use std::fs::File;
        use std::io::Read;

        let mut python_code = PythonCode {
            code: String::new(),
            file: path.to_path_buf(),
        };

        let mut file = File::open(path)?;
        file.read_to_string(&mut python_code.code)?;

        Ok(python_code)
    }

    pub fn run(&self) -> PyResult<()> {
        self.run_internal(None)
    }

    pub fn run_with_csv_file(&self, csv_extract: super::csv::Extract) -> PyResult<()> {
        self.run_internal(Some(csv_extract))
    }

    pub fn run_func(&self, func: &str) -> PyResult<()> {
        self.run_func_internal(func, None)
    }

    pub fn run_func_with_csv_file(
        &self,
        func: &str,
        csv_extract: super::csv::Extract,
    ) -> PyResult<()> {
        self.run_func_internal(func, Some(csv_extract))
    }

    fn run_internal(&self, csv_extract: Option<super::csv::Extract>) -> PyResult<()> {
        Python::with_gil(|py| {
            let mut vars = vec![];

            if let Some(extract) = csv_extract {
                let csv_file = Py::new(py, csv_file::CsvFile::from_extract(extract))?.into_any();
                vars.push(("csv", csv_file));
            }

            let locals = vars.into_py_dict_bound(py);

            py.run_bound(&self.code, None, Some(&locals))?;

            Ok(())
        })
    }

    fn run_func_internal(
        &self,
        func: &str,
        csv_extract: Option<super::csv::Extract>,
    ) -> PyResult<()> {
        Python::with_gil(|py| {
            let python_code = PyModule::from_code_bound(
                py,
                &self.code,
                &self.file.to_string_lossy(),
                &self
                    .file
                    .as_path()
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy(),
            )?;

            let func: Py<PyAny> = python_code.getattr(func)?.into();
            Self::call_func(py, &func, csv_extract)?;

            Ok(())
        })
    }

    fn call_func(
        py: Python,
        func: &Py<PyAny>,
        csv_extract: Option<super::csv::Extract>,
    ) -> PyResult<()> {
        use pyo3::types::PyTuple;

        let csv_file: PyObject = match csv_extract {
            Some(extract) => Py::new(py, csv_file::CsvFile::from_extract(extract))?.into_any(),
            None => py.None(),
        };
        let csv_names = BTreeSet::from(["csv", "csv_file", "csv_data"]);

        let arg_names = extract_function_arguments(py, &func)?;
        let args = PyTuple::new_bound(
            py,
            arg_names.iter().map(|name| {
                if csv_names.contains(name.as_str()) {
                    csv_file.clone_ref(py)
                } else {
                    py.None()
                }
            }),
        );

        func.call1(py, args)?;

        Ok(())
    }
}
