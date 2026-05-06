mod balancing;
mod builder;
pub mod convert;
mod groups;
mod helpers;
pub mod ids;
mod misc;
mod native_extras;
mod pairings;
mod periodicity;
mod schedule_structure;
pub mod tools;
mod types;
pub mod vars;

pub use builder::{build_model, build_model_with_log};
pub use types::{
    ConstraintDesc, ExtraVarName, InfeasibleConstraint, PreferenceConstraint,
    ProgressiveConstraint, QualityConstraint, SEVERITY_LEVEL_COUNT, StructuralConstraint,
};

pub use collomatique_ilp_modeler::{
    ConstraintSource, FeasableSolution, InternalVar, MinimalBlame, Model, ModelStats, Solution,
    ViolationImplication,
};
pub use vars::Var;

pub type ColloscopeModel = Model<Var, ExtraVarName, ConstraintDesc>;
pub type ProblemConstraintSource = ConstraintSource<ExtraVarName, ConstraintDesc>;
pub type ProblemInternalVar = InternalVar<Var, ExtraVarName>;
pub type IlpInnerProblem = collomatique_ilp::Problem<ProblemInternalVar, ProblemConstraintSource>;
