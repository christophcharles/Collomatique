use pyo3::prelude::*;

#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(log, m)?)?;

    Ok(())
}

#[pyfunction]
pub fn log(msg: String) {
    use std::io::Write;
    eprint!("{}\r\n", msg);
    std::io::stderr().flush().expect("no error on flush");
}
