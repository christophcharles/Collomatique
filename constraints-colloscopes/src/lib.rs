mod builder;
mod forbidden_groups;
mod group_count_per_interrogation;
mod groups_filled_by_ascending_order;
pub mod ids;
mod native_extras;
mod one_interrogation_at_once;
mod problem;
mod students_have_groups;
mod students_per_group;
mod students_per_group_for_subject;
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
