use std::path::PathBuf;

use pyo3::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonCode {
    code: String,
    file: PathBuf,
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

            let fun: Py<PyAny> = python_code.getattr(func)?.into();

            fun.call0(py)?;

            Ok(())
        })
    }
}
