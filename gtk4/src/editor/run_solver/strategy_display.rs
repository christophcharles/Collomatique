mod strategy_frame;
mod strategy_status_bar;

pub use strategy_frame::StrategyFrame;
pub use strategy_status_bar::{StrategyStatusBar, StrategyStatusBarOutput};

use collomatique_strategies::{SolveStatus, StrategyKind, StrategyProgressData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyName {
    Default,
}

pub fn strategy_name_from_kind(kind: &StrategyKind) -> StrategyName {
    match kind {
        StrategyKind::Default(_) => StrategyName::Default,
        StrategyKind::NoObjective(_) => todo!(),
        StrategyKind::NoObjectiveStarter(_) => todo!(),
        StrategyKind::Conductor(_) => todo!(),
    }
}

#[derive(Debug, Clone)]
pub enum StrategyDisplayInput {
    Echo(String),
    Clear(StrategyName),
    StrategyUpdate(Result<StrategyProgressData, String>),
    Finished(SolveStatus),
    ToggleDebug(bool),
}
