use pyo3::prelude::*;

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

pub fn python_test() -> PyResult<()> {
    Python::with_gil(|py| {
        let python_code = PyModule::from_code_bound(
            py,
            "import collomatique
def example():
    data = collomatique.test_function()
    print(data)",
            "",
            "",
        )?;

        let fun: Py<PyAny> = python_code.getattr("example")?.into();

        fun.call0(py)?;

        Ok(())
    })
}
