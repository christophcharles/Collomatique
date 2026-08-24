use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    GetData,
    SetData(super::InternalDataStream),
    Solver(super::SolverMsg),
    Strategy(super::StrategyMsg),
}
