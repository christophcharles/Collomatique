use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyMsg {
    Progress(StrategyProgressData),
    Result(StrategyResultData),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializedStrategyRequest {
    serialized: String,
}

impl From<String> for SerializedStrategyRequest {
    fn from(serialized: String) -> Self {
        Self { serialized }
    }
}

impl From<SerializedStrategyRequest> for String {
    fn from(value: SerializedStrategyRequest) -> Self {
        value.serialized
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyProgressData {
    pub message: String,
    pub best_objective: Option<OrderedFloat<f64>>,
    pub feasible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyResultData {
    pub status: StrategyStatus,
    pub objective: Option<OrderedFloat<f64>>,
    pub best_bound: Option<OrderedFloat<f64>>,
    pub solution: Option<Vec<OrderedFloat<f64>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyStatus {
    Optimal,
    Infeasible,
    Stopped,
    Error,
}
