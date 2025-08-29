use pyo3::prelude::*;

#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Session>()?;
    m.add_class::<time::NaiveMondayDate>()?;
    m.add_class::<time::NaiveDate>()?;

    m.add_function(wrap_pyfunction!(log, m)?)?;
    m.add_function(wrap_pyfunction!(current_session, m)?)?;

    Ok(())
}

#[pyfunction]
pub fn log(msg: String) {
    use std::io::Write;
    eprint!("{}\r\n", msg);
    std::io::stderr().flush().expect("no error on flush");
}

#[pyfunction]
pub fn current_session() -> Session {
    Session {}
}

#[pyclass]
pub struct Session {}

mod general_planning;
mod time;

#[pymethods]
impl Session {
    fn periods(_self: PyRef<'_, Self>) -> general_planning::SessionPeriods {
        general_planning::SessionPeriods {}
    }
}
