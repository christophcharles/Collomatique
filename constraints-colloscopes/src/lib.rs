mod balancing;
mod builder;
pub mod config;
pub mod convert;
mod extras;
mod groups;
mod helpers;
pub mod ids;
mod incremental;
mod misc;
mod pairings;
mod periodicity;
mod schedule_structure;
pub mod tools;
mod types;
pub mod vars;
mod weights;

pub use builder::{build_model, build_model_with_log};
pub use config::{
    ConfiguredConstraintDesc, ConfiguredExtra, GroupListRecompute, GroupListSolveData,
    PeriodSolveData, SolveConfig,
};
pub use incremental::build_incremental_epochs;
pub use types::{
    ConstraintDesc, ExtraVarName, InfeasibleConstraint, PreferenceConstraint,
    ProgressiveConstraint, QualityConstraint, SEVERITY_LEVEL_COUNT, SeverityLevel,
    StructuralConstraint,
};

pub use collomatique_ilp_modeler::{
    ConstraintSource, FeasibleSolution, InternalVar, MinimalBlame, Model, ModelStats, Solution,
    ViolationImplication,
};
pub use vars::Var;

pub type ColloscopeModel = Model<Var, ExtraVarName, ConstraintDesc>;
/// The model produced by [`SolveConfig::build_model`]: the base [`ColloscopeModel`]'s
/// variable/constraint spaces extended with the configuration wrappers
/// ([`ConfiguredExtra`] penalty variables and [`ConfiguredConstraintDesc`] pin/anchor
/// descriptions).
///
/// [`SolveConfig::build_model`]: config::SolveConfig::build_model
pub type ConfiguredColloscopeModel = Model<Var, ConfiguredExtra, ConfiguredConstraintDesc>;
pub type ProblemConstraintSource = ConstraintSource<ExtraVarName, ConstraintDesc>;
pub type ProblemInternalVar = InternalVar<Var, ExtraVarName>;
pub type IlpInnerProblem = collomatique_ilp::Problem<ProblemInternalVar, ProblemConstraintSource>;
