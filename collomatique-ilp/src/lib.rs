//! The collomatique-ilp crate contains the code of collomatique
//! pertaining to representing Integer Linear Programming problems
//! pertinent for the collomatique algorithm.
//!
//! ILP problems (or MILP problems for Mixed-ILP) are linear problems
//! containing equations and inequations with several unknown variables.
//! There are of the form:
//!
//! a<sub>11</sub> x<sub>1</sub> + a<sub>12</sub> x<sub>2</sub> + a<sub>13</sub> x<sub>3</sub> + ... <= b<sub>1</sub>\
//! a<sub>21</sub> x<sub>1</sub> + a<sub>22</sub> x<sub>2</sub> + a<sub>23</sub> x<sub>3</sub> + ... <= b<sub>2</sub>\
//! ...
//!
//! The a<sub>ij</sub> are coefficients and the b<sub>i</sub> are fixed values. The unknown variables are denoted
//! x<sub>i</sub> in the previous equations.
//! The equations are equalities or (large) inequalities.
//!
//! On top of these equations, we add possibles ranges for the various variables :
//!
//! m<sub>1</sub> <= x<sub>1</sub> <= M<sub>1</sub>\
//! m<sub>2</sub> <= x<sub>2</sub> <= M<sub>2</sub>\
//! ...
//!
//! and a (linear) objective function that we try to minimize or maximize :
//!
//! c<sub>1</sub> x<sub>1</sub> + c<sub>2</sub> x<sub>2</sub> + c<sub>3</sub> x<sub>3</sub> + ...
//!
//! where the c<sub>i</sub> are fixed coefficients.
//!
//! Such a type of problem is called a Linear Programming (LP) problem.
//!
//! An Integer Linear Programming problem adds the requirement that all (or only some of
//! them for a Mixed-ILP problem) the variables are integers.
//!
//! It turns out that a lot of problems can be represented this way (See the wikipedia page: <https://en.wikipedia.org/wiki/Integer_programming>).
//! In fact, such a problem is NP-hard and so, solving it means we can solve any NP problem.
//!
//! This covers a *lot* of problems. But in our case, it is particularly suited to the representation of
//! scheduling problems (see for instance <https://doi.org/10.1016/S0377-2217(03)00095-X>, <https://doi.org/10.1007/s11750-015-0366-z> or
//! <https://doi.org/10.1016/j.ejor.2012.11.029>). Though it is not the only way to solve our colloscope problem, it is the one we chose
//! and this crate contains only the mathematical tools for it.
//!
//! There are already a few crates to represent ILP problems in Rust, most notably good_lp (<https://docs.rs/good_lp/latest/good_lp/>).
//! There are also crates for solving ILP problems either with their own implementation of an algorithm, for instance microlp
//! <https://docs.rs/microlp/latest/microlp/>, or as interfaces to code in other languages (usually in C or C++), for instance
//! highs <https://docs.rs/highs/latest/highs/> and coinc_cbc <https://docs.rs/coin_cbc/latest/coin_cbc/>.
//!
//! We don't try to reinvent the wheel here. In fact, we do use such crates as backend. However, this crate was developed to serve two other goals:
//! - to have an internal representation with more generic variable names that is easier to handler in the main collomatique code.
//! - to have a possibility to simply check if a possible solution is indeed a solution without calling a solver. And in case it is not
//!   a solution to be able to trace which constraints are not satisfied.
//!
//! The normal workflow with this crate is to start with a [ProblemBuilder].

pub mod linexpr;

use std::collections::BTreeMap;
use thiserror::Error;

pub use linexpr::{Constraint, LinExpr};

/// Trait for displayable, ordonnable, comparable, clonable, sendable data
///
/// The code is using generics to allow for specialized types. So it is for instance
/// possible to use enums for variable names or constraint descriptions.
///
/// We use this trait to check that indeed some basic properties are garanteed.
pub trait UsableData:
    std::fmt::Debug
    + std::fmt::Display
    + PartialOrd
    + Ord
    + PartialEq
    + Eq
    + Clone
    + for<'a> From<&'a Self>
    + Send
    + Sync
{
}

impl<
        T: std::fmt::Debug
            + std::fmt::Display
            + PartialOrd
            + Ord
            + PartialEq
            + Eq
            + Clone
            + for<'a> From<&'a T>
            + Send
            + Sync,
    > UsableData for T
{
}

/// Represents the different possible types of variables
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariableType {
    /// Continuous variable.
    ///
    /// In practice, this is represented by an f64
    #[default]
    Continuous,
    /// Integer variable.
    ///
    /// It is still represented by an f64, but the possible
    /// values will be restricted to integers (positive or negative).
    Integer,
    /// Binary variable.
    ///
    /// It is still represented by an f64, but the possible
    /// values will be restricted to 0 and 1.
    Binary,
}

/// Complete description of the possible range of values for a variable.
///
/// The description is built using a builder pattern by starting with a call to
/// [Variable::integer], [Variable::binary] or [Variable::continuous].
/// By default, there are no other constraints on the range of possible values
/// (except those implied by the type of variable - for instance a binary variable is
/// always bigger than 0 and less than 1).
///
/// Further constraints can be imposed with [Variable::min] and [Variable::max].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variable {
    var_type: VariableType,
    min: Option<ordered_float::OrderedFloat<f64>>,
    max: Option<ordered_float::OrderedFloat<f64>>,
}

impl Default for Variable {
    /// By default, a variable is continuous without any range restriction.
    fn default() -> Self {
        Variable {
            var_type: VariableType::default(),
            min: None,
            max: None,
        }
    }
}

impl Variable {
    /// Builds the description of an integer variable.
    ///
    /// The range of possible values can be restricted using
    /// [Variable::min] and [Variable::max].
    ///
    /// ```
    /// # use collomatique_ilp::{Variable, VariableType};
    /// let var_desc = Variable::integer();
    /// assert_eq!(var_desc.get_type(), VariableType::Integer);
    /// ```
    pub fn integer() -> Self {
        Variable {
            var_type: VariableType::Integer,
            min: None,
            max: None,
        }
    }

    /// Builds the description of a binary variable.
    ///
    /// A binary variable is only allowed to be 0 or 1
    /// so no further restrictions is usually needed (at this
    /// would lead to a constant).
    ///
    /// ```
    /// # use collomatique_ilp::{Variable, VariableType};
    /// let var_desc = Variable::binary();
    /// assert_eq!(var_desc.get_type(), VariableType::Binary);
    /// ```
    pub fn binary() -> Self {
        Variable {
            var_type: VariableType::Binary,
            min: None,
            max: None,
        }
    }

    /// Builds the description of a continuous (real) variable.
    ///
    /// The range of possible values can be restricted using
    /// [Variable::min] and [Variable::max].
    ///
    /// ```
    /// # use collomatique_ilp::{Variable, VariableType};
    /// let var_desc = Variable::continuous();
    /// assert_eq!(var_desc.get_type(), VariableType::Continuous);
    /// ```
    pub fn continuous() -> Self {
        Variable {
            var_type: VariableType::Continuous,
            min: None,
            max: None,
        }
    }

    /// Sets a minimum bound for a variable.
    ///
    /// ```
    /// # use collomatique_ilp::Variable;
    /// let var_desc = Variable::continuous().min(0.0);
    /// // var_desc describes a positive continuous variable.
    ///
    /// assert_eq!(var_desc.get_min(), Some(0.0));
    /// ```
    ///
    /// This function can be chained with [Variable::max] to set
    /// a range of possible values.
    /// ```
    /// # use collomatique_ilp::Variable;
    /// let var_desc = Variable::continuous().min(0.0).max(42.0);
    /// // var_desc describes a variable that takes its values in [0,42].
    ///
    /// assert_eq!(var_desc.get_min(), Some(0.0));
    /// assert_eq!(var_desc.get_max(), Some(42.0));
    /// ```
    pub fn min(mut self, m: f64) -> Self {
        self.min = Some(ordered_float::OrderedFloat(m));
        self
    }

    /// Sets a minimum bound for a variable.
    ///
    /// ```
    /// # use collomatique_ilp::Variable;
    /// let var_desc = Variable::continuous().max(0.0);
    /// // var_desc describes a negative continuous variable.
    ///
    /// assert_eq!(var_desc.get_max(), Some(0.0));
    /// ```
    ///
    /// This function can be chained with [Variable::min] to set
    /// a range of possible values.
    /// ```
    /// # use collomatique_ilp::Variable;
    /// let var_desc = Variable::continuous().max(0.0).min(-1.0);
    /// // var_desc describes a variable that takes its values in [-1,0].
    ///
    /// assert_eq!(var_desc.get_min(), Some(-1.0));
    /// assert_eq!(var_desc.get_max(), Some(0.0));
    /// ```
    pub fn max(mut self, m: f64) -> Self {
        self.max = Some(ordered_float::OrderedFloat(m));
        self
    }

    /// Returns the type of the variable
    ///
    /// ```
    /// # use collomatique_ilp::{Variable, VariableType};
    /// let continuous_var = Variable::continuous();
    /// let integer_var = Variable::integer();
    /// let binary_var = Variable::binary();
    ///
    /// assert_eq!(continuous_var.get_type(), VariableType::Continuous);
    /// assert_eq!(integer_var.get_type(), VariableType::Integer);
    /// assert_eq!(binary_var.get_type(), VariableType::Binary);
    /// ```
    pub fn get_type(&self) -> VariableType {
        self.var_type
    }

    /// Returns the minimum bound of the variable.
    ///
    /// ```
    /// # use collomatique_ilp::Variable;
    /// let desc_var = Variable::continuous().min(0.0);
    ///
    /// assert_eq!(desc_var.get_min(), Some(0.0));
    /// ```
    ///
    /// If no minimum bound was set, returns None.
    /// ```
    /// # use collomatique_ilp::Variable;
    /// let desc_var = Variable::continuous();
    ///
    /// assert_eq!(desc_var.get_min(), None);
    /// ```
    pub fn get_min(&self) -> Option<f64> {
        self.min.map(|x| x.into_inner())
    }

    /// Returns the maximum bound of the variable.
    ///
    /// ```
    /// # use collomatique_ilp::Variable;
    /// let desc_var = Variable::continuous().max(42.0);
    ///
    /// assert_eq!(desc_var.get_max(), Some(42.0));
    /// ```
    ///
    /// If no maximum bound was set, returns None.
    /// ```
    /// # use collomatique_ilp::Variable;
    /// let desc_var = Variable::continuous();
    ///
    /// assert_eq!(desc_var.get_max(), None);
    /// ```
    pub fn get_max(&self) -> Option<f64> {
        self.max.map(|x| x.into_inner())
    }
}

/// Sense for the objectiove function
///
/// This enum represents the sense in which
/// we try to optimize the objective function
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ObjectiveSense {
    /// Minimize the objective function (default)
    #[default]
    Minimize,
    /// Maximize the objective function
    Maximize,
}

/// Problem builder
///
/// This is the builder for [Problem].
/// To build a problem, start with [ProblemBuilder::new].
///
/// Then, add the various constraints with [ProblemBuilder::set_constraint]
/// or [ProblemBuilder::set_constraints].
///
/// Don't forget to declare the variables with [ProblemBuilder::set_variable] or [ProblemBuilder::set_variables].
///
/// Each variable used either by a constraint or by the objective function must be declared. This is necessary
/// as each variable type and range must be specified. This is also used as a consistency check.
///
/// You can optionally specify an objective function with [ProblemBuilder::set_objective_function].
///
/// Once the problem is fully specified, you can call [ProblemBuilder::build]. This will return a [Problem] struct
/// that you can use with a solver.
#[derive(Debug, Clone)]
pub struct ProblemBuilder<V: UsableData, C: UsableData> {
    constraints: BTreeMap<Constraint<V>, C>,
    variables: BTreeMap<V, Variable>,
    objective_func: LinExpr<V>,
    objective_sense: ObjectiveSense,
}

impl<V: UsableData, C: UsableData> Default for ProblemBuilder<V, C> {
    fn default() -> Self {
        ProblemBuilder {
            constraints: BTreeMap::default(),
            variables: BTreeMap::default(),
            objective_func: LinExpr::default(),
            objective_sense: ObjectiveSense::default(),
        }
    }
}

impl<V: UsableData, C: UsableData> ProblemBuilder<V, C> {
    /// Returns a new ProblemBuilder corresponding to an empty ILP problem.
    ///
    /// ```
    /// # use collomatique_ilp::ProblemBuilder;
    /// let problem_builder = ProblemBuilder::<String,String>::new();
    ///
    /// let problem = problem_builder.build().unwrap();
    /// assert!(problem.get_constraints().is_empty());
    /// assert!(problem.get_variables().is_empty());
    /// ```
    ///
    /// This is only a starting point. You can add variables by using [ProblemBuilder::set_variable]
    /// or [ProblemBuilder::set_variables]. You can similarly add constraints with [ProblemBuilder::set_constraint]
    /// or [ProblemBuilder::set_constraints].
    ///
    /// An objective function can also be set with [ProblemBuilder::set_objective_function].
    ///
    /// Finally, the problem is generated using [ProblemBuilder::build].
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_variable<T: Into<V>>(mut self, name: T, var: Variable) -> Self {
        self.variables.insert(name.into(), var);
        self
    }

    pub fn set_variables<U: Into<V>, T: IntoIterator<Item = (U, Variable)>>(
        mut self,
        vars: T,
    ) -> Self {
        for (name, var) in vars {
            self.variables.insert(name.into(), var);
        }
        self
    }

    pub fn set_constraint(mut self, constraint: Constraint<V>, desc: C) -> Self {
        self.constraints.insert(constraint, desc);
        self
    }

    pub fn set_constraints<T: IntoIterator<Item = (Constraint<V>, C)>>(
        mut self,
        constraints: T,
    ) -> Self {
        for (constraint, desc) in constraints {
            self.constraints.insert(constraint, desc);
        }
        self
    }

    pub fn set_objective_function(mut self, obj_fn: LinExpr<V>, obj_sense: ObjectiveSense) -> Self {
        self.objective_func = obj_fn;
        self.objective_sense = obj_sense;
        self
    }
}

/// Possible errors when building a [Problem] when calling [ProblemBuilder::build].
///
/// To build a [Problem], we use a [ProblemBuilder] and end with a call to [ProblemBuilder::build].
/// At this point a few sanity checks are done and some errors might appear.
/// This enum is the error type for these errors.
///
/// All the possible errors correspond to an undeclared variable.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BuildError<V: UsableData, C: UsableData> {
    /// A variable was not declared in a constraint.
    ///
    /// The parameters correspond, in order, to the undeclared variable,
    /// the constraint using it, and the description given of the constraint.
    #[error("Variable {0} is used in constraint {2} ({1}) but not explicitly declared")]
    UndeclaredVariableInConstraint(V, Constraint<V>, C),
    /// A variable was not declared in the objective function.
    ///
    /// The parameters correspond, in order, to the undeclared variable and the objective function.
    #[error("Variable {0} is used in objective function ({1}) but not explicitly declared")]
    UndeclaredVariableInObjFunc(V, LinExpr<V>),
}

pub type BuildResult<T, V, C> = std::result::Result<T, BuildError<V, C>>;

impl<V: UsableData, C: UsableData> ProblemBuilder<V, C> {
    pub fn build(self) -> BuildResult<Problem<V, C>, V, C> {
        // Check that all the variables are declared in constraints
        for (constraint, desc) in &self.constraints {
            if let Some(var) = self.check_variables_in_constraint(constraint) {
                return Err(BuildError::UndeclaredVariableInConstraint(
                    var,
                    constraint.clone(),
                    desc.clone(),
                ));
            }
        }

        // And now in the objective function
        if let Some(var) = self.check_variables_in_expr(&self.objective_func) {
            return Err(BuildError::UndeclaredVariableInObjFunc(
                var,
                self.objective_func.clone(),
            ));
        }

        Ok(Problem {
            constraints: self.constraints,
            variables: self.variables,
            objective_func: self.objective_func,
            objective_sense: self.objective_sense,
        })
    }

    /// Helper function to check if a constraint has undeclared variables.
    ///
    /// Returns None if no problem is detected, otherwise returns the undeclared variable.
    fn check_variables_in_constraint(&self, constraint: &Constraint<V>) -> Option<V> {
        self.check_variables_in_expr(constraint.get_lhs())
    }

    /// Helper function to check if an expression has undeclared variables.
    ///
    /// Returns None if no problem is detected, otherwise returns the undeclared variable.
    fn check_variables_in_expr(&self, expr: &LinExpr<V>) -> Option<V> {
        for var in expr.variables() {
            if !self.variables.contains_key(&var) {
                return Some(var);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Problem<V: UsableData, C: UsableData> {
    constraints: BTreeMap<Constraint<V>, C>,
    variables: BTreeMap<V, Variable>,
    objective_func: LinExpr<V>,
    objective_sense: ObjectiveSense,
}

impl<V: UsableData, C: UsableData> Problem<V, C> {
    pub fn get_constraints(&self) -> &BTreeMap<Constraint<V>, C> {
        &self.constraints
    }

    pub fn get_variables(&self) -> &BTreeMap<V, Variable> {
        &self.variables
    }

    pub fn get_objective_func(&self) -> &LinExpr<V> {
        &self.objective_func
    }

    pub fn get_objective_sense(&self) -> ObjectiveSense {
        self.objective_sense
    }
}
