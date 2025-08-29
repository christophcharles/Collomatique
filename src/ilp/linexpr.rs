#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Expr {
    coefs: BTreeMap<String, i32>,
    constant: i32,
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.constant == other.constant && (self.cleaned().coefs == other.cleaned().coefs)
    }
}

impl Eq for Expr {}

pub struct Config {
    values: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sign {
    Equals,
    #[default]
    LessThan,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Constraint {
    expr: Expr,
    sign: Sign,
}

use std::collections::BTreeSet;

impl Expr {
    pub fn var<T: Into<String>>(name: T) -> Self {
        Expr {
            coefs: BTreeMap::from([(name.into(), 1)]),
            constant: 0,
        }
    }

    pub fn constant(number: i32) -> Self {
        Expr {
            coefs: BTreeMap::new(),
            constant: number,
        }
    }
}

impl Expr {
    pub fn variables(&self) -> BTreeSet<String> {
        self.coefs.keys().cloned().collect()
    }

    pub fn get(&self, var: &str) -> Option<i32> {
        self.coefs.get(var).cloned()
    }

    pub fn leq(&self, rhs: &Expr) -> Constraint {
        Constraint {
            expr: self - rhs,
            sign: Sign::LessThan,
        }
    }

    pub fn geq(&self, rhs: &Expr) -> Constraint {
        Constraint {
            expr: rhs - self,
            sign: Sign::LessThan,
        }
    }

    pub fn eq(&self, rhs: &Expr) -> Constraint {
        Constraint {
            expr: self - rhs,
            sign: Sign::Equals,
        }
    }

    pub fn clean(&mut self) {
        self.coefs.retain(|_k, v| *v != 0);
    }

    pub fn cleaned(&self) -> Expr {
        let mut output = self.clone();
        output.clean();
        output
    }

    pub fn reduce(&self, config: &Config) -> Expr {
        let mut constant = self.constant;
        let mut coefs = BTreeMap::new();
        for (key, coef) in self.coefs.iter() {
            match config.values.get(key) {
                Some(val) => {
                    if *val {
                        constant += *coef;
                    }
                }
                None => {
                    coefs.insert(key.clone(), *coef);
                }
            }
        }
        Expr { coefs, constant }
    }

    pub fn to_value(&self) -> Option<i32> {
        if self.cleaned().coefs.is_empty() {
            Some(self.constant)
        } else {
            None
        }
    }

    pub fn eval(&self, config: &Config) -> Option<i32> {
        self.reduce(config).to_value()
    }
}

impl Constraint {
    pub fn variables(&self) -> BTreeSet<String> {
        self.expr.variables()
    }

    pub fn get_var(&self, var: &str) -> Option<i32> {
        self.expr.get(var)
    }

    pub fn get_sign(&self) -> Sign {
        self.sign
    }

    pub fn get_constant(&self) -> i32 {
        self.expr.constant
    }

    pub fn clean(&mut self) {
        self.expr.clean();
    }

    pub fn cleaned(&self) -> Constraint {
        let mut output = self.clone();
        output.clean();
        output
    }

    pub fn reduce(&self, config: &Config) -> Constraint {
        Constraint {
            expr: self.expr.reduce(config),
            sign: self.sign,
        }
    }

    pub fn to_bool(&self) -> Option<bool> {
        let val = self.expr.to_value()?;

        Some(match self.sign {
            Sign::Equals => val == 0,
            Sign::LessThan => val <= 0,
        })
    }

    pub fn eval(&self, config: &Config) -> Option<bool> {
        self.reduce(config).to_bool()
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.coefs.is_empty() && self.constant == 0 {
            write!(f, "0")?;
            return Ok(());
        }

        let mut it = self.coefs.iter().peekable();
        while let Some((key, value)) = it.next() {
            if *value < 0 {
                write!(f, "({})*{}", value, key)?;
            } else {
                write!(f, "{}*{}", value, key)?;
            }

            if it.peek().is_some() || self.constant != 0 {
                write!(f, " + ")?;
            }
        }

        if self.constant != 0 || self.coefs.is_empty() {
            if self.constant < 0 {
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

impl std::fmt::Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} 0", self.expr, self.sign)
    }
}

impl std::ops::Add for &Expr {
    type Output = Expr;

    fn add(self, rhs: &Expr) -> Self::Output {
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

impl std::ops::Add for Expr {
    type Output = Expr;

    fn add(self, rhs: Expr) -> Self::Output {
        &self + &rhs
    }
}

impl std::ops::Add<Expr> for &Expr {
    type Output = Expr;

    fn add(self, rhs: Expr) -> Self::Output {
        self + &rhs
    }
}

impl std::ops::Add<&Expr> for Expr {
    type Output = Expr;

    fn add(self, rhs: &Expr) -> Self::Output {
        &self + rhs
    }
}

impl std::ops::Add<&i32> for &Expr {
    type Output = Expr;

    fn add(self, rhs: &i32) -> Self::Output {
        self + Expr::constant(*rhs)
    }
}

impl std::ops::Add<i32> for &Expr {
    type Output = Expr;

    fn add(self, rhs: i32) -> Self::Output {
        self + &rhs
    }
}

impl std::ops::Add<&i32> for Expr {
    type Output = Expr;

    fn add(self, rhs: &i32) -> Self::Output {
        &self + rhs
    }
}

impl std::ops::Add<i32> for Expr {
    type Output = Expr;

    fn add(self, rhs: i32) -> Self::Output {
        &self + &rhs
    }
}

impl std::ops::Add<&Expr> for &i32 {
    type Output = Expr;

    fn add(self, rhs: &Expr) -> Self::Output {
        rhs + self
    }
}

impl std::ops::Add<Expr> for &i32 {
    type Output = Expr;

    fn add(self, rhs: Expr) -> Self::Output {
        self + &rhs
    }
}

impl std::ops::Add<&Expr> for i32 {
    type Output = Expr;

    fn add(self, rhs: &Expr) -> Self::Output {
        &self + rhs
    }
}

impl std::ops::Add<Expr> for i32 {
    type Output = Expr;

    fn add(self, rhs: Expr) -> Self::Output {
        &self + &rhs
    }
}

impl std::ops::Mul<&Expr> for &i32 {
    type Output = Expr;

    fn mul(self, rhs: &Expr) -> Self::Output {
        let mut output = rhs.clone();

        for (_key, value) in output.coefs.iter_mut() {
            *value *= *self;
        }

        output.constant *= *self;

        output
    }
}

impl std::ops::Mul<&Expr> for i32 {
    type Output = Expr;

    fn mul(self, rhs: &Expr) -> Self::Output {
        (&self) * rhs
    }
}

impl std::ops::Mul<Expr> for &i32 {
    type Output = Expr;

    fn mul(self, rhs: Expr) -> Self::Output {
        self * &rhs
    }
}

impl std::ops::Mul<Expr> for i32 {
    type Output = Expr;

    fn mul(self, rhs: Expr) -> Self::Output {
        &self * &rhs
    }
}

impl std::ops::Neg for &Expr {
    type Output = Expr;

    fn neg(self) -> Self::Output {
        (-1) * self
    }
}

impl std::ops::Neg for Expr {
    type Output = Expr;

    fn neg(self) -> Self::Output {
        -&self
    }
}

impl std::ops::Sub for &Expr {
    type Output = Expr;

    fn sub(self, rhs: &Expr) -> Self::Output {
        self + (-1) * rhs
    }
}

impl std::ops::Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Expr) -> Self::Output {
        &self - &rhs
    }
}

impl std::ops::Sub<Expr> for &Expr {
    type Output = Expr;

    fn sub(self, rhs: Expr) -> Self::Output {
        self - &rhs
    }
}

impl std::ops::Sub<&Expr> for Expr {
    type Output = Expr;

    fn sub(self, rhs: &Expr) -> Self::Output {
        &self - rhs
    }
}

impl std::ops::Sub<&i32> for &Expr {
    type Output = Expr;

    fn sub(self, rhs: &i32) -> Self::Output {
        self + (-*rhs)
    }
}

impl std::ops::Sub<&i32> for Expr {
    type Output = Expr;

    fn sub(self, rhs: &i32) -> Self::Output {
        &self - rhs
    }
}

impl std::ops::Sub<i32> for &Expr {
    type Output = Expr;

    fn sub(self, rhs: i32) -> Self::Output {
        self - &rhs
    }
}

impl std::ops::Sub<i32> for Expr {
    type Output = Expr;

    fn sub(self, rhs: i32) -> Self::Output {
        &self - &rhs
    }
}

impl std::ops::Sub<&Expr> for &i32 {
    type Output = Expr;

    fn sub(self, rhs: &Expr) -> Self::Output {
        -rhs + self
    }
}

impl std::ops::Sub<&Expr> for i32 {
    type Output = Expr;

    fn sub(self, rhs: &Expr) -> Self::Output {
        &self - rhs
    }
}

impl std::ops::Sub<Expr> for &i32 {
    type Output = Expr;

    fn sub(self, rhs: Expr) -> Self::Output {
        self - &rhs
    }
}

impl std::ops::Sub<Expr> for i32 {
    type Output = Expr;

    fn sub(self, rhs: Expr) -> Self::Output {
        &self - &rhs
    }
}
