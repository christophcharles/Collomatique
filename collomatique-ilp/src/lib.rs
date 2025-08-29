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
//! An Integer Linear Programming (ILP) problem adds the requirement that all (or only some of
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
/// Then, add the various constraints with [ProblemBuilder::add_constraint]
/// or [ProblemBuilder::add_constraints].
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
///
/// Here is an example usage defining a very simple schedule problem :
/// ---
/// We have two student groups X and Y.
/// They must both attend exactly once two different courses (1 and 2)
/// on the span of two weeks.
/// But the courses happen simultaneously.
///
/// This means we must fill a timetable of the following form:
/// |          | Week 1 | Week 2 |
/// |----------|--------|--------|
/// | Course 1 |        |        |
/// | Course 2 |        |        |
///
/// by putting an x or a y in each cell.
///
/// We represent this with 8 boolean variables.
/// The variable `xij` is 1 if X is written in the cell on the line i and column j, 0 otherwise.
/// The same pattern is used for `yij`.

/// We have three broad conditions :
/// - We should not put an X and a Y in the same cell. But a cell can possibly be empty.
///   This means that for all i and j, we have `xij + yij <= 1`.
/// - We should not put two Xs or two Ys in the same column (but column could have zero).
///   This means that for all j, we should have `x1j + x2j <= 1` and the same for the `yij`.
/// - We must put exactly one X and one Y on each line.
///   This means that `xi1 + xi2 = 1` for all i and the same for the group Y.
///
/// Additionally, we will prefer having group X in course 1 on week 1 by using an objective function.
/// We just use the expression `x11` which is indeed linear. If we maximize it, this should
/// value having gorup X in course 1 on week 1 rather than not.
///
/// ```
/// # use collomatique_ilp::{ProblemBuilder, LinExpr, Variable, ObjectiveSense};
/// let x11 = LinExpr::<String>::var("x11"); // Group X has course 1 on week 1
/// let x12 = LinExpr::<String>::var("x12"); // Group X has course 1 on week 2
/// let x21 = LinExpr::<String>::var("x21"); // Group X has course 2 on week 1
/// let x22 = LinExpr::<String>::var("x22"); // Group X has course 2 on week 2
///
/// let y11 = LinExpr::<String>::var("y11"); // Group Y has course 1 on week 1
/// let y12 = LinExpr::<String>::var("y12"); // Group Y has course 1 on week 2
/// let y21 = LinExpr::<String>::var("y21"); // Group Y has course 2 on week 1
/// let y22 = LinExpr::<String>::var("y22"); // Group Y has course 2 on week 2
///
/// let one = LinExpr::<String>::constant(1.0); // Constant for easier writing of constraints
///
/// let pb = ProblemBuilder::<String, String>::new()
///     .set_variables([
///         ("x11", Variable::binary()),
///         ("x12", Variable::binary()),
///         ("x21", Variable::binary()),
///         ("x22", Variable::binary())
///     ])
///     .set_variables([
///         ("y11", Variable::binary()),
///         ("y12", Variable::binary()),
///         ("y21", Variable::binary()),
///         ("y22", Variable::binary())
///     ])
///     // Both class should not attend a course at the same time
///     .add_constraints([
///         ((&x11 + &y11).leq(&one), "At most one group in course 1 on week 1"),
///         ((&x12 + &y12).leq(&one), "At most one group in course 1 on week 2"),
///         ((&x21 + &y21).leq(&one), "At most one group in course 2 on week 1"),
///         ((&x22 + &y22).leq(&one), "At most one group in course 2 on week 2")
///     ])
///     // Each class should not attend more than one course at a given time
///     .add_constraints([
///         ((&x11 + &x21).leq(&one), "At most one course for group X on week 1"),
///         ((&x12 + &x22).leq(&one), "At most one course for group X on week 2"),
///         ((&y11 + &y21).leq(&one), "At most one course for group Y on week 1"),
///         ((&y12 + &y22).leq(&one), "At most one course for group Y on week 2")
///     ])
///     // Each class must complete each course exactly once
///     .add_constraints([
///         ((&x11 + &x12).eq(&one), "Group X should have course 1 exactly once"),
///         ((&x21 + &x22).eq(&one), "Group X should have course 2 exactly once"),
///         ((&y11 + &y12).eq(&one), "Group Y should have course 1 exactly once"),
///         ((&y21 + &y22).eq(&one), "Group Y should have course 2 exactly once")
///     ])
///     // Objective function : prefer group X in course 1 on week 1
///     .set_objective_function(x11.clone(), ObjectiveSense::Maximize)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct ProblemBuilder<V: UsableData, C: UsableData> {
    constraints: Vec<(Constraint<V>, C)>,
    variables: BTreeMap<V, Variable>,
    objective_func: LinExpr<V>,
    objective_sense: ObjectiveSense,
}

impl<V: UsableData, C: UsableData> Default for ProblemBuilder<V, C> {
    fn default() -> Self {
        ProblemBuilder {
            constraints: Vec::default(),
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
    /// or [ProblemBuilder::set_variables]. You can similarly add constraints with [ProblemBuilder::add_constraint]
    /// or [ProblemBuilder::add_constraints].
    ///
    /// An objective function can also be set with [ProblemBuilder::set_objective_function].
    ///
    /// Finally, the problem is generated using [ProblemBuilder::build].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the definition of a variable and declares it at the same time.
    ///
    /// This is the primary function (along with [ProblemBuilder::set_variables])
    /// used to declare variables. It takes a name and a description of type [Variable].
    ///
    /// If the variable is already declared, its description is simply overwritten.
    ///
    /// ```
    /// # use collomatique_ilp::{ProblemBuilder, Variable, VariableType};
    /// let problem = ProblemBuilder::<String,String>::new()
    ///     .set_variable("A", Variable::binary())
    ///     .build()
    ///     .unwrap();
    ///
    /// let variables = problem.get_variables();
    /// assert_eq!(variables.len(), 1);
    /// assert_eq!(variables["A"].get_type(), VariableType::Binary);
    /// ```
    pub fn set_variable<T: Into<V>>(mut self, name: T, var: Variable) -> Self {
        self.variables.insert(name.into(), var);
        self
    }

    /// Sets the definition of multiple variables and declares them at the same time.
    ///
    /// This function is very similar to [ProblemBuilder::set_variable] but is designed
    /// to allow the declaration of multiple variables at once.
    ///
    /// If a variable is already declared, its description is simply overwritten.
    ///
    /// ```
    /// # use collomatique_ilp::{ProblemBuilder, Variable, VariableType};
    /// let problem = ProblemBuilder::<String,String>::new()
    ///     .set_variables([
    ///         ("A", Variable::binary()),
    ///         ("B", Variable::integer().min(0.0))
    ///     ])
    ///     .build()
    ///     .unwrap();
    ///
    /// let variables = problem.get_variables();
    /// assert_eq!(variables.len(), 2);
    /// assert_eq!(variables["A"].get_type(), VariableType::Binary);
    /// assert_eq!(variables["A"].get_min(), None);
    /// assert_eq!(variables["A"].get_max(), None);
    /// assert_eq!(variables["B"].get_type(), VariableType::Integer);
    /// assert_eq!(variables["B"].get_min(), Some(0.0));
    /// assert_eq!(variables["B"].get_max(), None);
    /// ```
    pub fn set_variables<U: Into<V>, T: IntoIterator<Item = (U, Variable)>>(
        mut self,
        vars: T,
    ) -> Self {
        for (name, var) in vars {
            self.variables.insert(name.into(), var);
        }
        self
    }

    /// Add a constraint to the constructed problem
    ///
    /// This is the primary function (along with [ProblemBuilder::add_constraints])
    /// used to add constraints. It takes a constraint (represented with [linexpr::Constraint])
    /// and a description of this constraint (with the generic type C).
    /// ```
    /// # use collomatique_ilp::{ProblemBuilder, Variable, VariableType, linexpr::LinExpr};
    /// let a = LinExpr::var("A");
    /// let b = LinExpr::var("B");
    ///
    /// let constraint = (a + b).leq(&LinExpr::constant(1.));
    ///
    /// let problem = ProblemBuilder::<String,String>::new()
    ///     .set_variable("A", Variable::binary())
    ///     .set_variable("B", Variable::binary())
    ///     .add_constraint(constraint, "A + B <= 1")
    ///     .build()
    ///     .expect("No undeclared variables");
    ///
    /// let constraints = problem.get_constraints();
    /// assert_eq!(constraints.len(), 1);
    ///
    /// let c = constraints[0].0.clone();
    /// // Displays "1*A + 1*B + (-1) <= 0"
    /// println!("{}", c);
    /// # assert_eq!(format!("{}", c), "1*A + 1*B + (-1) <= 0");
    /// ```
    pub fn add_constraint<T: Into<C>>(mut self, constraint: Constraint<V>, desc: T) -> Self {
        self.constraints.push((constraint, desc.into()));
        self
    }

    /// Add multiple constraints to the constructed problem.
    ///
    /// This function works mostly like [ProblemBuilder::add_constraint] and is
    /// used to add constraints. It takes an iterator over tuples containing constraints (represented with [linexpr::Constraint])
    /// and descriptions of these constraint (with the generic type C).
    ///
    /// ```
    /// # use collomatique_ilp::{ProblemBuilder, Variable, VariableType, linexpr::LinExpr};
    /// let a = LinExpr::var("A");
    /// let b = LinExpr::var("B");
    /// let c = LinExpr::var("C");
    ///
    /// let c1 = (&a + &b).leq(&LinExpr::constant(1.));
    /// let c2 = (&a + &c).leq(&LinExpr::constant(1.));
    ///
    /// let problem = ProblemBuilder::<String,String>::new()
    ///     .set_variable("A", Variable::binary())
    ///     .set_variable("B", Variable::binary())
    ///     .set_variable("C", Variable::binary())
    ///     .add_constraints([
    ///         (c1, "A + B <= 1"),
    ///         (c2, "A + C <= 1")
    ///     ])
    ///     .build()
    ///     .expect("No undeclared variables");
    ///
    /// let constraints = problem.get_constraints();
    /// assert_eq!(constraints.len(), 2);
    ///
    /// // Displays :
    /// // "1) 1*A + 1*B + (-1) <= 0 (A + B <= 1)"
    /// // "2) 1*A + 1*C + (-1) <= 0 (A + C <= 1)"
    /// for (i,c) in constraints.into_iter().enumerate() {
    ///     println!("{}) {} ({})", i+1, c.0, c.1);
    /// #   if i == 0 {
    /// #       assert_eq!(format!("{}) {} ({})", i+1, c.0, c.1), "1) 1*A + 1*B + (-1) <= 0 (A + B <= 1)");
    /// #   } else {
    /// #       assert_eq!(format!("{}) {} ({})", i+1, c.0, c.1), "2) 1*A + 1*C + (-1) <= 0 (A + C <= 1)");
    /// #   }
    /// }
    /// ```
    pub fn add_constraints<U: Into<C>, T: IntoIterator<Item = (Constraint<V>, U)>>(
        mut self,
        constraints: T,
    ) -> Self {
        for (constraint, desc) in constraints {
            self.constraints.push((constraint, desc.into()));
        }
        self
    }

    /// Set the objective function for the ILP problem
    ///
    /// This function can be used to set an objective function.
    /// An objective function is just a linear expression that must be minimized or maximized.
    ///
    /// As a design choice, the sense must always be specified with the objective function.
    ///
    /// Be careful, all variables must be declared.
    ///
    /// ```
    /// # use collomatique_ilp::{ProblemBuilder, Variable, VariableType, linexpr::LinExpr, ObjectiveSense};
    /// let a = LinExpr::<String>::var("A");
    /// let b = LinExpr::<String>::var("B");
    ///
    /// let obj_func = a + b;
    ///
    /// let problem = ProblemBuilder::<String,String>::new()
    ///     .set_variable("A", Variable::binary())
    ///     .set_variable("B", Variable::binary())
    ///     .set_objective_function(obj_func.clone(), ObjectiveSense::Maximize)
    ///     .build()
    ///     .expect("No undeclared variables");
    ///
    /// assert_eq!(*problem.get_objective_function(), obj_func);
    /// assert_eq!(problem.get_objective_sense(), ObjectiveSense::Maximize);
    /// ```
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
    /// Builds the underlying problem.
    ///
    /// Once you have constructed the problem using [ProblemBuilder::add_constraint],
    /// [ProblemBuilder::add_constraints], declared the variables using [ProblemBuilder::set_variable]
    /// or [ProblemBuilder::set_variables] and optionally defined an objective function with
    /// [ProblemBuilder::set_objective_function], you can commit the result into a [Problem].
    ///
    /// This function does just that. It does a simple sanity check: all variables that appear in
    /// the constraints and the objective function must be declared.
    ///
    /// If some variable is not declared, it returns an error.
    /// ```should_panic
    /// # use collomatique_ilp::{ProblemBuilder, Variable, VariableType, linexpr::LinExpr, ObjectiveSense};
    /// let a = LinExpr::<String>::var("A");
    /// let b = LinExpr::<String>::var("B");
    ///
    /// let obj_func = a.clone();
    /// let c = (&a + &b).leq(&LinExpr::constant(1.));
    ///
    /// let problem = ProblemBuilder::<String,String>::new()
    ///     .add_constraint(c, "A + B <= 1")
    ///     .set_objective_function(obj_func.clone(), ObjectiveSense::Maximize)
    ///     .build()
    ///     .unwrap(); // Panics!
    /// ```
    ///
    /// Otherwise, it returns the constructed problem.
    /// ```
    /// # use collomatique_ilp::{ProblemBuilder, Variable, VariableType, linexpr::LinExpr, ObjectiveSense};
    /// let a = LinExpr::<String>::var("A");
    /// let b = LinExpr::<String>::var("B");
    ///
    /// let obj_func = a.clone();
    /// let c = (&a + &b).leq(&LinExpr::constant(1.));
    ///
    /// let problem = ProblemBuilder::<String,String>::new()
    ///     .set_variable("A", Variable::binary())
    ///     .set_variable("B", Variable::binary())
    ///     .add_constraint(c, "A + B <= 1")
    ///     .set_objective_function(obj_func.clone(), ObjectiveSense::Maximize)
    ///     .build()
    ///     .expect("No undeclared variables");
    /// ```
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

/// ILP problem
///
/// This data structure represents an ILP problem.
/// You cannot build it directly. It is built through the builder
/// pattern, using [ProblemBuilder].
#[derive(Debug, Clone)]
pub struct Problem<V: UsableData, C: UsableData> {
    constraints: Vec<(Constraint<V>, C)>,
    variables: BTreeMap<V, Variable>,
    objective_func: LinExpr<V>,
    objective_sense: ObjectiveSense,
}

impl<V: UsableData, C: UsableData> Problem<V, C> {
    /// Transforms the problem back into a [ProblemBuilder].
    ///
    /// This is useful when you have a problem that works that
    /// you want to change a bit (maybe add a constraint or a variable).
    pub fn into_builder(self) -> ProblemBuilder<V, C> {
        ProblemBuilder {
            constraints: self.constraints,
            variables: self.variables,
            objective_func: self.objective_func,
            objective_sense: self.objective_sense,
        }
    }

    /// Returns the constraints of the problem.
    ///
    /// The constraints are returned as a list of tuples.
    /// The first element of the tuple is the algebraic constraint (described by a [linexpr::Constraint]).
    /// The second element is a description of the constraint (given at
    /// building time).
    pub fn get_constraints(&self) -> &[(Constraint<V>, C)] {
        &self.constraints[..]
    }

    /// Returns the list of variables.
    ///
    /// The result is a map associating to each variable name
    /// a description of type [Variable].
    pub fn get_variables(&self) -> &BTreeMap<V, Variable> {
        &self.variables
    }

    /// Returns the objective function of the problem.
    ///
    /// The objective function is a simple linear expression described a struct of type [linexpr::LinExpr].
    pub fn get_objective_function(&self) -> &LinExpr<V> {
        &self.objective_func
    }

    /// Returns the sense of the obejctive function (is it maximized or minimized).
    ///
    /// See [ObjectiveSense].
    pub fn get_objective_sense(&self) -> ObjectiveSense {
        self.objective_sense
    }
}
