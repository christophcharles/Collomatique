mod builder;
pub mod ids;
mod native_extras;
mod problem;
mod types;

pub use builder::{ProblemBuilder, default_problem_builder};
pub use problem::{FeasableSolution, Problem, Solution};
pub use problem::{IlpInnerProblem, ProblemConstraintSource, ProblemInternalVar};
pub use types::{ConstraintDesc, ReifiedVarName};

pub use collomatique_binding_colloscopes::scripts::{
    SimpleScriptError, get_default_main_module, get_modules,
};
pub use collomatique_binding_colloscopes::vars::Var;
pub use collomatique_ilp_modeler::ConstraintSource;
