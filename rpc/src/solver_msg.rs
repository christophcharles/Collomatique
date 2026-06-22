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
    pub time_limit_seconds: Option<u32>,
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
    pub best_obj: OrderedFloat<f64>,
    pub best_bound: OrderedFloat<f64>,
    pub node_count: u64,
    pub solutions_found: u64,
    pub incumbent_info: Option<SolverIncumbentInfo>,
    pub incumbent_solution: Option<Vec<OrderedFloat<f64>>>,
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
