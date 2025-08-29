#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

pub trait VariableName:
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
    > VariableName for T
{
}

#[derive(Debug, Clone, Default, PartialOrd, Ord, PartialEq, Eq)]
pub struct Expr<V: VariableName> {
    coefs: BTreeMap<V, ordered_float::OrderedFloat<f64>>,
    constant: ordered_float::OrderedFloat<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Sign {
    Equals,
    #[default]
    LessThan,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Constraint<V: VariableName> {
    sign: Sign,
    expr: Expr<V>,
}

impl<V: VariableName> Expr<V> {
    pub fn var<T: Into<V>>(name: T) -> Self {
        Expr {
            coefs: BTreeMap::from([(name.into(), ordered_float::OrderedFloat(1.0))]),
            constant: ordered_float::OrderedFloat(0.0),
        }
    }

    pub fn constant(number: f64) -> Self {
        Expr {
            coefs: BTreeMap::new(),
            constant: ordered_float::OrderedFloat(number),
        }
    }
}

impl<V: VariableName> Expr<V> {
    pub fn get_constant(&self) -> f64 {
        self.constant.into_inner()
    }

    pub fn get<T: Into<V>>(&self, var: T) -> Option<f64> {
        self.coefs.get(&var.into()).map(|&x| x.into_inner())
    }

    pub fn variables(&self) -> BTreeSet<V> {
        self.coefs.keys().cloned().collect()
    }

    pub fn clean(&mut self) {
        self.coefs.retain(|_k, v| *v != ordered_float::OrderedFloat(0.0));
    }

    pub fn cleaned(&self) -> Expr<V> {
        let mut output = self.clone();
        output.clean();
        output
    }
}

impl<V: VariableName> Expr<V> {
    pub fn leq(&self, rhs: &Expr<V>) -> Constraint<V> {
        Constraint {
            expr: self - rhs,
            sign: Sign::LessThan,
        }
    }

    pub fn geq(&self, rhs: &Expr<V>) -> Constraint<V> {
        Constraint {
            expr: rhs - self,
            sign: Sign::LessThan,
        }
    }

    pub fn eq(&self, rhs: &Expr<V>) -> Constraint<V> {
        Constraint {
            expr: self - rhs,
            sign: Sign::Equals,
        }
    }
}

impl<V: VariableName> Constraint<V> {
    pub fn variables(&self) -> BTreeSet<V> {
        self.expr.variables()
    }

    pub fn get_var<T: Into<V>>(&self, var: T) -> Option<f64> {
        self.expr.get(var)
    }

    pub fn get_sign(&self) -> Sign {
        self.sign
    }

    pub fn get_constant(&self) -> f64 {
        self.expr.get_constant()
    }

    pub fn get_lhs(&self) -> &Expr<V> {
        &self.expr
    }

    pub fn clean(&mut self) {
        self.expr.clean();
    }

    pub fn cleaned(&self) -> Constraint<V> {
        let mut output = self.clone();
        output.clean();
        output
    }
}

impl<V: VariableName> std::fmt::Display for Expr<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.coefs.is_empty() && self.constant.into_inner() == 0.0 {
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

            if it.peek().is_some() || self.constant.0 != 0.0 {
                write!(f, " + ")?;
            }
        }

        if self.constant.into_inner() != 0.0 || self.coefs.is_empty() {
            if self.constant.is_sign_negative() {
                write!(f, "({})", self.constant)?
            } else {
                write!(f, "{}", self.constant)?
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Sign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Sign::Equals => "=",
                Sign::LessThan => "<=",
            }
        )
    }
}

impl<V: VariableName> std::fmt::Display for Constraint<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} 0", self.expr, self.sign)
    }
}

impl<V: VariableName> std::ops::Add for &Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: &Expr<V>) -> Self::Output {
        let mut output = Expr {
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

impl<V: VariableName> std::ops::Add for Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: Expr<V>) -> Self::Output {
        &self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<Expr<V>> for &Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: Expr<V>) -> Self::Output {
        self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<&Expr<V>> for Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: &Expr<V>) -> Self::Output {
        &self + rhs
    }
}

impl<V: VariableName> std::ops::Add<&f64> for &Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: &f64) -> Self::Output {
        self + Expr::constant(*rhs)
    }
}

impl<V: VariableName> std::ops::Add<f64> for &Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: f64) -> Self::Output {
        self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<&f64> for Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: &f64) -> Self::Output {
        &self + rhs
    }
}

impl<V: VariableName> std::ops::Add<f64> for Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: f64) -> Self::Output {
        &self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<&i32> for &Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: &i32) -> Self::Output {
        self + f64::from(*rhs)
    }
}

impl<V: VariableName> std::ops::Add<i32> for &Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: i32) -> Self::Output {
        self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<&i32> for Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: &i32) -> Self::Output {
        &self + rhs
    }
}

impl<V: VariableName> std::ops::Add<i32> for Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: i32) -> Self::Output {
        &self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<&Expr<V>> for &f64 {
    type Output = Expr<V>;

    fn add(self, rhs: &Expr<V>) -> Self::Output {
        rhs + self
    }
}

impl<V: VariableName> std::ops::Add<Expr<V>> for &f64 {
    type Output = Expr<V>;

    fn add(self, rhs: Expr<V>) -> Self::Output {
        self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<&Expr<V>> for f64 {
    type Output = Expr<V>;

    fn add(self, rhs: &Expr<V>) -> Self::Output {
        &self + rhs
    }
}

impl<V: VariableName> std::ops::Add<Expr<V>> for f64 {
    type Output = Expr<V>;

    fn add(self, rhs: Expr<V>) -> Self::Output {
        &self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<&Expr<V>> for &i32 {
    type Output = Expr<V>;

    fn add(self, rhs: &Expr<V>) -> Self::Output {
        rhs + self
    }
}

impl<V: VariableName> std::ops::Add<Expr<V>> for &i32 {
    type Output = Expr<V>;

    fn add(self, rhs: Expr<V>) -> Self::Output {
        self + &rhs
    }
}

impl<V: VariableName> std::ops::Add<&Expr<V>> for i32 {
    type Output = Expr<V>;

    fn add(self, rhs: &Expr<V>) -> Self::Output {
        &self + rhs
    }
}

impl<V: VariableName> std::ops::Add<Expr<V>> for i32 {
    type Output = Expr<V>;

    fn add(self, rhs: Expr<V>) -> Self::Output {
        &self + &rhs
    }
}

impl<V: VariableName> std::ops::Mul<&Expr<V>> for &f64 {
    type Output = Expr<V>;

    fn mul(self, rhs: &Expr<V>) -> Self::Output {
        let mut output = rhs.clone();

        for (_key, value) in output.coefs.iter_mut() {
            *value *= ordered_float::OrderedFloat(*self);
        }

        output.constant *= *self;

        output
    }
}

impl<V: VariableName> std::ops::Mul<&Expr<V>> for f64 {
    type Output = Expr<V>;

    fn mul(self, rhs: &Expr<V>) -> Self::Output {
        (&self) * rhs
    }
}

impl<V: VariableName> std::ops::Mul<Expr<V>> for &f64 {
    type Output = Expr<V>;

    fn mul(self, rhs: Expr<V>) -> Self::Output {
        self * &rhs
    }
}

impl<V: VariableName> std::ops::Mul<Expr<V>> for f64 {
    type Output = Expr<V>;

    fn mul(self, rhs: Expr<V>) -> Self::Output {
        &self * &rhs
    }
}

impl<V: VariableName> std::ops::Mul<&Expr<V>> for &i32 {
    type Output = Expr<V>;

    fn mul(self, rhs: &Expr<V>) -> Self::Output {
        f64::from(*self) * rhs
    }
}

impl<V: VariableName> std::ops::Mul<&Expr<V>> for i32 {
    type Output = Expr<V>;

    fn mul(self, rhs: &Expr<V>) -> Self::Output {
        (&self) * rhs
    }
}

impl<V: VariableName> std::ops::Mul<Expr<V>> for &i32 {
    type Output = Expr<V>;

    fn mul(self, rhs: Expr<V>) -> Self::Output {
        self * &rhs
    }
}

impl<V: VariableName> std::ops::Mul<Expr<V>> for i32 {
    type Output = Expr<V>;

    fn mul(self, rhs: Expr<V>) -> Self::Output {
        &self * &rhs
    }
}

impl<V: VariableName> std::ops::Neg for &Expr<V> {
    type Output = Expr<V>;

    fn neg(self) -> Self::Output {
        (-1.0) * self
    }
}

impl<V: VariableName> std::ops::Neg for Expr<V> {
    type Output = Expr<V>;

    fn neg(self) -> Self::Output {
        -&self
    }
}

impl<V: VariableName> std::ops::Sub for &Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: &Expr<V>) -> Self::Output {
        self + (-1.0) * rhs
    }
}

impl<V: VariableName> std::ops::Sub for Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: Expr<V>) -> Self::Output {
        &self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<Expr<V>> for &Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: Expr<V>) -> Self::Output {
        self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<&Expr<V>> for Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: &Expr<V>) -> Self::Output {
        &self - rhs
    }
}

impl<V: VariableName> std::ops::Sub<&f64> for &Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: &f64) -> Self::Output {
        self + (-*rhs)
    }
}

impl<V: VariableName> std::ops::Sub<&f64> for Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: &f64) -> Self::Output {
        &self - rhs
    }
}

impl<V: VariableName> std::ops::Sub<f64> for &Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: f64) -> Self::Output {
        self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<f64> for Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: f64) -> Self::Output {
        &self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<&i32> for &Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: &i32) -> Self::Output {
        self - f64::from(*rhs)
    }
}

impl<V: VariableName> std::ops::Sub<&i32> for Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: &i32) -> Self::Output {
        &self - rhs
    }
}

impl<V: VariableName> std::ops::Sub<i32> for &Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: i32) -> Self::Output {
        self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<i32> for Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: i32) -> Self::Output {
        &self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<&Expr<V>> for &f64 {
    type Output = Expr<V>;

    fn sub(self, rhs: &Expr<V>) -> Self::Output {
        -rhs + self
    }
}

impl<V: VariableName> std::ops::Sub<&Expr<V>> for f64 {
    type Output = Expr<V>;

    fn sub(self, rhs: &Expr<V>) -> Self::Output {
        &self - rhs
    }
}

impl<V: VariableName> std::ops::Sub<Expr<V>> for &f64 {
    type Output = Expr<V>;

    fn sub(self, rhs: Expr<V>) -> Self::Output {
        self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<Expr<V>> for f64 {
    type Output = Expr<V>;

    fn sub(self, rhs: Expr<V>) -> Self::Output {
        &self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<&Expr<V>> for &i32 {
    type Output = Expr<V>;

    fn sub(self, rhs: &Expr<V>) -> Self::Output {
        -rhs + self
    }
}

impl<V: VariableName> std::ops::Sub<&Expr<V>> for i32 {
    type Output = Expr<V>;

    fn sub(self, rhs: &Expr<V>) -> Self::Output {
        &self - rhs
    }
}

impl<V: VariableName> std::ops::Sub<Expr<V>> for &i32 {
    type Output = Expr<V>;

    fn sub(self, rhs: Expr<V>) -> Self::Output {
        self - &rhs
    }
}

impl<V: VariableName> std::ops::Sub<Expr<V>> for i32 {
    type Output = Expr<V>;

    fn sub(self, rhs: Expr<V>) -> Self::Output {
        &self - &rhs
    }
}
