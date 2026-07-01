mod strategy_frame;
mod strategy_status_bar;

pub use strategy_frame::StrategyFrame;
pub use strategy_status_bar::{StrategyStatusBar, StrategyStatusBarOutput};

use collomatique_strategies::{StrategyKind, StrategyProgressData};

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
    /// The conductor was launched: full reset of the display (metrics and echo).
    Clear,
    /// The displayed worker was (re)assigned: `Some` = a substrategy is running,
    /// `None` = the worker went idle. The echo is preserved; the display marks the
    /// boundary itself.
    Assigned(Option<StrategyName>),
    StrategyUpdate(StrategyProgressData),
    ToggleDebug(bool),
}
