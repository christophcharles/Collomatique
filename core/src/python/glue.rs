use pyo3::prelude::*;

#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Session>()?;
    m.add_class::<general_planning::Period>()?;
    m.add_class::<subjects::Subject>()?;
    m.add_class::<subjects::SubjectParameters>()?;
    m.add_class::<subjects::SubjectPeriodicity>()?;
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
use general_planning::Period;
mod subjects;
use subjects::Subject;
mod time;

use crate::rpc::cmd_msg::{MsgPeriodId, MsgSubjectId};

#[pymethods]
impl Session {
    fn periods(self_: PyRef<'_, Self>) -> general_planning::SessionPeriods {
        general_planning::SessionPeriods {
            token: self_.token.clone(),
        }
    }

    fn subjects_get(self_: PyRef<'_, Self>) -> Vec<subjects::Subject> {
        self_
            .token
            .get_data()
            .get_subjects()
            .ordered_subject_list
            .iter()
            .map(|(id, data)| Subject {
                id: MsgSubjectId::from(*id).into(),
                parameters: data.parameters.clone().into(),
            })
            .collect()
    }

    fn periods_get_first_week(self_: PyRef<'_, Self>) -> Option<time::NaiveMondayDate> {
        self_
            .token
            .get_data()
            .get_periods()
            .first_week
            .as_ref()
            .map(|x| x.clone().into())
    }

    fn periods_get(self_: PyRef<'_, Self>) -> Vec<Period> {
        self_
            .token
            .get_data()
            .get_periods()
            .ordered_period_list
            .iter()
            .map(|(id, data)| Period {
                id: MsgPeriodId::from(*id).into(),
                weeks_status: data.clone(),
            })
            .collect()
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
