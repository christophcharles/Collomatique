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
    Session { token: Token {} }
}

#[pyclass]
pub struct Session {
    token: Token,
}

mod general_planning;
mod subjects;
mod time;

#[pymethods]
impl Session {
    fn periods(self_: PyRef<'_, Self>) -> general_planning::SessionPeriods {
        general_planning::SessionPeriods {
            token: self_.token.clone(),
        }
    }

    fn subjects(self_: PyRef<'_, Self>) -> subjects::SessionSubjects {
        subjects::SessionSubjects {
            token: self_.token.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct Token {}

impl Token {
    fn get_data(&self) -> collomatique_state_colloscopes::Data {
        use crate::rpc::ResultMsg;

        let result =
            crate::rpc::send_rpc(crate::rpc::CmdMsg::GetData).expect("No error for getting data");
        let ResultMsg::Data(serialized_data) = result else {
            panic!("Unexpected response to GetData");
        };
        collomatique_state_colloscopes::Data::from(serialized_data)
    }

    fn send_msg(&self, msg: crate::rpc::CmdMsg) -> crate::rpc::ResultMsg {
        crate::rpc::send_rpc(msg).expect("Valid result message")
    }
}
