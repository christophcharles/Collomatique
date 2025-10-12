use std::collections::BTreeSet;

use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId {
    id: collomatique_state_colloscopes::RuleId,
}

#[pymethods]
impl RuleId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::RuleId> for RuleId {
    fn from(value: &collomatique_state_colloscopes::RuleId) -> Self {
        RuleId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::RuleId> for RuleId {
    fn from(value: collomatique_state_colloscopes::RuleId) -> Self {
        RuleId::from(&value)
    }
}

impl From<&RuleId> for collomatique_state_colloscopes::RuleId {
    fn from(value: &RuleId) -> Self {
        value.id.clone()
    }
}

impl From<RuleId> for collomatique_state_colloscopes::RuleId {
    fn from(value: RuleId) -> Self {
        collomatique_state_colloscopes::RuleId::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(set, get)]
    pub logic_rule: LogicRule,
    #[pyo3(set, get)]
    pub excluded_periods: BTreeSet<PeriodId>,
}

#[pymethods]
impl Rule {
    #[new]
    fn new(name: String, logic_rule: LogicRule) -> Self {
        Rule {
            name,
            logic_rule,
            excluded_periods: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass(eq)]
#[derive(Clone, Debug)]
pub enum LogicRule {
    And(Py<LogicRule>, Py<LogicRule>),
    Or(Py<LogicRule>, Py<LogicRule>),
    Not(Py<LogicRule>),
    Variable(slots::SlotId),
}

impl PartialEq for LogicRule {
    fn eq(&self, other: &Self) -> bool {
        match self {
            LogicRule::And(pl1a, pl2a) => match other {
                LogicRule::And(pl1b, pl2b) => {
                    let l1a = Python::with_gil(|py| pl1a.borrow(py).clone());
                    let l2a = Python::with_gil(|py| pl2a.borrow(py).clone());
                    let l1b = Python::with_gil(|py| pl1b.borrow(py).clone());
                    let l2b = Python::with_gil(|py| pl2b.borrow(py).clone());
                    ((l1a == l1b) && (l2a == l2b)) || ((l1a == l2b) && (l2a == l1b))
                }
                _ => false,
            },
            LogicRule::Or(pl1a, pl2a) => match other {
                LogicRule::Or(pl1b, pl2b) => {
                    let l1a = Python::with_gil(|py| pl1a.borrow(py).clone());
                    let l2a = Python::with_gil(|py| pl2a.borrow(py).clone());
                    let l1b = Python::with_gil(|py| pl1b.borrow(py).clone());
                    let l2b = Python::with_gil(|py| pl2b.borrow(py).clone());
                    ((l1a == l1b) && (l2a == l2b)) || ((l1a == l2b) && (l2a == l1b))
                }
                _ => false,
            },
            LogicRule::Not(pla) => match other {
                LogicRule::Not(plb) => {
                    let la = Python::with_gil(|py| pla.borrow(py).clone());
                    let lb = Python::with_gil(|py| plb.borrow(py).clone());
                    la == lb
                }
                _ => false,
            },
            LogicRule::Variable(id) => match other {
                LogicRule::Variable(id2) => id == id2,
                _ => false,
            },
        }
    }
}

impl Eq for LogicRule {}

#[pymethods]
impl LogicRule {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    TryFrom<
        collomatique_state_colloscopes::rules::LogicRule<collomatique_state_colloscopes::SlotId>,
    > for LogicRule
{
    type Error = PyErr;
    fn try_from(
        value: collomatique_state_colloscopes::rules::LogicRule<
            collomatique_state_colloscopes::SlotId,
        >,
    ) -> PyResult<Self> {
        use collomatique_state_colloscopes::rules;
        match value {
            rules::LogicRule::And(l1, l2) => {
                let pl1 = LogicRule::try_from(*l1)?;
                let pl2 = LogicRule::try_from(*l2)?;
                Python::with_gil(|py| Ok(LogicRule::And(Py::new(py, pl1)?, Py::new(py, pl2)?)))
            }
            rules::LogicRule::Or(l1, l2) => {
                let pl1 = LogicRule::try_from(*l1)?;
                let pl2 = LogicRule::try_from(*l2)?;
                Python::with_gil(|py| Ok(LogicRule::Or(Py::new(py, pl1)?, Py::new(py, pl2)?)))
            }
            rules::LogicRule::Not(l) => {
                let pl = LogicRule::try_from(*l)?;
                Python::with_gil(|py| Ok(LogicRule::Not(Py::new(py, pl)?)))
            }
            rules::LogicRule::Variable(id) => Ok(LogicRule::Variable(id.into())),
        }
    }
}

impl From<LogicRule>
    for collomatique_state_colloscopes::rules::LogicRule<collomatique_state_colloscopes::SlotId>
{
    fn from(value: LogicRule) -> Self {
        match value {
            LogicRule::And(pl1, pl2) => {
                let l1 = Python::with_gil(|py| pl1.borrow(py).clone());
                let l2 = Python::with_gil(|py| pl2.borrow(py).clone());
                collomatique_state_colloscopes::rules::LogicRule::And(
                    Box::new(l1.into()),
                    Box::new(l2.into()),
                )
            }
            LogicRule::Or(pl1, pl2) => {
                let l1 = Python::with_gil(|py| pl1.borrow(py).clone());
                let l2 = Python::with_gil(|py| pl2.borrow(py).clone());
                collomatique_state_colloscopes::rules::LogicRule::Or(
                    Box::new(l1.into()),
                    Box::new(l2.into()),
                )
            }
            LogicRule::Not(pl) => {
                let l = Python::with_gil(|py| pl.borrow(py).clone());
                collomatique_state_colloscopes::rules::LogicRule::Not(Box::new(l.into()))
            }
            LogicRule::Variable(id) => {
                collomatique_state_colloscopes::rules::LogicRule::Variable(id.into())
            }
        }
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColloscopeRuleId {
    id: collomatique_state_colloscopes::ColloscopeRuleId,
}

#[pymethods]
impl ColloscopeRuleId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::ColloscopeRuleId> for ColloscopeRuleId {
    fn from(value: &collomatique_state_colloscopes::ColloscopeRuleId) -> Self {
        ColloscopeRuleId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::ColloscopeRuleId> for ColloscopeRuleId {
    fn from(value: collomatique_state_colloscopes::ColloscopeRuleId) -> Self {
        ColloscopeRuleId::from(&value)
    }
}

impl From<&ColloscopeRuleId> for collomatique_state_colloscopes::ColloscopeRuleId {
    fn from(value: &ColloscopeRuleId) -> Self {
        value.id.clone()
    }
}

impl From<ColloscopeRuleId> for collomatique_state_colloscopes::ColloscopeRuleId {
    fn from(value: ColloscopeRuleId) -> Self {
        collomatique_state_colloscopes::ColloscopeRuleId::from(&value)
    }
}

#[pyclass(eq)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeRule {
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(set, get)]
    pub logic_rule: ColloscopeLogicRule,
    #[pyo3(set, get)]
    pub excluded_periods: BTreeSet<ColloscopePeriodId>,
}

#[pymethods]
impl ColloscopeRule {
    #[new]
    fn new(name: String, logic_rule: ColloscopeLogicRule) -> Self {
        ColloscopeRule {
            name,
            logic_rule,
            excluded_periods: BTreeSet::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass(eq)]
#[derive(Clone, Debug)]
pub enum ColloscopeLogicRule {
    And(Py<ColloscopeLogicRule>, Py<ColloscopeLogicRule>),
    Or(Py<ColloscopeLogicRule>, Py<ColloscopeLogicRule>),
    Not(Py<ColloscopeLogicRule>),
    Variable(slots::ColloscopeSlotId),
}

impl PartialEq for ColloscopeLogicRule {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ColloscopeLogicRule::And(pl1a, pl2a) => match other {
                ColloscopeLogicRule::And(pl1b, pl2b) => {
                    let l1a = Python::with_gil(|py| pl1a.borrow(py).clone());
                    let l2a = Python::with_gil(|py| pl2a.borrow(py).clone());
                    let l1b = Python::with_gil(|py| pl1b.borrow(py).clone());
                    let l2b = Python::with_gil(|py| pl2b.borrow(py).clone());
                    ((l1a == l1b) && (l2a == l2b)) || ((l1a == l2b) && (l2a == l1b))
                }
                _ => false,
            },
            ColloscopeLogicRule::Or(pl1a, pl2a) => match other {
                ColloscopeLogicRule::Or(pl1b, pl2b) => {
                    let l1a = Python::with_gil(|py| pl1a.borrow(py).clone());
                    let l2a = Python::with_gil(|py| pl2a.borrow(py).clone());
                    let l1b = Python::with_gil(|py| pl1b.borrow(py).clone());
                    let l2b = Python::with_gil(|py| pl2b.borrow(py).clone());
                    ((l1a == l1b) && (l2a == l2b)) || ((l1a == l2b) && (l2a == l1b))
                }
                _ => false,
            },
            ColloscopeLogicRule::Not(pla) => match other {
                ColloscopeLogicRule::Not(plb) => {
                    let la = Python::with_gil(|py| pla.borrow(py).clone());
                    let lb = Python::with_gil(|py| plb.borrow(py).clone());
                    la == lb
                }
                _ => false,
            },
            ColloscopeLogicRule::Variable(id) => match other {
                ColloscopeLogicRule::Variable(id2) => id == id2,
                _ => false,
            },
        }
    }
}

impl Eq for ColloscopeLogicRule {}

#[pymethods]
impl ColloscopeLogicRule {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    TryFrom<
        collomatique_state_colloscopes::rules::LogicRule<
            collomatique_state_colloscopes::ColloscopeSlotId,
        >,
    > for ColloscopeLogicRule
{
    type Error = PyErr;
    fn try_from(
        value: collomatique_state_colloscopes::rules::LogicRule<
            collomatique_state_colloscopes::ColloscopeSlotId,
        >,
    ) -> PyResult<Self> {
        use collomatique_state_colloscopes::rules;
        match value {
            rules::LogicRule::And(l1, l2) => {
                let pl1 = ColloscopeLogicRule::try_from(*l1)?;
                let pl2 = ColloscopeLogicRule::try_from(*l2)?;
                Python::with_gil(|py| {
                    Ok(ColloscopeLogicRule::And(
                        Py::new(py, pl1)?,
                        Py::new(py, pl2)?,
                    ))
                })
            }
            rules::LogicRule::Or(l1, l2) => {
                let pl1 = ColloscopeLogicRule::try_from(*l1)?;
                let pl2 = ColloscopeLogicRule::try_from(*l2)?;
                Python::with_gil(|py| {
                    Ok(ColloscopeLogicRule::Or(
                        Py::new(py, pl1)?,
                        Py::new(py, pl2)?,
                    ))
                })
            }
            rules::LogicRule::Not(l) => {
                let pl = ColloscopeLogicRule::try_from(*l)?;
                Python::with_gil(|py| Ok(ColloscopeLogicRule::Not(Py::new(py, pl)?)))
            }
            rules::LogicRule::Variable(id) => Ok(ColloscopeLogicRule::Variable(id.into())),
        }
    }
}

impl From<ColloscopeLogicRule>
    for collomatique_state_colloscopes::rules::LogicRule<
        collomatique_state_colloscopes::ColloscopeSlotId,
    >
{
    fn from(value: ColloscopeLogicRule) -> Self {
        match value {
            ColloscopeLogicRule::And(pl1, pl2) => {
                let l1 = Python::with_gil(|py| pl1.borrow(py).clone());
                let l2 = Python::with_gil(|py| pl2.borrow(py).clone());
                collomatique_state_colloscopes::rules::LogicRule::And(
                    Box::new(l1.into()),
                    Box::new(l2.into()),
                )
            }
            ColloscopeLogicRule::Or(pl1, pl2) => {
                let l1 = Python::with_gil(|py| pl1.borrow(py).clone());
                let l2 = Python::with_gil(|py| pl2.borrow(py).clone());
                collomatique_state_colloscopes::rules::LogicRule::Or(
                    Box::new(l1.into()),
                    Box::new(l2.into()),
                )
            }
            ColloscopeLogicRule::Not(pl) => {
                let l = Python::with_gil(|py| pl.borrow(py).clone());
                collomatique_state_colloscopes::rules::LogicRule::Not(Box::new(l.into()))
            }
            ColloscopeLogicRule::Variable(id) => {
                collomatique_state_colloscopes::rules::LogicRule::Variable(id.into())
            }
        }
    }
}
