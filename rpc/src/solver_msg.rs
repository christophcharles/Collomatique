use std::fmt;

use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolverMsg {
    Progress(SolverProgressData),
    Result(SolverResultData),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializedIlpProblem {
    serialized: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IlpSolveRequest {
    pub problem_desc: collomatique_ilp::ProblemDesc,
    pub warm_start: Option<Vec<f64>>,
    pub time_limit: collomatique_time::TimeLimit,
    pub disable_logging: bool,
}

impl From<&IlpSolveRequest> for SerializedIlpProblem {
    fn from(request: &IlpSolveRequest) -> Self {
        SerializedIlpProblem {
            serialized: serde_json::to_string(request)
                .expect("Serialization of IlpSolveRequest should never fail"),
        }
    }
}

impl From<IlpSolveRequest> for SerializedIlpProblem {
    fn from(request: IlpSolveRequest) -> Self {
        SerializedIlpProblem::from(&request)
    }
}

impl From<SerializedIlpProblem> for IlpSolveRequest {
    fn from(value: SerializedIlpProblem) -> Self {
        serde_json::from_str(&value.serialized)
            .expect("Data from SerializedIlpProblem should always be deserializable")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverProgressData {
    /// Objective of the current incumbent, or `None` if none has been found yet.
    /// An objective only exists as a property of an incumbent — this is never a
    /// free-floating running value. (`Option` also keeps infinities out of the
    /// serialized form.)
    pub best_obj: Option<OrderedFloat<f64>>,
    pub best_bound: OrderedFloat<f64>,
    pub node_count: u64,
    pub solutions_found: u64,
    pub incumbent_info: Option<SolverIncumbentInfo>,
    pub incumbent_solution: Option<Vec<OrderedFloat<f64>>>,
}

impl fmt::Display for SolverProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "obj=")?;
        match self.best_obj {
            Some(obj) => write!(f, "{:.4}", obj.into_inner())?,
            None => write!(f, "—")?,
        }
        write!(
            f,
            " bound={:.4} nodes={} solutions={} incumbent={}",
            self.best_bound.into_inner(),
            self.node_count,
            self.solutions_found,
            if self.incumbent_solution.is_some() {
                "yes"
            } else {
                "no"
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverIncumbentInfo {
    pub objective: OrderedFloat<f64>,
    pub feasible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverResultData {
    pub status: SolverStatus,
    pub obj_value: Option<OrderedFloat<f64>>,
    pub best_bound: Option<OrderedFloat<f64>>,
    pub node_count: u64,
    pub solution: Option<Vec<OrderedFloat<f64>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolverStatus {
    Optimal,
    Infeasible,
    Stopped,
    Error,
}
