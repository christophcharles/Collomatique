//! This module defines [IntLinExpr] and [IntConstraint].
//! These are wrapper types around [LinExpr] and [Constraint] that enforce
//! integer coefficients (and constant terms).
//!
//! Variable integrality is NOT checked here — that requires knowledge of the
//! variable types, which is checked during reification.

use crate::linexpr::{Constraint, EqSymbol, LinExpr};
use crate::{UsableData, f64_is_zero};
use thiserror::Error;

/// Error returned when a coefficient or constant is not an integer.
#[derive(Debug, Clone, Error)]
#[error("Non-integer value: {0}")]
pub struct NonIntegerError(pub f64);

fn check_integer(v: f64) -> Result<(), NonIntegerError> {
    if f64_is_zero(v - v.round()) {
        Ok(())
    } else {
        Err(NonIntegerError(v))
    }
}

/// A linear expression guaranteed to have integer coefficients and constant.
///
/// Wraps a [`LinExpr<V>`] with a validation invariant: all coefficients and
/// the constant term must be integers (within [`TOLERANCE`](crate::TOLERANCE)).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntLinExpr<V: UsableData>(LinExpr<V>);

impl<V: UsableData> IntLinExpr<V> {
    /// Try to construct from a `LinExpr`. Fails if any coefficient or the
    /// constant is not an integer (within tolerance).
    pub fn try_new(expr: LinExpr<V>) -> Result<Self, NonIntegerError> {
        check_integer(expr.get_constant())?;
        for (_var, coef) in expr.coefficients() {
            check_integer(coef)?;
        }
        Ok(IntLinExpr(expr))
    }

    /// Build an integer expression with a single variable (coefficient 1, constant 0).
    pub fn var<T: Into<V>>(name: T) -> Self {
        IntLinExpr(LinExpr::var(name))
    }

    /// Build a constant integer expression.
    pub fn constant(c: i64) -> Self {
        IntLinExpr(LinExpr::constant(c as f64))
    }

    /// Access the underlying `LinExpr`.
    pub fn as_linexpr(&self) -> &LinExpr<V> {
        &self.0
    }

    /// Consume into the inner `LinExpr`.
    pub fn into_linexpr(self) -> LinExpr<V> {
        self.0
    }

    /// Transmute variables, preserving the integer property.
    pub fn transmute<U: UsableData, F: FnMut(&V) -> U>(&self, f: F) -> IntLinExpr<U> {
        IntLinExpr(self.0.transmute(f))
    }

    /// Transmute variables with a fallible mapping.
    pub fn try_transmute<U: UsableData, F: FnMut(&V) -> Option<U>>(
        &self,
        f: F,
    ) -> Option<IntLinExpr<U>> {
        self.0.try_transmute(f).map(IntLinExpr)
    }

    /// Keep only variables for which the predicate returns `true`.
    pub fn retain(&mut self, f: impl FnMut(&V) -> bool) {
        self.0.retain(f);
    }

    /// Like [`IntLinExpr::retain`] but returns a new expression instead of mutating.
    pub fn retained(&self, f: impl FnMut(&V) -> bool) -> IntLinExpr<V> {
        IntLinExpr(self.0.retained(f))
    }

    /// Build constraint: `self <= rhs`
    pub fn leq(&self, rhs: &IntLinExpr<V>) -> IntConstraint<V> {
        IntConstraint(self.0.leq(&rhs.0))
    }

    /// Build constraint: `self >= rhs`
    pub fn geq(&self, rhs: &IntLinExpr<V>) -> IntConstraint<V> {
        IntConstraint(self.0.geq(&rhs.0))
    }

    /// Build constraint: `self == rhs`
    pub fn eq(&self, rhs: &IntLinExpr<V>) -> IntConstraint<V> {
        IntConstraint(self.0.eq(&rhs.0))
    }
}

impl<V: UsableData> std::ops::Deref for IntLinExpr<V> {
    type Target = LinExpr<V>;
    fn deref(&self) -> &LinExpr<V> {
        &self.0
    }
}

// IntLinExpr + IntLinExpr
impl<V: UsableData> std::ops::Add for &IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn add(self, rhs: Self) -> IntLinExpr<V> {
        IntLinExpr(&self.0 + &rhs.0)
    }
}

impl<V: UsableData> std::ops::Add for IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn add(self, rhs: Self) -> IntLinExpr<V> {
        &self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&IntLinExpr<V>> for IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn add(self, rhs: &IntLinExpr<V>) -> IntLinExpr<V> {
        &self + rhs
    }
}

impl<V: UsableData> std::ops::Add<IntLinExpr<V>> for &IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn add(self, rhs: IntLinExpr<V>) -> IntLinExpr<V> {
        self + &rhs
    }
}

// IntLinExpr + i64
impl<V: UsableData> std::ops::Add<i64> for &IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn add(self, rhs: i64) -> IntLinExpr<V> {
        IntLinExpr(&self.0 + rhs as f64)
    }
}

impl<V: UsableData> std::ops::Add<i64> for IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn add(self, rhs: i64) -> IntLinExpr<V> {
        &self + rhs
    }
}

// IntLinExpr - IntLinExpr
impl<V: UsableData> std::ops::Sub for &IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn sub(self, rhs: Self) -> IntLinExpr<V> {
        IntLinExpr(&self.0 - &rhs.0)
    }
}

impl<V: UsableData> std::ops::Sub for IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn sub(self, rhs: Self) -> IntLinExpr<V> {
        &self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<&IntLinExpr<V>> for IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn sub(self, rhs: &IntLinExpr<V>) -> IntLinExpr<V> {
        &self - rhs
    }
}

impl<V: UsableData> std::ops::Sub<IntLinExpr<V>> for &IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn sub(self, rhs: IntLinExpr<V>) -> IntLinExpr<V> {
        self - &rhs
    }
}

// IntLinExpr - i64
impl<V: UsableData> std::ops::Sub<i64> for &IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn sub(self, rhs: i64) -> IntLinExpr<V> {
        IntLinExpr(&self.0 - rhs as f64)
    }
}

impl<V: UsableData> std::ops::Sub<i64> for IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn sub(self, rhs: i64) -> IntLinExpr<V> {
        &self - rhs
    }
}

// Negation
impl<V: UsableData> std::ops::Neg for &IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn neg(self) -> IntLinExpr<V> {
        IntLinExpr(-&self.0)
    }
}

impl<V: UsableData> std::ops::Neg for IntLinExpr<V> {
    type Output = IntLinExpr<V>;
    fn neg(self) -> IntLinExpr<V> {
        -&self
    }
}

// i64 * IntLinExpr
impl<V: UsableData> std::ops::Mul<&IntLinExpr<V>> for i64 {
    type Output = IntLinExpr<V>;
    fn mul(self, rhs: &IntLinExpr<V>) -> IntLinExpr<V> {
        IntLinExpr((self as f64) * &rhs.0)
    }
}

impl<V: UsableData> std::ops::Mul<IntLinExpr<V>> for i64 {
    type Output = IntLinExpr<V>;
    fn mul(self, rhs: IntLinExpr<V>) -> IntLinExpr<V> {
        self * &rhs
    }
}

// AddAssign
impl<V: UsableData> std::ops::AddAssign<&IntLinExpr<V>> for IntLinExpr<V> {
    fn add_assign(&mut self, rhs: &IntLinExpr<V>) {
        self.0 += &rhs.0;
    }
}

impl<V: UsableData> std::ops::AddAssign<IntLinExpr<V>> for IntLinExpr<V> {
    fn add_assign(&mut self, rhs: IntLinExpr<V>) {
        *self += &rhs;
    }
}

// SubAssign
impl<V: UsableData> std::ops::SubAssign<&IntLinExpr<V>> for IntLinExpr<V> {
    fn sub_assign(&mut self, rhs: &IntLinExpr<V>) {
        self.0 -= &rhs.0;
    }
}

impl<V: UsableData> std::ops::SubAssign<IntLinExpr<V>> for IntLinExpr<V> {
    fn sub_assign(&mut self, rhs: IntLinExpr<V>) {
        *self -= &rhs;
    }
}

// AddAssign/SubAssign for i64
impl<V: UsableData> std::ops::AddAssign<i64> for IntLinExpr<V> {
    fn add_assign(&mut self, rhs: i64) {
        self.0 += rhs as f64;
    }
}

impl<V: UsableData> std::ops::SubAssign<i64> for IntLinExpr<V> {
    fn sub_assign(&mut self, rhs: i64) {
        self.0 -= rhs as f64;
    }
}

// i64 + IntLinExpr
impl<V: UsableData> std::ops::Add<&IntLinExpr<V>> for i64 {
    type Output = IntLinExpr<V>;
    fn add(self, rhs: &IntLinExpr<V>) -> IntLinExpr<V> {
        rhs + self
    }
}

impl<V: UsableData> std::ops::Add<IntLinExpr<V>> for i64 {
    type Output = IntLinExpr<V>;
    fn add(self, rhs: IntLinExpr<V>) -> IntLinExpr<V> {
        &rhs + self
    }
}

// i64 - IntLinExpr
impl<V: UsableData> std::ops::Sub<&IntLinExpr<V>> for i64 {
    type Output = IntLinExpr<V>;
    fn sub(self, rhs: &IntLinExpr<V>) -> IntLinExpr<V> {
        -rhs + self
    }
}

impl<V: UsableData> std::ops::Sub<IntLinExpr<V>> for i64 {
    type Output = IntLinExpr<V>;
    fn sub(self, rhs: IntLinExpr<V>) -> IntLinExpr<V> {
        -&rhs + self
    }
}

// Sum
impl<V: UsableData> std::iter::Sum for IntLinExpr<V> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut acc = IntLinExpr::constant(0);
        for item in iter {
            acc += item;
        }
        acc
    }
}

impl<'a, V: UsableData> std::iter::Sum<&'a IntLinExpr<V>> for IntLinExpr<V> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        let mut acc = IntLinExpr::constant(0);
        for item in iter {
            acc += item;
        }
        acc
    }
}

// i32 * IntLinExpr (for convenience)
impl<V: UsableData> std::ops::Mul<&IntLinExpr<V>> for i32 {
    type Output = IntLinExpr<V>;
    fn mul(self, rhs: &IntLinExpr<V>) -> IntLinExpr<V> {
        (self as i64) * rhs
    }
}

impl<V: UsableData> std::ops::Mul<IntLinExpr<V>> for i32 {
    type Output = IntLinExpr<V>;
    fn mul(self, rhs: IntLinExpr<V>) -> IntLinExpr<V> {
        (self as i64) * &rhs
    }
}

/// A constraint guaranteed to have integer coefficients and constant.
///
/// Wraps a [`Constraint<V>`] with the same integer validation as [`IntLinExpr`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntConstraint<V: UsableData>(Constraint<V>);

impl<V: UsableData> IntConstraint<V> {
    /// Try to construct from a `Constraint`. Fails if any coefficient or the
    /// constant is not an integer (within tolerance).
    pub fn try_new(constraint: Constraint<V>) -> Result<Self, NonIntegerError> {
        check_integer(constraint.get_constant())?;
        for (_var, coef) in constraint.coefficients() {
            check_integer(coef)?;
        }
        Ok(IntConstraint(constraint))
    }

    /// Access the underlying `Constraint`.
    pub fn as_constraint(&self) -> &Constraint<V> {
        &self.0
    }

    /// Consume into the inner `Constraint`.
    pub fn into_constraint(self) -> Constraint<V> {
        self.0
    }

    /// Transmute variables, preserving the integer property.
    pub fn transmute<U: UsableData, F: FnMut(&V) -> U>(&self, f: F) -> IntConstraint<U> {
        IntConstraint(self.0.transmute(f))
    }

    /// Transmute variables with a fallible mapping.
    pub fn try_transmute<U: UsableData, F: FnMut(&V) -> Option<U>>(
        &self,
        f: F,
    ) -> Option<IntConstraint<U>> {
        self.0.try_transmute(f).map(IntConstraint)
    }

    /// Keep only variables for which the predicate returns `true`.
    pub fn retain(&mut self, f: impl FnMut(&V) -> bool) {
        self.0.retain(f);
    }

    /// Like [`IntConstraint::retain`] but returns a new constraint instead of mutating.
    pub fn retained(&self, f: impl FnMut(&V) -> bool) -> IntConstraint<V> {
        IntConstraint(self.0.retained(f))
    }

    /// Get the equality/inequality symbol.
    pub fn get_symbol(&self) -> EqSymbol {
        self.0.get_symbol()
    }

    /// Get the internal left-hand side expression.
    pub fn get_lhs(&self) -> &LinExpr<V> {
        self.0.get_lhs()
    }

    /// Create an always-false constraint (1 <= 0).
    pub fn infeasible() -> Self {
        IntLinExpr::<V>::constant(1).leq(&IntLinExpr::constant(0))
    }
}

impl<V: UsableData> std::ops::Deref for IntConstraint<V> {
    type Target = Constraint<V>;
    fn deref(&self) -> &Constraint<V> {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_linexpr_valid() {
        let expr = 2.0 * LinExpr::<String>::var("A") + LinExpr::constant(3.0);
        assert!(IntLinExpr::try_new(expr).is_ok());
    }

    #[test]
    fn int_linexpr_invalid_coef() {
        let expr = 2.5 * LinExpr::<String>::var("A");
        assert!(IntLinExpr::try_new(expr).is_err());
    }

    #[test]
    fn int_linexpr_invalid_constant() {
        let expr = LinExpr::<String>::constant(1.5);
        assert!(IntLinExpr::try_new(expr).is_err());
    }

    #[test]
    fn int_linexpr_arithmetic() {
        let a = IntLinExpr::<String>::var("A");
        let b = IntLinExpr::<String>::var("B");

        let expr: IntLinExpr<String> = 2 * &a + 3 * &b - IntLinExpr::constant(4);
        assert_eq!(expr.get("A"), Some(2.0));
        assert_eq!(expr.get("B"), Some(3.0));
        assert_eq!(expr.get_constant(), -4.0);
    }

    #[test]
    fn int_linexpr_negation() {
        let a = IntLinExpr::<String>::var("A");
        let neg = -&a;
        assert_eq!(neg.get("A"), Some(-1.0));
    }

    #[test]
    fn int_constraint_valid() {
        let lhs = 2.0 * LinExpr::<String>::var("A") + LinExpr::constant(1.0);
        let rhs = LinExpr::<String>::var("B");
        let constraint = lhs.leq(&rhs);
        assert!(IntConstraint::try_new(constraint).is_ok());
    }

    #[test]
    fn int_constraint_invalid() {
        let lhs = 2.5 * LinExpr::<String>::var("A");
        let rhs = LinExpr::<String>::constant(0.0);
        let constraint = lhs.leq(&rhs);
        assert!(IntConstraint::try_new(constraint).is_err());
    }

    #[test]
    fn int_linexpr_leq() {
        let a = IntLinExpr::<String>::var("A");
        let b = IntLinExpr::<String>::var("B");
        let c = a.leq(&b);
        assert_eq!(c.get_symbol(), EqSymbol::LessThan);
    }

    #[test]
    fn int_linexpr_transmute() {
        let a = IntLinExpr::<String>::var("A");
        let b: IntLinExpr<String> = a.transmute(|v| format!("prefix_{v}"));
        assert_eq!(b.get("prefix_A"), Some(1.0));
    }

    #[test]
    fn int_linexpr_sub_assign() {
        let mut a = IntLinExpr::<String>::var("A");
        let b = IntLinExpr::<String>::var("B");
        a -= &b;
        assert_eq!(a.get("A"), Some(1.0));
        assert_eq!(a.get("B"), Some(-1.0));
    }

    #[test]
    fn int_linexpr_add_assign_i64() {
        let mut a = IntLinExpr::<String>::var("A");
        a += 5i64;
        assert_eq!(a.get("A"), Some(1.0));
        assert_eq!(a.get_constant(), 5.0);
    }

    #[test]
    fn int_linexpr_sub_assign_i64() {
        let mut a = IntLinExpr::<String>::var("A");
        a -= 3i64;
        assert_eq!(a.get("A"), Some(1.0));
        assert_eq!(a.get_constant(), -3.0);
    }

    #[test]
    fn int_linexpr_reverse_add_i64() {
        let a = IntLinExpr::<String>::var("A");
        let expr = 5i64 + &a;
        assert_eq!(expr.get("A"), Some(1.0));
        assert_eq!(expr.get_constant(), 5.0);
    }

    #[test]
    fn int_linexpr_reverse_sub_i64() {
        let a = IntLinExpr::<String>::var("A");
        let expr = 5i64 - &a;
        assert_eq!(expr.get("A"), Some(-1.0));
        assert_eq!(expr.get_constant(), 5.0);
    }

    #[test]
    fn int_linexpr_sum_iterator() {
        let exprs = vec![
            IntLinExpr::<String>::var("A"),
            IntLinExpr::<String>::var("B"),
            IntLinExpr::constant(3),
        ];
        let sum: IntLinExpr<String> = exprs.into_iter().sum();
        assert_eq!(sum.get("A"), Some(1.0));
        assert_eq!(sum.get("B"), Some(1.0));
        assert_eq!(sum.get_constant(), 3.0);
    }

    #[test]
    fn int_linexpr_sum_empty_iterator() {
        let sum: IntLinExpr<String> = Vec::<IntLinExpr<String>>::new().into_iter().sum();
        assert!(sum.variables().is_empty());
        assert_eq!(sum.get_constant(), 0.0);
    }
}
