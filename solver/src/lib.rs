//! Collomatique-solver
//! ---
//!
//! This crate contain the generic code used to convert a problem
//! and its potential solution into an ILP formulation usable by [collomatique_ilp].
//! The code is split into two modules: [generics] and [solver].
//!
//! The main struct is [ProblemBuilder] and you should start from there to see how this crate
//! works. If you want to implement a problem, you should implement [BaseProblem] on some structure
//! and possibly [ProblemConstraints] on a few others.
//! A generic usage example is provided in the [examples::simple_schedule] module.
//!
//! However a simpler route is to use the [simplified] module and rather implement [SimpleBaseProblem]
//! and [SimpleProblemConstraints]. This will auto-implement [BaseProblem] and [ProblemConstraints]
//! but will allow a simplified definition of structure variables using [tools::AggregatedVariables].

pub mod examples;
pub mod generics;
pub mod simplified;
pub mod solver;
pub mod tools;

pub use generics::{BaseProblem, BaseVariable, ExtraVariable, ProblemConstraints, SoftConstraints};
pub use simplified::{SimpleBaseProblem, SimpleProblemConstraints};
pub use solver::{
    DecoratedCompleteSolution, Problem, ProblemBuilder, TimeLimitSolution, Translator,
};
