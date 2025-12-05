//! This module defines [LinExpr] and [Constraint].
//! These structs are used to represent linear expressions and constraints for
//! integer linear optimization problems within collomatique.

#[cfg(test)]
mod tests;

use crate::f64_is_positive;

use super::{f64_is_zero, UsableData};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet},
};

/// [LinExpr] represents a linear expression (of the form 2*a + 3*b - 4*c + 2).
///
/// The coefficients are f64 and natural operations are overloaded to naturally represent
/// operations on linear expressions.
///
/// There are mainly two ways to build an Expr.
/// You can use [LinExpr::var]. This builds an expression containing only one variable with coefficient one.
/// ```
/// # use collomatique_ilp::linexpr::LinExpr;
/// # use std::collections::BTreeSet;
/// // expr represents the linear expression : "1*A"
/// let expr = LinExpr::<String>::var("A");
///
/// assert_eq!(expr.variables(), BTreeSet::from([String::from("A")])); // There is only "A"
/// assert_eq!(expr.get("A"), Some(1.0)); // The coefficient for "A" is 1
/// assert_eq!(expr.get_constant(), 0.0); // The constant is 0.0 (there is no constant)
/// ```
///
/// You can use [LinExpr::constant]. This builds a constant expression containing no variables.
/// ```
/// # use collomatique_ilp::linexpr::LinExpr;
/// # use std::collections::BTreeSet;
/// // expr represents the constant linear expression equals to 42
/// let expr = LinExpr::<String>::constant(42.0);
///
/// assert_eq!(expr.variables(), BTreeSet::new()); // There are no variables
/// assert_eq!(expr.get_constant(), 42.0); // The constant is 42.0
/// ```
///
/// More complex expressions are then built using overloaded operations
/// ```
/// # use collomatique_ilp::linexpr::LinExpr;
/// # use std::collections::BTreeSet;
/// let expr1 = LinExpr::<String>::var("A");
/// let expr2 = LinExpr::<String>::var("B");
/// let expr3 = LinExpr::<String>::constant(42.0);
///
/// // expr represents the linear expr 2*A - 3*B - 42
/// let expr = 2.0*expr1 - 3 *expr2 - expr3;
/// // Note you can use i32 or f64 in your operations
///
/// // There are 2 variables : A and B
/// assert_eq!(expr.variables(), BTreeSet::from([String::from("A"), String::from("B")]));
/// assert_eq!(expr.get("A"), Some(2.0)); // The coefficient for "A" is 2
/// assert_eq!(expr.get("B"), Some(-3.0)); // The coefficient for "B" is -3
/// assert_eq!(expr.get_constant(), -42.0); // The constant is -42.0
/// ```
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct LinExpr<V: UsableData> {
    coefs: BTreeMap<V, ordered_float::OrderedFloat<f64>>,
    constant: ordered_float::OrderedFloat<f64>,
}

impl<V: UsableData> Default for LinExpr<V> {
    fn default() -> Self {
        LinExpr {
            coefs: BTreeMap::default(),
            constant: ordered_float::OrderedFloat::default(),
        }
    }
}

/// [EqSymbol] represents an equality or inequality symbol.
///
/// It can only represents an equality or a "less-than" inequality.
/// "more-than" inequalities can always be represented by changing all the signs.
///
/// It is done so to simplify comparison between constraints.
///
/// Normally, you don't have to handle EqSymbol directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum EqSymbol {
    /// Represents an "equal" ("=") symbol
    Equals,
    /// Represents a "less-than" ("<=") symbol
    #[default]
    LessThan,
}

/// [Constraint] represents a linear constraint
///
/// A linear constraint is a constraint linking different variables
/// with coefficients for each one of them and possibly a constant.
///
/// Here is an example : 2*a +3*c <= 4*b - 42
///
/// The precise position of every term is not recorded in [Constraint].
/// Internally, everything is sent to the left hand side and always compared to zero.
///
/// [Constraint] is usually built using [LinExpr::leq], [LinExpr::eq] or [LinExpr::geq].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Constraint<V: UsableData> {
    symbol: EqSymbol,
    expr: LinExpr<V>,
}

impl<V: UsableData> LinExpr<V> {
    /// Expr::var builds a simple linear expression with a single
    /// variable with coefficient 1 and no constant term.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// // expr represents the linear expression : "1*A"
    /// let expr = LinExpr::<String>::var("A");
    ///
    /// assert_eq!(expr.variables(), BTreeSet::from([String::from("A")])); // There is only "A"
    /// assert_eq!(expr.get("A"), Some(1.0)); // The coefficient for "A" is 1
    /// assert_eq!(expr.get_constant(), 0.0); // The constant is 0.0 (there is no constant)
    /// ```
    pub fn var<T: Into<V>>(name: T) -> Self {
        LinExpr {
            coefs: BTreeMap::from([(name.into(), ordered_float::OrderedFloat(1.0))]),
            constant: ordered_float::OrderedFloat(0.0),
        }
    }

    /// Expr::var builds a simple linear expression with no variables and only the constant term.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// // expr represents the constant linear expression equals to 42
    /// let expr = LinExpr::<String>::constant(42.0);
    ///
    /// assert_eq!(expr.variables(), BTreeSet::new()); // There are no variables
    /// assert_eq!(expr.get_constant(), 42.0); // The constant is 42.0
    /// ```
    pub fn constant(number: f64) -> Self {
        LinExpr {
            coefs: BTreeMap::new(),
            constant: ordered_float::OrderedFloat(number),
        }
    }
}

impl<V: UsableData> LinExpr<V> {
    /// Returns the constant term in the expression.
    ///
    /// For instance for the expression 2*a+3*b - 4*c + 42, this would return 42.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr = LinExpr::<String>::constant(4.0);
    /// let constant = expr.get_constant(); // should be 4
    ///
    /// assert_eq!(constant, 4.0);
    /// ```
    ///
    /// There is always a constant term. If empty, it is actually zero.
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr = LinExpr::<String>::var("a");
    /// // no constant really is constant = 0
    /// let constant = expr.get_constant();
    ///
    /// assert_eq!(constant, 0.0);
    /// ```
    pub fn get_constant(&self) -> f64 {
        self.constant.into_inner()
    }

    /// Returns the coefficient associated to a variable in the expression.
    ///
    /// For instance for the expression 2*a+3*b - 4*c + 42, and for the variable c, this would return -4.
    /// Because the variable might not appear at all in the expression, this returns an option.
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr = 2 * LinExpr::<String>::var("a") + 3 * LinExpr::<String>::var("b") - 4 * LinExpr::<String>::var("c") + LinExpr::<String>::constant(42.0);
    /// let coef = expr.get("c"); // should be Some(-4.)
    ///
    /// assert_eq!(coef, Some(-4.0));
    /// ```
    ///
    /// If a variable does not appear in an expression, it returns None.
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr = LinExpr::<String>::var("a");
    /// let coef_b = expr.get("b");
    ///
    /// assert_eq!(coef_b, None);
    /// ```
    ///
    /// But if a coefficient is 0, it is indeed returned.
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr = LinExpr::<String>::var("a") + 0*LinExpr::<String>::var("b");
    /// let coef_b = expr.get("b");
    ///
    /// assert_eq!(coef_b, Some(0.));
    /// ```
    pub fn get<T: Into<V>>(&self, var: T) -> Option<f64> {
        self.coefs.get(&var.into()).map(|&x| x.into_inner())
    }

    /// Returns the set of variables that appears in the expression
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*expr1 - 3 *expr2 - expr3;
    ///
    /// // There are 2 variables: "A" and "B"
    /// assert_eq!(expr.variables(), BTreeSet::from([String::from("A"), String::from("B")]));
    /// ```
    ///
    /// This set can be empty :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr = LinExpr::<String>::constant(42.0);
    ///
    /// assert!(expr.variables().is_empty()); // There are no variables
    /// ```
    ///
    /// But there is a difference between having no coefficient
    /// (the variable does not appear at all in the expression)
    /// and having 0 as a coefficient :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::constant(42.0);
    /// assert!(expr1.variables().is_empty()); // There are no variables
    ///
    /// let expr2 = 0 * LinExpr::<String>::var("A");
    /// // There is actually one variable eventhough its coefficient is 0
    /// assert_eq!(expr2.variables(), BTreeSet::from([String::from("A")]));
    /// ```
    /// You can use [LinExpr::clean] to remove the 0 coefficients.
    pub fn variables(&self) -> BTreeSet<V> {
        self.coefs.keys().cloned().collect()
    }

    /// Returns an iterator over the variables that appears in the expression and their associated coefficients
    ///
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*expr1 - 3 *expr2 - expr3;
    ///
    /// // There are 2 variables: "A" and "B"
    /// assert_eq!(expr.coefficients().map(|(x,y)| (x.clone(), y)).collect::<BTreeMap<_,_>>(), BTreeMap::from([
    ///     (String::from("A"), 2.0),
    ///     (String::from("B"), -3.0)
    /// ]));
    /// ```
    ///
    /// This set can be empty :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr = LinExpr::<String>::constant(42.0);
    ///
    /// assert!(expr.coefficients().len() == 0); // There are no variables
    /// ```
    ///
    /// But there is a difference between having no coefficient
    /// (the variable does not appear at all in the expression)
    /// and having 0 as a coefficient :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::{BTreeSet,BTreeMap};
    /// let expr1 = LinExpr::<String>::constant(42.0);
    /// assert!(expr1.coefficients().len() == 0); // There are no variables
    ///
    /// let expr2 = 0 * LinExpr::<String>::var("A");
    /// // There is actually one variable eventhough its coefficient is 0
    /// assert_eq!(expr2.coefficients().map(|(x,y)| (x.clone(), y)).collect::<BTreeMap<_,_>>(), BTreeMap::from([(String::from("A"),0.0)]));
    /// ```
    /// You can use [LinExpr::clean] to remove the 0 coefficients.
    pub fn coefficients(&self) -> impl ExactSizeIterator<Item = (&V, f64)> {
        self.coefs.iter().map(|(x, y)| (x, y.into_inner()))
    }

    /// Removes variables that have a 0 coefficient.
    ///
    /// This changes the expression and removes variable whose
    /// coefficient is zero.
    ///
    /// After this call, such a variable does not appear at all
    /// in the expression.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let mut expr = 2.0*expr1 - 0*expr2 - expr3;
    ///
    /// // So far, the variables "A" and "B" both appear
    /// // eventhough "B" has a 0 in front of it
    /// assert_eq!(expr.variables(), BTreeSet::from([String::from("A"), String::from("B")]));
    ///
    /// // This should remove the "B" which has a zero coefficient:
    /// expr.clean();
    ///
    /// assert_eq!(expr.variables(), BTreeSet::from([String::from("A")]));
    /// ```
    ///
    /// Other variables and coefficients are unchanged:
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let mut expr = 2.0*expr1 - 0*expr2 - expr3;
    ///
    /// // The coefficient for "A" is 2.
    /// assert_eq!(expr.get("A"), Some(2.0));
    ///
    /// // This should remove the "B" which has a zero coefficient:
    /// expr.clean();
    ///
    /// // But "A" is unchanged:
    /// assert_eq!(expr.get("A"), Some(2.0));
    /// ```
    pub fn clean(&mut self) {
        self.coefs.retain(|_k, v| !f64_is_zero(v.into_inner()));
    }

    /// This works like [LinExpr::clean] but instead of changing
    /// the expression (which requires mutability)
    /// it returns a new cleaned version.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*expr1 - 0*expr2 - expr3;
    ///
    /// // The coefficient for "A" is 2.
    /// assert_eq!(expr.get("A"), Some(2.0));
    /// // The coefficient for "B" is 0.
    /// assert_eq!(expr.get("B"), Some(0.0));
    ///
    /// // This should remove the "B" which has a zero coefficient
    /// let new_expr = expr.cleaned();
    ///
    /// // expr should however be unchanged.
    /// assert_eq!(expr.get("A"), Some(2.0));
    /// assert_eq!(expr.get("B"), Some(0.0));
    /// // but new_expr only has "A"
    /// assert_eq!(new_expr.get("A"), Some(2.0));
    /// assert_eq!(new_expr.get("B"), None);
    /// ```
    pub fn cleaned(&self) -> LinExpr<V> {
        let mut output = self.clone();
        output.clean();
        output
    }

    /// Reduce an expression by replacing part or all
    /// of its variables by values.
    ///
    /// This function takes a list of values for some variables
    /// and substitute these values into the expression.
    /// The result is a new expression (which might be constant).
    /// This can be understood as a partial evaluation of the expression.
    ///
    /// The list of variables can contain variables that do not appear in
    /// the expression. It can also omit variables that do appear since
    /// the evaluation is only partial. As such, this function can't fail.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*&expr1 - 3.0*&expr2 - &expr3;
    /// let expr_reduced = expr.reduce(&BTreeMap::from([
    ///     (String::from("A"), -1.0),
    ///     (String::from("C"), 2.0),
    /// ]));
    ///
    /// let expr_expected = -3.0*&expr2 - 44.0;
    /// assert_eq!(expr_reduced, expr_expected);
    /// ```
    pub fn reduce(&self, vars: &BTreeMap<V, f64>) -> LinExpr<V> {
        let mut new_constant = self.constant.into_inner();
        let mut new_coefs = BTreeMap::new();

        for (v, c) in &self.coefs {
            match vars.get(v) {
                Some(val) => {
                    new_constant += c.into_inner() * (*val);
                }
                None => {
                    new_coefs.insert(v.clone(), *c);
                }
            }
        }

        LinExpr {
            coefs: new_coefs,
            constant: ordered_float::OrderedFloat(new_constant),
        }
    }

    /// Evaluate an expression on a set of values for its variables.
    ///
    /// This function takes a list of values for its variables
    /// and substitute these values into the expression.
    ///
    /// The list of variables can contain variables that do not appear in
    /// the expression.
    ///
    /// However, if some variables of the expression do not have a value
    /// the function will fail and will return as an error the partially reduced
    /// expression.
    ///
    /// If all the variables have a value assigned to them, then the function
    /// succeeds and returns a single floating point value.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*&expr1 - 3.0*&expr2 - &expr3;
    ///
    /// let bad_eval = expr.eval(&BTreeMap::from([
    ///     (String::from("A"), -1.0),
    ///     (String::from("C"), 2.0),
    /// ]));
    /// let bad_eval_expected = Err(-3.0*&expr2 - 44.0);
    /// assert_eq!(bad_eval, bad_eval_expected);
    ///
    /// let good_eval = expr.eval(&BTreeMap::from([
    ///     (String::from("A"), -1.0),
    ///     (String::from("B"), -3.0),
    ///     (String::from("C"), 2.0),
    /// ]));
    /// let good_eval_expected = Ok(-35.0);
    /// assert_eq!(good_eval, good_eval_expected);
    /// ```
    pub fn eval(&self, vars: &BTreeMap<V, f64>) -> Result<f64, LinExpr<V>> {
        let reduced = self.reduce(vars);

        if !reduced.coefs.is_empty() {
            return Err(reduced);
        }

        Ok(reduced.get_constant())
    }

    /// Returns the minimum possible value of an expression from a list of variable ranges.
    ///
    /// This will return the minimum possible value for the expression given a range
    /// of possible values for each variable. This is the absolutely worst case scenario.
    /// If some variable has no range given, it is assumed the variable can take any value.
    ///
    /// The range for each variable is described by [super::Variable].
    ///
    /// If there is no minimum, then -infinity is returned. This means that arbitrarily low values
    /// are possible.
    ///
    /// For instance:
    /// ```
    /// # use collomatique_ilp::{linexpr::LinExpr, Variable};
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*&expr1 - 3.0*&expr2 - &expr3;
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::binary()),
    /// ]);
    /// let minimum_expected = -45.0; // Here all variables have a range so we can actually find a bound
    /// assert_eq!(expr.compute_minimum(&ranges), minimum_expected);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    /// ]);
    /// let minimum_expected = -f64::INFINITY; // Because no range is given to B, there is no lower bound
    /// assert_eq!(expr.compute_minimum(&ranges), minimum_expected);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::non_negative()),
    /// ]);
    /// let minimum_expected = -f64::INFINITY; // Because B is not bounded from above, there is no lower bound for the linear expr
    /// assert_eq!(expr.compute_minimum(&ranges), minimum_expected);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::non_positive()),
    /// ]);
    /// let minimum_expected = -42.0; // But if B is bounded from above, there is a lower bound for the linear expr
    /// assert_eq!(expr.compute_minimum(&ranges), minimum_expected);
    /// ```
    pub fn compute_minimum(&self, vars: &BTreeMap<V, super::Variable>) -> f64 {
        *self.compute_range(vars).start()
    }

    /// Returns the maximum possible value of an expression from a list of variable ranges.
    ///
    /// This will return the maximum possible value for the expression given a range
    /// of possible values for each variable. This is the absolutely worst case scenario.
    /// If some variable has no range given, it is assumed the variable can take any value.
    ///
    /// The range for each variable is described by [super::Variable].
    ///
    /// If there is no maximum, then +infinity is returned. This means that arbitrarily high values
    /// are possible.
    ///
    /// For instance:
    /// ```
    /// # use collomatique_ilp::{linexpr::LinExpr, Variable};
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*&expr1 - 3.0*&expr2 - &expr3;
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::binary()),
    /// ]);
    /// let maximum_expected = -40.0; // Here all variables have a range so we can actually find a bound
    /// assert_eq!(expr.compute_maximum(&ranges), maximum_expected);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    /// ]);
    /// let maximum_expected = f64::INFINITY; // Because no range is given to B, there is no upper bound
    /// assert_eq!(expr.compute_maximum(&ranges), maximum_expected);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::non_negative()),
    /// ]);
    /// let maximum_expected = -40.0; // If B is bounded from below, there is an upper bound for the linear expr
    /// assert_eq!(expr.compute_maximum(&ranges), maximum_expected);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::non_positive()),
    /// ]);
    /// let maximum_expected = f64::INFINITY; // But if B is not bounded from below, there is no upper bound for the linear expr
    /// assert_eq!(expr.compute_maximum(&ranges), maximum_expected);
    /// ```
    pub fn compute_maximum(&self, vars: &BTreeMap<V, super::Variable>) -> f64 {
        *self.compute_range(vars).end()
    }

    /// Returns the range of possible values of an expression from a list of variable ranges.
    ///
    /// This will return the range of possible values for the expression given a range
    /// of possible values for each variable. This is the absolutely worst case scenario.
    /// If some variable has no range given, it is assumed the variable can take any value.
    ///
    /// The range for each variable is described by [super::Variable].
    ///
    /// If there is no minimum, then -infinity is given for the lower bound. This means that arbitrarily low values
    /// are possible.
    /// If there is no maximum, then +infinity is given for the upper bound. This means that arbitrarily high values
    /// are possible.
    ///
    /// For instance:
    /// ```
    /// # use collomatique_ilp::{linexpr::LinExpr, Variable};
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*&expr1 - 3.0*&expr2 - &expr3;
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::binary()),
    /// ]);
    /// assert_eq!(expr.compute_range(&ranges), -45.0..=-40.0);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    /// ]);
    /// assert_eq!(expr.compute_range(&ranges), -f64::INFINITY..=f64::INFINITY);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::non_negative()),
    /// ]);
    /// assert_eq!(expr.compute_range(&ranges), -f64::INFINITY..=-40.0);
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::non_positive()),
    /// ]);
    /// assert_eq!(expr.compute_range(&ranges), -42.0..=f64::INFINITY);
    /// ```
    pub fn compute_range(
        &self,
        vars: &BTreeMap<V, super::Variable>,
    ) -> std::ops::RangeInclusive<f64> {
        self.compute_range_with(|v| vars.get(v).cloned())
    }

    /// Returns the range of possible values of an expression from a closure.
    ///
    /// Compared to [Self::compute_range], [Self::compute_range_with] takes a closure.
    /// This means we can evaluate dynamically what variables should be defined. This is useful in some
    /// contexts.
    ///
    /// This will return the range of possible values for the expression given a range
    /// of possible values for each variable. This is the absolutely worst case scenario.
    /// If some variable has no range given, it is assumed the variable can take any value.
    ///
    /// The range for each variable is described by [super::Variable].
    ///
    /// If there is no minimum, then -infinity is given for the lower bound. This means that arbitrarily low values
    /// are possible.
    /// If there is no maximum, then +infinity is given for the upper bound. This means that arbitrarily high values
    /// are possible.
    ///
    /// For instance:
    /// ```
    /// # use collomatique_ilp::{linexpr::LinExpr, Variable};
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*&expr1 - 3.0*&expr2 - &expr3;
    ///
    /// let ranges = |v: &str| match v {
    ///     "A" => Some(Variable::binary()),
    ///     "B" => Some(Variable::binary()),
    ///     _ => None,
    /// };
    /// assert_eq!(expr.compute_range_with(ranges), -45.0..=-40.0);
    ///
    /// let ranges = |v: &str| match v {
    ///     "A" => Some(Variable::binary()),
    ///     _ => None,
    /// };
    /// assert_eq!(expr.compute_range_with(&ranges), -f64::INFINITY..=f64::INFINITY);
    ///
    /// let ranges = |v: &str| match v {
    ///     "A" => Some(Variable::binary()),
    ///     "B" => Some(Variable::non_negative()),
    ///     _ => None,
    /// };
    /// assert_eq!(expr.compute_range_with(&ranges), -f64::INFINITY..=-40.0);
    ///
    /// let ranges = |v: &str| match v {
    ///     "A" => Some(Variable::binary()),
    ///     "B" => Some(Variable::non_positive()),
    ///     _ => None,
    /// };
    /// assert_eq!(expr.compute_range_with(&ranges), -42.0..=f64::INFINITY);
    /// ```
    pub fn compute_range_with<U: ?Sized, F: FnMut(&U) -> Option<super::Variable>>(
        &self,
        mut f: F,
    ) -> std::ops::RangeInclusive<f64>
    where
        V: Borrow<U>,
    {
        let mut minimum = self.constant.0;
        let mut maximum = self.constant.0;

        for (var, coef) in &self.coefs {
            let var_def = f(var.borrow());
            let var_range = match var_def {
                Some(def) => {
                    let min_value = def.get_min().unwrap_or(-f64::INFINITY);
                    let max_value = def.get_max().unwrap_or(f64::INFINITY);

                    min_value..=max_value
                }
                None => -f64::INFINITY..=f64::INFINITY,
            };

            // Ignore variables that have coefficient 0
            if f64_is_zero(coef.0) {
                continue;
            }

            if coef.0 > 0.0 {
                minimum += coef.0 * var_range.start();
                maximum += coef.0 * var_range.end();
            } else {
                minimum += coef.0 * var_range.end();
                maximum += coef.0 * var_range.start();
            }
        }

        minimum..=maximum
    }

    /// Transmute variables
    ///
    /// This method creates a new [LinExpr] with a different
    /// variable type.
    ///
    /// This is useful when an expression was originally written
    /// using some variable type but must be used in a context of a
    /// wider variable type.
    ///
    /// For instance :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// // We write some expression using variables from type V1
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V1 {
    ///     A,
    ///     B,
    ///     C,
    /// }
    ///
    /// let expr = LinExpr::var(V1::A) + 2.0*LinExpr::var(V1::B) + 3.0*LinExpr::var(V1::C);
    ///
    /// // We do something more complex that has more variables
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V2 {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    ///     F,
    /// }
    ///
    /// // We can "transmute" the old expression into the more complex setting
    /// let expr_transmute = expr.transmute(|v| match v {
    ///     V1::A => V2::A,
    ///     V1::B => V2::B,
    ///     V1::C => V2::C,
    /// });
    ///
    /// let expected_result = LinExpr::var(V2::A) + 2.0*LinExpr::var(V2::B) + 3.0*LinExpr::var(V2::C);
    /// assert_eq!(expr_transmute, expected_result);
    /// ```
    pub fn transmute<U: UsableData, F: FnMut(&V) -> U>(&self, mut f: F) -> LinExpr<U> {
        self.try_transmute(|x| Some(f(x)))
            .expect("No undefined transmutation is possible")
    }

    /// Transmute variables
    ///
    /// Works like [Self::transmute] but consumes the expression
    pub fn into_transmuted<U: UsableData, F: FnMut(V) -> U>(self, mut f: F) -> LinExpr<U> {
        let mut expr = LinExpr::constant(self.get_constant());

        for (v, c) in self.coefs {
            expr = expr + c.into_inner() * LinExpr::var(f(v));
        }

        expr
    }

    /// Transmute variables
    ///
    /// This method is a faillible version of [LinExpr::transmute].
    ///
    /// This is useful when an expression was originally written
    /// using some variable type but must be used in a context of a
    /// different variable type and the conversion between the two might fail.
    ///
    /// For instance :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// // We write some expression using variables from type V1
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V1 {
    ///     A,
    ///     B,
    ///     C,
    /// }
    ///
    /// let expr = LinExpr::var(V1::A) + 2.0*LinExpr::var(V1::B) + 3.0*LinExpr::var(V1::C);
    ///
    /// // We do something more complex that has more variables
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V2 {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    ///     F,
    /// }
    ///
    /// // We can "transmute" the old expression into the more complex setting
    /// let expr_transmute = expr.try_transmute(|v| match v {
    ///     V1::A => Some(V2::A),
    ///     V1::B => Some(V2::B),
    ///     V1::C => Some(V2::C),
    /// }).unwrap(); // No issue here
    ///
    /// let expected_result = LinExpr::var(V2::A) + 2.0*LinExpr::var(V2::B) + 3.0*LinExpr::var(V2::C);
    /// assert_eq!(expr_transmute, expected_result);
    /// ```
    /// However, this code will fail correctly because `V3` does not encore `C` correctly:
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V1 {
    ///     A,
    ///     B,
    ///     C,
    /// }
    ///
    /// let expr = LinExpr::var(V1::A) + 2.0*LinExpr::var(V1::B) + 3.0*LinExpr::var(V1::C);
    ///
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V3 {
    ///     A,
    ///     B,
    ///     D,
    ///     E,
    ///     F,
    /// }
    ///
    /// let expr_transmute = expr.try_transmute(|v| match v {
    ///     V1::A => Some(V3::A),
    ///     V1::B => Some(V3::B),
    ///     V1::C => None,
    /// }); // This will actually return None because of the C variable;
    ///
    /// assert_eq!(expr_transmute, None);
    /// ```
    pub fn try_transmute<U: UsableData, F: FnMut(&V) -> Option<U>>(
        &self,
        mut f: F,
    ) -> Option<LinExpr<U>> {
        let mut expr = LinExpr::constant(self.get_constant());

        for (v, c) in &self.coefs {
            expr = expr + c.into_inner() * LinExpr::var(f(v)?);
        }

        Some(expr)
    }
}

impl<V: UsableData> LinExpr<V> {
    /// Builds a new constraint of the form: "self <= rhs"
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let constraint = (2.0*expr1 - expr2).leq(&expr3);
    ///
    /// // Displays "2*A + (-1)*B + (-42) <= 0"
    /// println!("{}", constraint);
    /// # assert_eq!(format!("{}", constraint), "2*A + (-1)*B + (-42) <= 0");
    /// ```
    pub fn leq(&self, rhs: &LinExpr<V>) -> Constraint<V> {
        Constraint {
            expr: self - rhs,
            symbol: EqSymbol::LessThan,
        }
    }

    /// Builds a new constraint of the form: "self >= rhs"
    ///
    /// Internally, this is still represented by a "less than" ("<=")
    /// constraint. It is simply equivalent to calling `rhs.leq(self)`.
    ///
    /// But it is sometimes convenient and more readable to use this function.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let constraint1 = (2.0*&expr1 - &expr2).geq(&expr3);
    /// let constraint2 = expr3.leq(&(2.0*&expr1 - &expr2));
    ///
    /// assert_eq!(constraint1, constraint2);
    /// ```
    pub fn geq(&self, rhs: &LinExpr<V>) -> Constraint<V> {
        Constraint {
            expr: rhs - self,
            symbol: EqSymbol::LessThan,
        }
    }

    /// Builds a new constraint of the form: "self = rhs"
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let constraint = (2.0*expr1 - expr2).eq(&expr3);
    ///
    /// // Displays "2*A + (-1)*B + (-42) = 0"
    /// println!("{}", constraint);
    /// # assert_eq!(format!("{}", constraint), "2*A + (-1)*B + (-42) = 0");
    /// ```
    pub fn eq(&self, rhs: &LinExpr<V>) -> Constraint<V> {
        Constraint {
            expr: self - rhs,
            symbol: EqSymbol::Equals,
        }
    }
}

impl<V: UsableData> Constraint<V> {
    /// Returns true if the constraints is trivially true
    ///
    /// A constraint is trivially true if there are no variables in it
    /// and it is indeed satisfied
    pub fn is_trivially_true(&self) -> bool {
        if !self.expr.coefs.is_empty() {
            return false;
        }
        match self.symbol {
            EqSymbol::Equals => f64_is_zero(self.expr.constant.0),
            EqSymbol::LessThan => f64_is_positive(-self.expr.constant.0),
        }
    }

    /// Returns the variables that appear in the constraint.
    ///
    /// As for [LinExpr::variables], if a variable has a zero coefficient
    /// it is still listed in the list of variables that appear.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    /// let expr4 = LinExpr::<String>::var("C");
    ///
    /// let expr = 2.0*expr1 - 3 *expr2 - expr3;
    /// let constraint = expr.leq(&expr4);
    ///
    /// // There are 3 variables: "A", "B" and "C"
    /// assert_eq!(constraint.variables(), BTreeSet::from([String::from("A"), String::from("B"), String::from("C")]));
    /// ```
    ///
    /// This set can be empty :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::constant(42.0);
    /// let expr2 = LinExpr::<String>::constant(-1.0);
    ///
    /// let constraint = expr1.eq(&expr2);
    ///
    /// assert!(constraint.variables().is_empty()); // There are no variables
    /// ```
    ///
    /// But there is a difference between having no coefficient
    /// (the variable does not appear at all in the expression)
    /// and having 0 as a coefficient :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::constant(42.0);
    /// let expr2 = LinExpr::<String>::constant(-1.0);
    /// let constraint1 = expr1.leq(&expr2);
    /// assert!(constraint1.variables().is_empty()); // There are no variables
    ///
    /// let expr3 = 0 * LinExpr::<String>::var("A");
    /// let constraint2 = (&expr1 + &expr3).leq(&expr2);
    /// // There is actually one variable eventhough its coefficient is 0
    /// assert_eq!(constraint2.variables(), BTreeSet::from([String::from("A")]));
    /// ```
    /// You can use [Constraint::clean] to remove the 0 coefficients.
    pub fn variables(&self) -> BTreeSet<V> {
        self.expr.variables()
    }

    /// Returns an iterator over the variables that appear in the constraint and their associated values.
    ///
    /// As for [LinExpr::coefficients], if a variable has a zero coefficient
    /// it is still listed in the list of variables that appear.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    /// let expr4 = LinExpr::<String>::var("C");
    ///
    /// let expr = 2.0*expr1 - 3 *expr2 - expr3;
    /// let constraint = expr.leq(&expr4);
    ///
    /// // There are 3 variables: "A", "B" and "C"
    /// assert_eq!(constraint.coefficients().map(|(x,y)| (x.clone(), y)).collect::<BTreeMap<_,_>>(), BTreeMap::from([
    ///     (String::from("A"),2.0),
    ///     (String::from("B"),-3.0),
    ///     (String::from("C"),-1.0)
    /// ]));
    /// ```
    ///
    /// This set can be empty :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::constant(42.0);
    /// let expr2 = LinExpr::<String>::constant(-1.0);
    ///
    /// let constraint = expr1.eq(&expr2);
    ///
    /// assert!(constraint.coefficients().len() == 0); // There are no variables
    /// ```
    ///
    /// But there is a difference between having no coefficient
    /// (the variable does not appear at all in the expression)
    /// and having 0 as a coefficient :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::constant(42.0);
    /// let expr2 = LinExpr::<String>::constant(-1.0);
    /// let constraint1 = expr1.leq(&expr2);
    /// assert!(constraint1.coefficients().len() == 0); // There are no variables
    ///
    /// let expr3 = 0 * LinExpr::<String>::var("A");
    /// let constraint2 = (&expr1 + &expr3).leq(&expr2);
    /// // There is actually one variable eventhough its coefficient is 0
    /// assert_eq!(constraint2.coefficients().map(|(x,y)| (x.clone(),y)).collect::<BTreeMap<_,_>>(), BTreeMap::from([(String::from("A"),0.0)]));
    /// ```
    /// You can use [Constraint::clean] to remove the 0 coefficients.
    pub fn coefficients(&self) -> impl ExactSizeIterator<Item = (&V, f64)> {
        self.expr.coefficients()
    }

    /// Returns the coefficient for a variable in the constraint.
    ///
    /// This works similarly to [LinExpr::get].
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let constraint = expr.leq(&LinExpr::<String>::constant(1.0));
    ///
    /// assert_eq!(constraint.get_var("A"), Some(2.0));
    /// ```
    ///
    /// If a variable does not appear
    /// at all, it returns None:
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let constraint = expr.leq(&LinExpr::<String>::constant(1.0));
    ///
    /// assert_eq!(constraint.get_var("C"), None);
    /// ```
    ///
    /// However, if a variable appears but it is coefficient is zero,
    /// it does return a value :
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr = 2*LinExpr::<String>::var("A") + 0 * LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let constraint = expr.leq(&LinExpr::<String>::constant(1.0));
    ///
    /// assert_eq!(constraint.get_var("B"), Some(0.));
    /// ```
    ///
    /// Internally, a constraint only has a left hand side. So, the signs are changed when
    /// the coefficient was originally on the right hand side.
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr1 = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let expr2 = 1*LinExpr::<String>::var("A") + 3*LinExpr::<String>::var("C") + LinExpr::<String>::constant(-2.0);
    /// let constraint = expr1.leq(&expr2);
    ///
    /// assert_eq!(constraint.get_var("C"), Some(-3.0));
    /// ```
    ///
    /// And the constraints are always "<=" or "=". So if a constraints was built with [LinExpr::geq],
    /// signs are also reversed:
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr1 = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let expr2 = 1*LinExpr::<String>::var("A") + 3*LinExpr::<String>::var("C") + LinExpr::<String>::constant(-2.0);
    /// let constraint = expr1.geq(&expr2);
    ///
    /// assert_eq!(constraint.get_var("B"), Some(1.0));
    /// ```
    ///
    /// And there can be only one coefficient per variable. So multiple coefficients from left and right hand side
    /// are merged into one lhs coefficient after computation:
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr1 = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let expr2 = 1*LinExpr::<String>::var("A") + 3*LinExpr::<String>::var("C") + LinExpr::<String>::constant(-2.0);
    /// let constraint = expr1.leq(&expr2);
    ///
    /// assert_eq!(constraint.get_var("A"), Some(1.0));
    /// ```
    pub fn get_var<T: Into<V>>(&self, var: T) -> Option<f64> {
        self.expr.get(var)
    }

    /// Returns the (in)equality symbol used in the constraint.
    ///
    /// It can only be "<=" or "=". ">=" is tranformed internally into a "<=" symbol.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::{LinExpr, EqSymbol};
    /// let expr1 = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let expr2 = 1*LinExpr::<String>::var("A") + 3*LinExpr::<String>::var("C");
    ///
    /// let constraint1 = expr1.leq(&expr2);
    /// let constraint2 = expr1.geq(&expr2);
    /// let constraint3 = expr1.eq(&expr2);
    ///
    /// assert_eq!(constraint1.get_symbol(), EqSymbol::LessThan);
    /// assert_eq!(constraint2.get_symbol(), EqSymbol::LessThan);
    /// assert_eq!(constraint3.get_symbol(), EqSymbol::Equals);
    /// ```
    pub fn get_symbol(&self) -> EqSymbol {
        self.symbol
    }

    /// Returns the constant on the left hand side.
    ///
    /// This works like [LinExpr::get_constant].
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr1 = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let expr2 = 1*LinExpr::<String>::var("A") + 3*LinExpr::<String>::var("C");
    /// let constraint = expr1.leq(&expr2);
    ///
    /// assert_eq!(constraint.get_constant(), 42.0);
    /// ```
    ///
    /// Remember, there is always only one constant that was obtained by merging all the
    /// constants to the lhs:
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr1 = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let expr2 = 1*LinExpr::<String>::var("A") + 3*LinExpr::<String>::var("C") + LinExpr::<String>::constant(-2.0);
    /// let constraint = expr1.leq(&expr2);
    ///
    /// assert_eq!(constraint.get_constant(), 44.0);
    /// ```
    ///
    /// And even if it does even out and the constant is zero, there is still a constant:
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr1 = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let expr2 = 1*LinExpr::<String>::var("A") + 3*LinExpr::<String>::var("C") + LinExpr::<String>::constant(42.0);
    /// let constraint = expr1.leq(&expr2);
    ///
    /// assert_eq!(constraint.get_constant(), 0.0);
    /// ```
    pub fn get_constant(&self) -> f64 {
        self.expr.get_constant()
    }

    /// This returns the internal expression used by the constraint
    /// to represent the left hand side (the rhs is always 0 internally).
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// let expr1 = 2*LinExpr::<String>::var("A") - LinExpr::<String>::var("B") + LinExpr::<String>::constant(42.0);
    /// let expr2 = 1*LinExpr::<String>::var("A") + 3*LinExpr::<String>::var("C") + LinExpr::<String>::constant(-2.0);
    /// let constraint = expr1.leq(&expr2);
    ///
    /// assert_eq!(*constraint.get_lhs(), &expr1 - &expr2);
    /// ```
    pub fn get_lhs(&self) -> &LinExpr<V> {
        &self.expr
    }

    /// Removes variables that have a 0 coefficient.
    ///
    /// It works similarly to [LinExpr::clean].
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*expr1 - 0*expr2 - expr3;
    /// let mut constraint = expr.leq(&LinExpr::constant(2.0));
    ///
    /// // So far, the variables "A" and "B" both appear
    /// // eventhough "B" has a 0 in front of it
    /// assert_eq!(constraint.variables(), BTreeSet::from([String::from("A"), String::from("B")]));
    ///
    /// // This should remove the "B" which has a zero coefficient:
    /// constraint.clean();
    ///
    /// assert_eq!(constraint.variables(), BTreeSet::from([String::from("A")]));
    /// ```
    pub fn clean(&mut self) {
        self.expr.clean();
    }

    /// Removes variables that have a 0 coefficient (like [Constraint::clean]
    /// but does it without mutability and returns a new constraint.
    ///
    /// It works similarly to [LinExpr::cleaned].
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeSet;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*expr1 - 0*expr2 - expr3;
    /// let constraint = expr.leq(&LinExpr::constant(2.0));
    ///
    /// // So far, the variables "A" and "B" both appear
    /// // eventhough "B" has a 0 in front of it
    /// assert_eq!(constraint.variables(), BTreeSet::from([String::from("A"), String::from("B")]));
    ///
    /// // This should remove the "B" which has a zero coefficient:
    /// let new_constraint = constraint.cleaned();
    ///
    /// assert_eq!(constraint.variables(), BTreeSet::from([String::from("A"), String::from("B")]));
    /// assert_eq!(new_constraint.variables(), BTreeSet::from([String::from("A")]));
    /// ```
    pub fn cleaned(&self) -> Constraint<V> {
        let mut output = self.clone();
        output.clean();
        output
    }

    /// Reduce a constraint by replacing part or all
    /// of its variables by values.
    ///
    /// This function takes a list of values for some variables
    /// and substitute these values into the constraint.
    /// The result is a new constraint (which might be trivial).
    /// This can be understood as a partial evaluation of the constraint.
    ///
    /// The list of variables can contain variables that do not appear in
    /// the constraint. It can also omit variables that do appear since
    /// the evaluation is only partial. As such, this function can't fail.
    ///
    /// ```
    /// # use collomatique_ilp::linexpr::LinExpr;
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    /// let expr3 = LinExpr::<String>::constant(42.0);
    ///
    /// let expr = 2.0*&expr1 - 3.0*&expr2 - &expr3;
    /// let constraint = expr.leq(&LinExpr::constant(0.0));
    ///
    /// let constraint_reduced = constraint.reduce(&BTreeMap::from([
    ///     (String::from("A"), -1.0),
    ///     (String::from("C"), 2.0),
    /// ]));
    ///
    /// let constraint_expected = (-3.0*&expr2 - 44.0).leq(&LinExpr::constant(0.0));
    /// assert_eq!(constraint_reduced, constraint_expected);
    /// ```
    pub fn reduce(&self, vars: &BTreeMap<V, f64>) -> Constraint<V> {
        let new_expr = self.expr.reduce(vars);

        Constraint {
            expr: new_expr,
            symbol: self.symbol,
        }
    }

    /// Checks if the constraint is still possible to satisfy
    /// given ranges of possible values for each variable.
    ///
    /// This will return `true` if it is still possible for the constraint to be
    /// satisfied given the ranges. This is based on a range evaluation (using [LinExpr::compute_range]) of
    /// the possible values for the linear expression in the constraint.
    /// If some variable has no range given, it is assumed the variable can take any value.
    ///
    /// The range for each variable is described by [super::Variable].
    ///
    /// For instance:
    /// ```
    /// # use collomatique_ilp::{linexpr::LinExpr, Variable};
    /// # use std::collections::BTreeMap;
    /// let expr1 = LinExpr::<String>::var("A");
    /// let expr2 = LinExpr::<String>::var("B");
    ///
    /// let constraint1 = (2.0*&expr1 - 3.0*&expr2).leq(&LinExpr::constant(0.0));
    /// let constraint2 = (2.0*&expr1 - 3.0*&expr2).leq(&LinExpr::constant(-4.0));
    ///
    /// let ranges = BTreeMap::from([
    ///     (String::from("A"), Variable::binary()),
    ///     (String::from("B"), Variable::binary()),
    /// ]);
    /// assert_eq!(constraint1.range_check(&ranges), true); // It is possible to satisfy constraint1 with binary variable
    /// assert_eq!(constraint2.range_check(&ranges), false); // No binary variable will ever satisfy constraint2 (because the lhs >= -3.0).
    /// ```
    pub fn range_check(&self, vars: &BTreeMap<V, super::Variable>) -> bool {
        match self.symbol {
            EqSymbol::Equals => {
                let possible_range = self.expr.compute_range(vars);
                possible_range.contains(&0.0)
            }
            EqSymbol::LessThan => {
                let minimum = self.expr.compute_minimum(vars);
                minimum <= 0.0
            }
        }
    }

    /// Transmute variables
    ///
    /// This method creates a new [Constraint] with a different
    /// variable type.
    ///
    /// This is useful when a constraint was originally written
    /// using some variable type but must be used in a context of a
    /// wider variable type.
    ///
    /// For instance :
    /// ```
    /// # use collomatique_ilp::linexpr::{LinExpr, Constraint};
    /// // We write some expression using variables from type V1
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V1 {
    ///     A,
    ///     B,
    ///     C,
    /// }
    ///
    /// let expr = LinExpr::var(V1::A) + 2.0*LinExpr::var(V1::B) + 3.0*LinExpr::var(V1::C);
    /// let constraint = expr.leq(&LinExpr::constant(4.0));
    ///
    /// // We do something more complex that has more variables
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V2 {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    ///     F,
    /// }
    ///
    /// // We can "transmute" the old expression into the more complex setting
    /// let constraint_transmute = constraint.transmute(|v| match v {
    ///     V1::A => V2::A,
    ///     V1::B => V2::B,
    ///     V1::C => V2::C,
    /// });
    ///
    /// let expected_result = (LinExpr::var(V2::A) + 2.0*LinExpr::var(V2::B) + 3.0*LinExpr::var(V2::C)).leq(&LinExpr::constant(4.0));
    /// assert_eq!(constraint_transmute, expected_result);
    /// ```
    pub fn transmute<U: UsableData, F: FnMut(&V) -> U>(&self, f: F) -> Constraint<U> {
        Constraint {
            symbol: self.symbol,
            expr: self.expr.transmute(f),
        }
    }

    /// Transmute variables
    ///
    /// Works like [Self::transmute] but consumes the constraint.
    ///
    /// For instance :
    /// ```
    /// # use collomatique_ilp::linexpr::{LinExpr, Constraint};
    /// // We write some expression using variables from type V1
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V1 {
    ///     A,
    ///     B,
    ///     C,
    /// }
    ///
    /// let expr = LinExpr::var(V1::A) + 2.0*LinExpr::var(V1::B) + 3.0*LinExpr::var(V1::C);
    /// let constraint = expr.leq(&LinExpr::constant(4.0));
    ///
    /// // We do something more complex that has more variables
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V2 {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    ///     F,
    /// }
    ///
    /// // We can "transmute" the old expression into the more complex setting
    /// let constraint_transmute = constraint.into_transmuted(|v| match v {
    ///     V1::A => V2::A,
    ///     V1::B => V2::B,
    ///     V1::C => V2::C,
    /// });
    ///
    /// let expected_result = (LinExpr::var(V2::A) + 2.0*LinExpr::var(V2::B) + 3.0*LinExpr::var(V2::C)).leq(&LinExpr::constant(4.0));
    /// assert_eq!(constraint_transmute, expected_result);
    /// ```
    pub fn into_transmuted<U: UsableData, F: FnMut(V) -> U>(self, f: F) -> Constraint<U> {
        Constraint {
            symbol: self.symbol,
            expr: self.expr.into_transmuted(f),
        }
    }

    /// Transmute variables
    ///
    /// This method is a faillible version of [Constraint::transmute].
    ///
    /// For instance, this works :
    /// ```
    /// # use collomatique_ilp::linexpr::{LinExpr, Constraint};
    /// // We write some expression using variables from type V1
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V1 {
    ///     A,
    ///     B,
    ///     C,
    /// }
    ///
    /// let expr = LinExpr::var(V1::A) + 2.0*LinExpr::var(V1::B) + 3.0*LinExpr::var(V1::C);
    /// let constraint = expr.leq(&LinExpr::constant(4.0));
    ///
    /// // We do something more complex that has more variables
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V2 {
    ///     A,
    ///     B,
    ///     C,
    ///     D,
    ///     E,
    ///     F,
    /// }
    ///
    /// // We can "transmute" the old expression into the more complex setting
    /// let constraint_transmute = constraint.try_transmute(|v| match v {
    ///     V1::A => Some(V2::A),
    ///     V1::B => Some(V2::B),
    ///     V1::C => Some(V2::C),
    /// }).unwrap();
    ///
    /// let expected_result = (LinExpr::var(V2::A) + 2.0*LinExpr::var(V2::B) + 3.0*LinExpr::var(V2::C)).leq(&LinExpr::constant(4.0));
    /// assert_eq!(constraint_transmute, expected_result);
    /// ```
    /// But this transmutation fails:
    /// ```
    /// # use collomatique_ilp::linexpr::{LinExpr, Constraint};
    /// // We write some expression using variables from type V1
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V1 {
    ///     A,
    ///     B,
    ///     C,
    /// }
    ///
    /// let expr = LinExpr::var(V1::A) + 2.0*LinExpr::var(V1::B) + 3.0*LinExpr::var(V1::C);
    /// let constraint = expr.leq(&LinExpr::constant(4.0));
    ///
    /// // We do something more complex that has more variables
    /// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// enum V3 {
    ///     A,
    ///     B,
    ///     D,
    ///     E,
    ///     F,
    /// }
    ///
    /// // We can "transmute" the old expression into the more complex setting
    /// let constraint_transmute = constraint.try_transmute(|v| match v {
    ///     V1::A => Some(V3::A),
    ///     V1::B => Some(V3::B),
    ///     V1::C => None,
    /// });
    ///
    /// assert_eq!(constraint_transmute, None);
    /// ```
    pub fn try_transmute<U: UsableData, F: FnMut(&V) -> Option<U>>(
        &self,
        f: F,
    ) -> Option<Constraint<U>> {
        Some(Constraint {
            symbol: self.symbol,
            expr: self.expr.try_transmute(f)?,
        })
    }
}

impl<V: UsableData + std::fmt::Display> std::fmt::Display for LinExpr<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.coefs.is_empty() && f64_is_zero(self.constant.into_inner()) {
            write!(f, "0.0")?;
            return Ok(());
        }

        let mut it = self.coefs.iter().peekable();
        while let Some((key, value)) = it.next() {
            if value.is_sign_negative() {
                write!(f, "({})*{}", value, key)?;
            } else {
                write!(f, "{}*{}", value, key)?;
            }

            if it.peek().is_some() || !f64_is_zero(self.constant.into_inner()) {
                write!(f, " + ")?;
            }
        }

        if !f64_is_zero(self.constant.into_inner()) || self.coefs.is_empty() {
            if self.constant.is_sign_negative() {
                write!(f, "({})", self.constant)?
            } else {
                write!(f, "{}", self.constant)?
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for EqSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                EqSymbol::Equals => "=",
                EqSymbol::LessThan => "<=",
            }
        )
    }
}

impl<V: UsableData + std::fmt::Display> std::fmt::Display for Constraint<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} 0", self.expr, self.symbol)
    }
}

impl<V: UsableData> std::ops::Add for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: &LinExpr<V>) -> Self::Output {
        let mut output = LinExpr {
            coefs: self.coefs.clone(),
            constant: self.constant,
        };

        for (key, value) in rhs.coefs.iter() {
            if let Some(coef) = output.coefs.get_mut(key) {
                *coef += value;
            } else {
                output.coefs.insert(key.clone(), *value);
            }
        }

        output.constant += rhs.constant;

        output
    }
}

impl<V: UsableData> std::ops::Add for LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: LinExpr<V>) -> Self::Output {
        &self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<LinExpr<V>> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: LinExpr<V>) -> Self::Output {
        self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&LinExpr<V>> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: &LinExpr<V>) -> Self::Output {
        &self + rhs
    }
}

impl<V: UsableData> std::ops::Add<&f64> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: &f64) -> Self::Output {
        self + LinExpr::constant(*rhs)
    }
}

impl<V: UsableData> std::ops::Add<f64> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: f64) -> Self::Output {
        self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&f64> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: &f64) -> Self::Output {
        &self + rhs
    }
}

impl<V: UsableData> std::ops::Add<f64> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: f64) -> Self::Output {
        &self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&i32> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: &i32) -> Self::Output {
        self + f64::from(*rhs)
    }
}

impl<V: UsableData> std::ops::Add<i32> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: i32) -> Self::Output {
        self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&i32> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: &i32) -> Self::Output {
        &self + rhs
    }
}

impl<V: UsableData> std::ops::Add<i32> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn add(self, rhs: i32) -> Self::Output {
        &self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&LinExpr<V>> for &f64 {
    type Output = LinExpr<V>;

    fn add(self, rhs: &LinExpr<V>) -> Self::Output {
        rhs + self
    }
}

impl<V: UsableData> std::ops::Add<LinExpr<V>> for &f64 {
    type Output = LinExpr<V>;

    fn add(self, rhs: LinExpr<V>) -> Self::Output {
        self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&LinExpr<V>> for f64 {
    type Output = LinExpr<V>;

    fn add(self, rhs: &LinExpr<V>) -> Self::Output {
        &self + rhs
    }
}

impl<V: UsableData> std::ops::Add<LinExpr<V>> for f64 {
    type Output = LinExpr<V>;

    fn add(self, rhs: LinExpr<V>) -> Self::Output {
        &self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&LinExpr<V>> for &i32 {
    type Output = LinExpr<V>;

    fn add(self, rhs: &LinExpr<V>) -> Self::Output {
        rhs + self
    }
}

impl<V: UsableData> std::ops::Add<LinExpr<V>> for &i32 {
    type Output = LinExpr<V>;

    fn add(self, rhs: LinExpr<V>) -> Self::Output {
        self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&LinExpr<V>> for i32 {
    type Output = LinExpr<V>;

    fn add(self, rhs: &LinExpr<V>) -> Self::Output {
        &self + rhs
    }
}

impl<V: UsableData> std::ops::Add<LinExpr<V>> for i32 {
    type Output = LinExpr<V>;

    fn add(self, rhs: LinExpr<V>) -> Self::Output {
        &self + &rhs
    }
}

impl<V: UsableData> std::ops::Mul<&LinExpr<V>> for &f64 {
    type Output = LinExpr<V>;

    fn mul(self, rhs: &LinExpr<V>) -> Self::Output {
        let mut output = rhs.clone();

        for (_key, value) in output.coefs.iter_mut() {
            *value *= ordered_float::OrderedFloat(*self);
        }

        output.constant *= *self;

        output
    }
}

impl<V: UsableData> std::ops::Mul<&LinExpr<V>> for f64 {
    type Output = LinExpr<V>;

    fn mul(self, rhs: &LinExpr<V>) -> Self::Output {
        (&self) * rhs
    }
}

impl<V: UsableData> std::ops::Mul<LinExpr<V>> for &f64 {
    type Output = LinExpr<V>;

    fn mul(self, rhs: LinExpr<V>) -> Self::Output {
        self * &rhs
    }
}

impl<V: UsableData> std::ops::Mul<LinExpr<V>> for f64 {
    type Output = LinExpr<V>;

    fn mul(self, rhs: LinExpr<V>) -> Self::Output {
        &self * &rhs
    }
}

impl<V: UsableData> std::ops::Mul<&LinExpr<V>> for &i32 {
    type Output = LinExpr<V>;

    fn mul(self, rhs: &LinExpr<V>) -> Self::Output {
        f64::from(*self) * rhs
    }
}

impl<V: UsableData> std::ops::Mul<&LinExpr<V>> for i32 {
    type Output = LinExpr<V>;

    fn mul(self, rhs: &LinExpr<V>) -> Self::Output {
        (&self) * rhs
    }
}

impl<V: UsableData> std::ops::Mul<LinExpr<V>> for &i32 {
    type Output = LinExpr<V>;

    fn mul(self, rhs: LinExpr<V>) -> Self::Output {
        self * &rhs
    }
}

impl<V: UsableData> std::ops::Mul<LinExpr<V>> for i32 {
    type Output = LinExpr<V>;

    fn mul(self, rhs: LinExpr<V>) -> Self::Output {
        &self * &rhs
    }
}

impl<V: UsableData> std::ops::Neg for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn neg(self) -> Self::Output {
        (-1.0) * self
    }
}

impl<V: UsableData> std::ops::Neg for LinExpr<V> {
    type Output = LinExpr<V>;

    fn neg(self) -> Self::Output {
        -&self
    }
}

impl<V: UsableData> std::ops::Sub for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &LinExpr<V>) -> Self::Output {
        self + (-1.0) * rhs
    }
}

impl<V: UsableData> std::ops::Sub for LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: LinExpr<V>) -> Self::Output {
        &self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<LinExpr<V>> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: LinExpr<V>) -> Self::Output {
        self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<&LinExpr<V>> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &LinExpr<V>) -> Self::Output {
        &self - rhs
    }
}

impl<V: UsableData> std::ops::Sub<&f64> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &f64) -> Self::Output {
        self + (-*rhs)
    }
}

impl<V: UsableData> std::ops::Sub<&f64> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &f64) -> Self::Output {
        &self - rhs
    }
}

impl<V: UsableData> std::ops::Sub<f64> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: f64) -> Self::Output {
        self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<f64> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: f64) -> Self::Output {
        &self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<&i32> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &i32) -> Self::Output {
        self - f64::from(*rhs)
    }
}

impl<V: UsableData> std::ops::Sub<&i32> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &i32) -> Self::Output {
        &self - rhs
    }
}

impl<V: UsableData> std::ops::Sub<i32> for &LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: i32) -> Self::Output {
        self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<i32> for LinExpr<V> {
    type Output = LinExpr<V>;

    fn sub(self, rhs: i32) -> Self::Output {
        &self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<&LinExpr<V>> for &f64 {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &LinExpr<V>) -> Self::Output {
        -rhs + self
    }
}

impl<V: UsableData> std::ops::Sub<&LinExpr<V>> for f64 {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &LinExpr<V>) -> Self::Output {
        &self - rhs
    }
}

impl<V: UsableData> std::ops::Sub<LinExpr<V>> for &f64 {
    type Output = LinExpr<V>;

    fn sub(self, rhs: LinExpr<V>) -> Self::Output {
        self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<LinExpr<V>> for f64 {
    type Output = LinExpr<V>;

    fn sub(self, rhs: LinExpr<V>) -> Self::Output {
        &self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<&LinExpr<V>> for &i32 {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &LinExpr<V>) -> Self::Output {
        -rhs + self
    }
}

impl<V: UsableData> std::ops::Sub<&LinExpr<V>> for i32 {
    type Output = LinExpr<V>;

    fn sub(self, rhs: &LinExpr<V>) -> Self::Output {
        &self - rhs
    }
}

impl<V: UsableData> std::ops::Sub<LinExpr<V>> for &i32 {
    type Output = LinExpr<V>;

    fn sub(self, rhs: LinExpr<V>) -> Self::Output {
        self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<LinExpr<V>> for i32 {
    type Output = LinExpr<V>;

    fn sub(self, rhs: LinExpr<V>) -> Self::Output {
        &self - &rhs
    }
}
