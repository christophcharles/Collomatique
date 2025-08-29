use pyo3::prelude::*;

pub fn python_test() -> PyResult<()> {
    Python::with_gil(|py| {
        let fun: Py<PyAny> = PyModule::from_code_bound(
            py,
            "def example():
                print('Test')",
            "",
            "",
        )?
        .getattr("example")?
        .into();

        fun.call0(py)?;

        Ok(())
    })
}
