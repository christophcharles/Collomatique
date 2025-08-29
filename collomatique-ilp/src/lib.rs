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
pub mod mat_repr;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub use linexpr::{Constraint, LinExpr};

use mat_repr::{ConfigRepr, ProblemRepr};

/// Tolerance for 0 checking on f64
/// 
/// Computation with f64 can lead, because
/// of rounding errors, to small values that really should be
/// zero.
/// 
/// In Collomatique, most values are integers.
/// So, using a small threshold can solve this kind of problems
/// 
/// Of course, there is also a bad consequence:
/// a configuration might be detected as valid when it isn't.
pub const TOLERANCE : f64 = 0.00001;

/// Tests for non-negativity with [TOLERANCE]].
/// 
/// Tests if an f64 is non-negative (positive but non-zero)
/// but does so taking into account the [TOLERANCE].
pub fn f64_is_non_negative(v: f64) -> bool {
    v > TOLERANCE
}

/// Tests for zero with [TOLERANCE]].
/// 
/// Tests if an f64 is zero
/// but does so taking into account the [TOLERANCE].
pub fn f64_is_zero(v: f64) -> bool {
    v.abs() <= TOLERANCE
}

/// Tests for equality with [TOLERANCE]].
/// 
/// Tests if two f64 are equals
/// but does so taking into account the [TOLERANCE].
pub fn f64_equals(v1: f64, v2: f64) -> bool {
    f64_is_zero(v1 - v2)
}

/// Default matrix representation for [Problem].
///
/// In most cases, the default representation is just fine
/// and there is no reason to change it.
///
/// By default, we use a sparse matrix representation (based on [sprs] as a backend).
/// This is well-suited to typical scheduling problems, and in particular to
/// the problems we try to solve in Collomatique.
/// See [mat_repr] for more information.
pub type DefaultRepr<V> = mat_repr::sparse::SprsProblem<V>;

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

impl std::fmt::Display for VariableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableType::Binary => write!(f, "binary"),
            VariableType::Integer => write!(f, "int"),
            VariableType::Continuous => write!(f, "real"),
        }
    }
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

impl std::fmt::Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.min, self.max) {
            (Some(mi), Some(ma)) => {
                write!(f, "{} [{},{}]", self.var_type, mi, ma)?;
            }
            (None, Some(ma)) => {
                write!(f, "{} ]-oo,{}]", self.var_type, ma)?;
            }
            (Some(mi), None) => {
                write!(f, "{} [{},+oo[", self.var_type, mi)?;
            }
            (None, None) => {
                write!(f, "{} ]-oo,+oo[", self.var_type)?;
            }
        }
        Ok(())
    }
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

impl std::fmt::Display for ObjectiveSense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectiveSense::Minimize => write!(f, "Minimize"),
            ObjectiveSense::Maximize => write!(f, "Maximize"),
        }
    }
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
pub struct ProblemBuilder<V: UsableData, C: UsableData, P: ProblemRepr<V> = DefaultRepr<V>> {
    constraints: Vec<(Constraint<V>, C)>,
    variables: BTreeMap<V, Variable>,
    objective_func: LinExpr<V>,
    objective_sense: ObjectiveSense,
    _phantom_data: std::marker::PhantomData<P>,
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Default for ProblemBuilder<V, C, P> {
    fn default() -> Self {
        ProblemBuilder {
            constraints: Vec::default(),
            variables: BTreeMap::default(),
            objective_func: LinExpr::default(),
            objective_sense: ObjectiveSense::default(),
            _phantom_data: std::marker::PhantomData,
        }
    }
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> ProblemBuilder<V, C, P> {
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

    /// Converts to a different matrix representation
    ///
    /// To simplify default traits, the builder is associated to a matrix representation.
    /// This functions allows converting the builder to another representation.
    pub fn convert_repr<P2: ProblemRepr<V>>(self) -> ProblemBuilder<V, C, P2> {
        ProblemBuilder {
            constraints: self.constraints,
            variables: self.variables,
            objective_func: self.objective_func,
            objective_sense: self.objective_sense,
            _phantom_data: std::marker::PhantomData,
        }
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

/// Result type associated with [BuildError].
/// 
/// It is returned by [ProblemBuilder::build].
pub type BuildResult<T, V, C> = std::result::Result<T, BuildError<V, C>>;

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> ProblemBuilder<V, C, P> {
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
    pub fn build(self) -> BuildResult<Problem<V, C, P>, V, C> {
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

        let repr = P::new(&self.variables, self.constraints.iter().map(|(x, _)| x));

        Ok(Problem {
            constraints: self.constraints,
            variables: self.variables,
            objective_func: self.objective_func,
            objective_sense: self.objective_sense,
            repr,
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Problem<V: UsableData, C: UsableData, P: ProblemRepr<V> = DefaultRepr<V>> {
    constraints: Vec<(Constraint<V>, C)>,
    variables: BTreeMap<V, Variable>,
    objective_func: LinExpr<V>,
    objective_sense: ObjectiveSense,
    repr: P,
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> std::fmt::Display for Problem<V, C, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "variables:\n")?;
        for (i, (v, desc)) in self.variables.iter().enumerate() {
            write!(f, "{}) {}: {}", i, v, desc)?;
        }

        write!(f, "constraints:\n")?;
        for (i, (c, desc)) in self.constraints.iter().enumerate() {
            write!(f, "{}) {} ({})\n", i, c, desc)?;
        }

        write!(f, "objective function: {}", self.objective_func)?;
        write!(f, "objective sense: {}", self.objective_sense)?;

        Ok(())
    }
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Problem<V, C, P> {
    /// Transforms the problem back into a [ProblemBuilder].
    ///
    /// This is useful when you have a problem that works that
    /// you want to change a bit (maybe add a constraint or a variable).
    pub fn into_builder(self) -> ProblemBuilder<V, C, P> {
        ProblemBuilder {
            constraints: self.constraints,
            variables: self.variables,
            objective_func: self.objective_func,
            objective_sense: self.objective_sense,
            _phantom_data: std::marker::PhantomData,
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

/// Report on confirmity between configuration data and an ILP problem
///
/// The structure [ConfigData] can represent the data for a configuration.
/// But it might not correspond to a given [Problem] for 3 major reasons:
/// - some variables (for the problem) might not have an associated value in the configuration
/// - some variables in the configuration are not part of the problem
/// - some variables, though part of the problem, do not conform to their type.
/// This report stores these problematic variables either as a result from a call
/// to [Problem::check_config_data_variables] or as an error when calling
/// [Problem::build_config].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigDataVarCheck<V: UsableData> {
    /// Set of missing variables in the configuration data (with respect to the given problem)
    pub missing_variables: BTreeSet<V>,
    /// Set of excess variables in the configuration data (variables not present in the given problem)
    pub excess_variables: BTreeSet<V>,
    /// Set of non-conforming variables in the configuration data
    /// (variables with values not conforming to their type with respect to the given problem)
    pub non_conforming_variables: BTreeSet<V>,
}

impl<V: UsableData> ConfigDataVarCheck<V> {
    /// Returns true if the report is empty, that is if there is no issue between
    /// the configuration data and the given problem.
    pub fn is_empty(&self) -> bool {
        self.missing_variables.is_empty()
            && self.excess_variables.is_empty()
            && self.non_conforming_variables.is_empty()
    }
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>> Problem<V, C, P> {
    /// Checks if there are mismatches between the variables in a configuration data
    /// (represented by a [ConfigData]) and the variables in the problem.
    ///
    /// The structure [ConfigData] can represent the data for a configuration.
    /// But it might not correspond to a given [Problem] for 2 major reasons:
    /// - some variables (for the problem) might not have an associated value in the configuration
    /// - some variables in the configuration are not part of the problem
    /// This functions checks for this and returns a report (possibly empty - see [ConfigDataVarCheck::is_empty])
    /// in a structure of type [ConfigDataVarCheck].
    pub fn check_config_data_variables(
        &self,
        config_data: &ConfigData<V>,
    ) -> ConfigDataVarCheck<V> {
        let config_vars: BTreeSet<V> = config_data.values.keys().cloned().collect();
        let problem_vars: BTreeSet<V> = self.variables.keys().cloned().collect();

        let vars_in_common = config_vars.intersection(&problem_vars);

        ConfigDataVarCheck {
            missing_variables: problem_vars.difference(&config_vars).cloned().collect(),
            excess_variables: config_vars.difference(&problem_vars).cloned().collect(),
            non_conforming_variables: vars_in_common
                .filter(|&x| !self.check_variable_conformity(config_data, x))
                .cloned()
                .collect(),
        }
    }

    /// Internal helper function: checks that a variable of a [ConfigData] is conforming
    /// with respect to the current [Problem].
    ///
    /// A variable is conforming if it is indeed part of the problem
    /// and if its value conforms to its type (as given by the problem).
    fn check_variable_conformity(&self, config_data: &ConfigData<V>, name: &V) -> bool {
        let Some(value) = config_data.values.get(name).map(|x| x.into_inner()) else {
            return false;
        };

        self.check_value_conformity(name, value)
    }

    /// Internal helper function: checks that a value for a variable is conforming
    /// with respect to the current [Problem].
    ///
    /// A variable is conforming if it is indeed part of the problem
    /// and if its value conforms to its type (as given by the problem).
    fn check_value_conformity(&self, name: &V, value: f64) -> bool {
        let Some(var_constraint) = self.variables.get(name) else {
            return false;
        };

        match var_constraint.get_type() {
            VariableType::Continuous => true,
            VariableType::Integer => value == value.floor(),
            VariableType::Binary => f64_equals(value, 0.) || f64_equals(value, 1.0),
        }
    }

    /// Builds a [Config] for the problem from a [ConfigData] without checking first
    /// if the variables match.
    ///
    /// This is obviously unsafe. Unless you are sure that the [ConfigData] does indeed
    /// have the right variables, you should first check with [Problem::check_config_data_variables]
    /// or rather call [Problem::build_config] which will have a sanity check first.
    pub unsafe fn build_config_unchecked(&self, config_data: ConfigData<V>) -> Config<'_, V, C, P> {
        let repr = self.repr.config_from(&config_data.values);

        Config {
            problem: self,
            values: config_data.values,
            repr,
        }
    }

    /// Builds a [Config] for the problem from a [ConfigData].
    ///
    /// This functions does the necessary sanity checks first. If some variables are
    /// missing (or in excess) in the configuration data, the function will fail and
    /// return a report in a [ConfigDataVarCheck].
    ///
    /// Otherwise, this will return a [Config] suited for the current problem.
    pub fn build_config(
        &self,
        config_data: ConfigData<V>,
    ) -> Result<Config<'_, V, C, P>, ConfigDataVarCheck<V>> {
        let report = self.check_config_data_variables(&config_data);

        if !report.is_empty() {
            return Err(report);
        }

        Ok(unsafe { self.build_config_unchecked(config_data) })
    }
}

/// ILP configuration data
///
/// This data structure is an intermediary structure
/// representing an association bewteen variables
/// and their values.
///
/// The main difference with [Config] is that [ConfigData]
/// exists on its own and does not need a corresponding [Problem].
///
/// This means two things:
/// - first, it can be built easily incrementaly. You can
///   modify the values of the variables with its various methods.
/// - second, it is not, in a absolute sense, feasable or not feasable.
///   A configuration is feasable if it satisfies all the hard
///   constraints of a problem. This of course depends on the problem
///   and assumes *some* compatibility between the problem and the configuration.
/// These two points imply that [ConfigData] can act as a builder type for [Config].
/// You build a configuration from scratch and once all the variables are correctly set,
/// you can convert it to a [Config] for a specific [Problem] using [Problem::build_config].

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigData<V: UsableData> {
    values: BTreeMap<V, ordered_float::OrderedFloat<f64>>,
}

impl<V: UsableData> Default for ConfigData<V> {
    fn default() -> Self {
        ConfigData {
            values: BTreeMap::default(),
        }
    }
}

impl<V: UsableData, U: Into<V>, W: Into<f64>, T: IntoIterator<Item = (U, W)>> From<T>
    for ConfigData<V>
{
    fn from(value: T) -> Self {
        ConfigData {
            values: BTreeMap::from_iter(
                value
                    .into_iter()
                    .map(|(x, y)| (x.into(), ordered_float::OrderedFloat(y.into()))),
            ),
        }
    }
}

impl<V: UsableData> ConfigData<V> {
    /// Creates an empty [ConfigData] with (initially) no variables at all
    pub fn new() -> Self {
        ConfigData::default()
    }

    /// Sets a variable in the configuration to a specific value.
    ///
    /// If the variable does not exist in the configuration yet, it is
    /// simply added.
    /// If the variable already exists, it is overwritten.
    ///
    /// A variable can be removed using [ConfigData::remove].
    pub fn set<U: Into<V>, W: Into<f64>>(mut self, name: U, value: W) -> Self {
        self.values
            .insert(name.into(), ordered_float::OrderedFloat(value.into()));
        self
    }

    /// Sets multiple variables in the configuration to specific values.
    ///
    /// This works like [ConfigData::set] but allows the setting of multiple
    /// variables at a time.
    ///
    /// It takes as a parameter an iterator over tuples, containing variable names
    /// and their associated values.
    ///
    /// If a variable does not exist in the configuration yet, it is
    /// simply added.
    /// If a variable already exists, it is overwritten.
    ///
    /// If a variable appears multiple time in the iterator, each new value overwrites
    /// the previous one.
    pub fn set_iter<U: Into<V>, W: Into<f64>, T: IntoIterator<Item = (U, W)>>(
        mut self,
        values: T,
    ) -> Self {
        for (name, value) in values {
            self.values
                .insert(name.into(), ordered_float::OrderedFloat(value.into()));
        }
        self
    }

    /// Removes a variable from the configuration.
    ///
    /// If the variable is not in the configuration yet, this is
    /// simply a no-op.
    ///
    /// You can add back a variable with [ConfigData::set].
    pub fn remove<U: Into<V>>(mut self, name: U) -> Self {
        self.values.remove(&name.into());
        self
    }

    /// Removes multiple variables from the configuration.
    ///
    /// This works like [ConfigData::remove] but is designed to remove
    /// multiple variables at a time.
    ///
    /// If a variable is not in the configuration yet, it is simply ignored.
    ///
    /// If a variable appears multiple times in the iterator, this removes it
    /// only once.
    pub fn remove_iter<U: Into<V>, T: IntoIterator<Item = U>>(mut self, names: T) -> Self {
        for name in names {
            self.values.remove(&name.into());
        }
        self
    }

    /// Keeps variables based on a predicate.
    ///
    /// This function works similarly to [ConfigData::remove] and
    /// [ConfigData::remove_iter]. Its net effect is to remove variables
    /// from the configuration.
    ///
    /// It takes a closure as a parameter. It is called on each variable
    /// with its name and its value. If it returns `true`, the variable is kept.
    /// If is returns `false`, the variable is removed.
    ///
    /// This allows to remove variables based on their values rather than just their name.
    pub fn retain<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(&V, f64) -> bool,
    {
        self.values.retain(|k, v| f(k, v.into_inner()));
        self
    }

    /// Returns the variables in the configuration
    ///
    /// This returns an iterator on the variables in the configuration.
    /// Only the names are given.
    ///
    /// If you also want the values, you should use [ConfigData::get_values].
    pub fn get_variables(&self) -> impl Iterator<Item = &V> {
        self.values.keys()
    }

    /// Returns the variables and their values in the configuration
    ///
    /// This returns a map associating each name to a value.
    ///
    /// If you only want the variable names, you should use [ConfigData::get_variables].
    pub fn get_values(&self) -> BTreeMap<V, f64> {
        self.values
            .iter()
            .map(|(x, y)| (x.clone(), y.into_inner()))
            .collect()
    }

    /// Returns the value of a single variable in the configuration
    ///
    /// This function returns the value of the variable `name`. If there
    /// is no such variable in the configuration, this returns `None`.
    ///
    /// If you want all the values of all the variables, you should use [ConfigData::get_values].
    /// If you want the list of possible variable names, you should use [ConfigData::get_variables].
    pub fn get<U: Into<V>>(&self, name: U) -> Option<f64> {
        self.values.get(&name.into()).map(|x| x.into_inner())
    }
}

/// A configuration for a [Problem].
///
/// A configuration is the affectation of a value to every variable of a
/// problem. As such, a [Config] is specific to a [Problem].
///
/// Such a configuration does not need to be *feasable* (meaning that
/// it does not have to satisfy the various inequalities and so it does
/// not have to be a solution of the problem). But it does need
/// to be a valid configuration, which means that all the variables
/// and only the variables of the problem have a value attributed to them
/// and these values conform to their prescribed type.
///
/// We do not require though that the variable ranges are satisfied. This
/// is considered to be a inequality on the variable and as such a constraint
/// that does not need to be satisfied.
///
/// [Config] represents such a valid configuration. It is usually built
/// from a [ConfigData] that has been checked and transformed using [Problem::build_config].
///
/// A [Config], because it is linked to a problem, keeps a reference to its
/// problem which explains the needed lifetime `'a`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Config<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    problem: &'a Problem<V, C, P>,
    values: BTreeMap<V, ordered_float::OrderedFloat<f64>>,
    repr: P::Config<'a>,
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> Config<'a, V, C, P> {
    /// Returns the [Problem] this [Config] is associated to.
    pub fn get_problem(&self) -> &Problem<V, C, P> {
        self.problem
    }

    /// Returns the variables in the configuration
    ///
    /// This returns an iterator on the variables in the configuration.
    /// Only the names are given.
    ///
    /// If you also want the values, you should use [Config::get_values].
    pub fn get_variables(&self) -> impl Iterator<Item = &V> {
        self.values.keys()
    }

    /// Returns the variables and their values in the configuration
    ///
    /// This returns a map associating each name to a value.
    ///
    /// If you only want the variable names, you should use [Config::get_variables].
    pub fn get_values(&self) -> BTreeMap<V, f64> {
        self.values
            .iter()
            .map(|(x, y)| (x.clone(), y.into_inner()))
            .collect()
    }

    /// Returns the value of the variable `name`.
    ///
    /// If the variable is not part of the [Problem] (and thus
    /// not part of [Config]), this function returns `None`.
    pub fn get<T: Into<V>>(&self, name: T) -> Option<f64> {
        self.values.get(&name.into()).map(|x| x.into_inner())
    }

    /// Returns true if the configuration is feasable
    pub fn is_feasable(&self) -> bool {
        for (var, value) in &self.values {
            let desc = &self.problem.variables[var];
            let v = value.into_inner();

            if let Some(m) = desc.get_min() {
                if v < m {
                    return false;
                }
            }
            if let Some(m) = desc.get_max() {
                if v > m {
                    return false;
                }
            }
        }

        self.repr.is_feasable()
    }

    /// Blames unsatisfied constraints
    ///
    /// This returns an iterator of the unsatisfied constraints.
    /// The iterator iterates over tuples containing
    /// the actual problematic constraints and the corresponding descriptions
    /// that was provided at problem construction.
    ///
    /// It can be empty if all the constraints are satisfied.
    pub fn blame(&self) -> impl ExactSizeIterator<Item = &(Constraint<V>, C)> {
        let unsatisfied_constraints = self.repr.unsatisfied_constraints();

        unsatisfied_constraints
            .into_iter()
            .map(|i| &self.problem.constraints[i])
    }

    /// Turns a configuration into a feasable configuration
    ///
    /// If the configuration is feasable, it is turned into a [FeasableConfig].
    /// Otherwise, this returns `None`.
    pub fn into_feasable(self) -> Option<FeasableConfig<'a, V, C, P>> {
        if !self.is_feasable() {
            return None;
        }

        Some(unsafe { self.into_feasable_unchecked() })
    }

    /// Turns a configuration into a feasable configuration
    ///
    /// This is the unchecked (and therefore unsafe) version of [Config::into_feasable].
    pub unsafe fn into_feasable_unchecked(self) -> FeasableConfig<'a, V, C, P> {
        FeasableConfig(self)
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> std::fmt::Display
    for Config<'a, V, C, P>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, (v, desc)) in self.values.iter().enumerate() {
            write!(f, "{}) {}: {}", i, v, desc)?;
        }

        Ok(())
    }
}

/// A feasable configuration
///
/// A feasable configuration is a configuration that satisfies all
/// the *hard* constraints (all the inequalities and equalities).
///
/// This type represents a configuration that is known to be feasable.
/// It is constructed by [Config::into_feasable].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeasableConfig<'a, V: UsableData, C: UsableData, P: ProblemRepr<V> = DefaultRepr<V>>(
    Config<'a, V, C, P>,
);

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> FeasableConfig<'a, V, C, P> {
    /// Turns a [FeasableConfig] back into a [Config].
    pub fn into_inner(self) -> Config<'a, V, C, P> {
        self.0
    }

    /// Gives a reference to the inner [Config].
    ///
    /// This is normally not needed as [FeasableConfig]
    /// implements [std::ops::Deref].
    pub fn inner(&self) -> &Config<'a, V, C, P> {
        &self.0
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> std::ops::Deref
    for FeasableConfig<'a, V, C, P>
{
    type Target = Config<'a, V, C, P>;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> std::fmt::Display
    for FeasableConfig<'a, V, C, P>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner().fmt(f)
    }
}
