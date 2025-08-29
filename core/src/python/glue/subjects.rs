use crate::rpc::cmd_msg::MsgSubjectId;

use super::*;
use pyo3::{exceptions::PyValueError, types::PyString};

use std::num::NonZeroU32;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectId {
    id: crate::rpc::cmd_msg::MsgSubjectId,
}

#[pymethods]
impl SubjectId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgSubjectId> for SubjectId {
    fn from(value: &crate::rpc::cmd_msg::MsgSubjectId) -> Self {
        SubjectId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgSubjectId> for SubjectId {
    fn from(value: crate::rpc::cmd_msg::MsgSubjectId) -> Self {
        SubjectId::from(&value)
    }
}

impl From<&SubjectId> for crate::rpc::cmd_msg::MsgSubjectId {
    fn from(value: &SubjectId) -> Self {
        value.id.clone()
    }
}

impl From<SubjectId> for crate::rpc::cmd_msg::MsgSubjectId {
    fn from(value: SubjectId) -> Self {
        crate::rpc::cmd_msg::MsgSubjectId::from(&value)
    }
}

#[pyclass]
pub struct SessionSubjects {
    pub(super) token: super::Token,
}

#[pymethods]
impl SessionSubjects {
    fn get_subjects(self_: PyRef<'_, Self>) -> Vec<Subject> {
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
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    #[pyo3(set, get)]
    id: SubjectId,
    #[pyo3(set, get)]
    parameters: SubjectParameters,
}

#[pymethods]
impl Subject {
    #[new]
    fn new(id: SubjectId, parameters: SubjectParameters) -> Self {
        Subject { id, parameters }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectParameters {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    students_per_group_min: NonZeroU32,
    #[pyo3(set, get)]
    students_per_group_max: NonZeroU32,
    #[pyo3(set, get)]
    groups_per_interrogation_min: NonZeroU32,
    #[pyo3(set, get)]
    groups_per_interrogation_max: NonZeroU32,
    #[pyo3(set, get)]
    duration: NonZeroU32,
    #[pyo3(set, get)]
    take_duration_into_account: bool,
    #[pyo3(set, get)]
    periodicity: SubjectPeriodicity,
}

impl From<collomatique_state_colloscopes::SubjectParameters> for SubjectParameters {
    fn from(value: collomatique_state_colloscopes::SubjectParameters) -> Self {
        SubjectParameters {
            name: value.name,
            students_per_group_min: value.students_per_group.start().clone(),
            students_per_group_max: value.students_per_group.end().clone(),
            groups_per_interrogation_min: value.students_per_group.start().clone(),
            groups_per_interrogation_max: value.students_per_group.end().clone(),
            duration: value.duration.get(),
            take_duration_into_account: value.take_duration_into_account,
            periodicity: value.periodicity.into(),
        }
    }
}

#[pymethods]
impl SubjectParameters {
    #[new]
    fn new(name: String) -> Self {
        SubjectParameters {
            name,
            students_per_group_min: NonZeroU32::new(2).unwrap(),
            students_per_group_max: NonZeroU32::new(3).unwrap(),
            groups_per_interrogation_min: NonZeroU32::new(1).unwrap(),
            groups_per_interrogation_max: NonZeroU32::new(1).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            take_duration_into_account: true,
            periodicity: SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
            },
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubjectPeriodicity {
    OnceForEveryBlockOfWeeks {
        weeks_per_block: u32,
    },
    ExactlyPeriodic {
        periodicity_in_weeks: NonZeroU32,
    },
    AmountInYear {
        interrogation_count_in_year_min: u32,
        interrogation_count_in_year_max: u32,
        minimum_week_separation: u32,
    },
    OnceForEveryArbitraryBlock {
        weeks_at_start_of_new_block: std::collections::BTreeSet<usize>,
    },
}

impl From<collomatique_state_colloscopes::SubjectPeriodicity> for SubjectPeriodicity {
    fn from(value: collomatique_state_colloscopes::SubjectPeriodicity) -> Self {
        match value {
            collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block,
            } => SubjectPeriodicity::OnceForEveryBlockOfWeeks { weeks_per_block },
            collomatique_state_colloscopes::SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            },
            collomatique_state_colloscopes::SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            } => SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year_min: interrogation_count_in_year.start().clone(),
                interrogation_count_in_year_max: interrogation_count_in_year.end().clone(),
                minimum_week_separation,
            },
            collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryArbitraryBlock {
                weeks_at_start_of_new_block,
            } => SubjectPeriodicity::OnceForEveryArbitraryBlock {
                weeks_at_start_of_new_block,
            },
        }
    }
}

#[pymethods]
impl SubjectPeriodicity {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}
