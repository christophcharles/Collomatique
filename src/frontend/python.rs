use pyo3::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonCode {
    code: String,
}

impl PythonCode {
    pub fn from_code(code: &str) -> Self {
        PythonCode {
            code: code.to_string(),
        }
    }

    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        use std::fs::File;
        use std::io::Read;

        let mut python_code = PythonCode {
            code: String::new(),
        };

        let mut file = File::open(path)?;
        file.read_to_string(&mut python_code.code)?;

        Ok(python_code)
    }

    pub fn run(&self) -> PyResult<()> {
        Python::with_gil(|py| py.run_bound(&self.code, None, None))
    }
}

#[pyfunction]
fn test_function() -> String {
    "test from rust code".into()
}

#[pymodule]
fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(test_function, m)?)
}

pub fn initialize() {
    pyo3::append_to_inittab!(collomatique);
    pyo3::prepare_freethreaded_python();
}
