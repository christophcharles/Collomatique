use std::path::PathBuf;

use pyo3::prelude::*;

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

    pub fn run(&self, func: &str) -> PyResult<()> {
        self.run_internal(func, None)
    }

    pub fn run_with_csv_file(&self, func: &str, csv_file: super::csv::Extract) -> PyResult<()> {
        self.run_internal(func, Some(csv_file))
    }

    fn run_internal(&self, func: &str, csv_file: Option<super::csv::Extract>) -> PyResult<()> {
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
            Self::call_func(py, &func, csv_file)?;

            Ok(())
        })
    }

    fn call_func(
        py: Python,
        func: &Py<PyAny>,
        _csv_file: Option<super::csv::Extract>,
    ) -> PyResult<()> {
        use pyo3::types::PyTuple;

        let arg_names = extract_function_arguments(py, &func)?;
        let args = PyTuple::new_bound(py, arg_names.iter().map(|_| py.None()));

        func.call1(py, args)?;

        Ok(())
    }
}
