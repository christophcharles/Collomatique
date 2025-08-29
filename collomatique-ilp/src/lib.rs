pub mod linexpr;

use std::collections::BTreeMap;

pub use linexpr::{LinExpr, Constraint};

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

/// Complete description of a variable possible range of values.
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

#[derive(Debug, Clone)]
pub struct ProblemBuilder<V: UsableData, C: UsableData> {
    constraints: BTreeMap<Constraint<V>, C>,
    variables: BTreeMap<V, Variable>,
    objective_func: LinExpr<V>,
}

impl<V: UsableData, C: UsableData> Default for ProblemBuilder<V, C> {
    fn default() -> Self {
        ProblemBuilder {
            constraints: BTreeMap::new(),
            variables: BTreeMap::new(),
            objective_func: LinExpr::constant(0.0),
        }
    }
}

impl<V: UsableData, C: UsableData> ProblemBuilder<V, C> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_variable<T: Into<V>>(mut self, name: T, var: Variable) -> Self {
        self.variables.insert(name.into(), var);
        self
    }

    pub fn set_variables<U: Into<V>, T: IntoIterator<Item = (U, Variable)>>(mut self, vars: T) -> Self {
        for (name, var) in vars {
            self.variables.insert(name.into(), var);
        }
        self
    }

    pub fn set_constraint(mut self, constraint: Constraint<V>, desc: C) -> Self {
        self.constraints.insert(constraint, desc);
        self
    }

    pub fn set_constraints<T: IntoIterator<Item = (Constraint<V>, C)>>(mut self, constraints: T) -> Self {
        for (constraint, desc) in constraints {
            self.constraints.insert(constraint, desc);
        }
        self
    }

    pub fn set_objective_func(mut self, obj_fn: LinExpr<V>) -> Self {
        self.objective_func = obj_fn;
        self
    }
}