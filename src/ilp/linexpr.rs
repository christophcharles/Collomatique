#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

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
    coefs: BTreeMap<V, i32>,
    constant: i32,
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

use std::collections::BTreeSet;

impl<V: VariableName> Expr<V> {
    pub fn var<T: Into<V>>(name: T) -> Self {
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

impl<V: VariableName> Expr<V> {
    pub fn coefs(&self) -> &BTreeMap<V, i32> {
        &self.coefs
    }

    pub fn variables(&self) -> BTreeSet<V> {
        self.coefs.keys().cloned().collect()
    }

    pub fn get<T: Into<V>>(&self, var: T) -> Option<i32> {
        self.coefs.get(&var.into()).cloned()
    }

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

    pub fn clean(&mut self) {
        self.coefs.retain(|_k, v| *v != 0);
    }

    pub fn cleaned(&self) -> Expr<V> {
        let mut output = self.clone();
        output.clean();
        output
    }

    pub fn reduce(&mut self, vars: &BTreeMap<V, bool>) {
        *self = self.reduced(vars);
    }

    pub fn reduced(&self, vars: &BTreeMap<V, bool>) -> Expr<V> {
        let mut constant = self.constant;
        let mut coefs = BTreeMap::new();

        for (v, c) in &self.coefs {
            match vars.get(v) {
                Some(val) => {
                    constant += if *val { *c } else { 0 };
                }
                None => {
                    coefs.insert(v.clone(), *c);
                }
            }
        }

        Expr { coefs, constant }
    }

    pub fn eval(&self, vars: &BTreeMap<V, bool>) -> Option<i32> {
        let mut tot = self.constant;

        for (v, c) in &self.coefs {
            if *c == 0 {
                continue;
            }

            match vars.get(v) {
                Some(val) => {
                    if *val {
                        tot += *c;
                    }
                }
                None => {
                    return None;
                }
            }
        }

        Some(tot)
    }
}

impl<V: VariableName> Constraint<V> {
    pub fn variables(&self) -> BTreeSet<V> {
        self.expr.variables()
    }

    pub fn coefs(&self) -> &BTreeMap<V, i32> {
        self.expr.coefs()
    }

    pub fn get_var<T: Into<V>>(&self, var: T) -> Option<i32> {
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

    pub fn cleaned(&self) -> Constraint<V> {
        let mut output = self.clone();
        output.clean();
        output
    }

    pub fn reduce(&mut self, vars: &BTreeMap<V, bool>) {
        self.expr.reduce(vars);
    }

    pub fn reduced(&self, vars: &BTreeMap<V, bool>) -> Constraint<V> {
        let mut output = self.clone();
        output.reduce(vars);
        output
    }

    pub fn eval(&self, vars: &BTreeMap<V, bool>) -> Option<bool> {
        let val = self.expr.eval(vars)?;

        match self.sign {
            Sign::Equals => Some(val == 0),
            Sign::LessThan => Some(val <= 0),
        }
    }

    pub fn simple_solve(&self) -> SimpleSolution<V> {
        let mut non_zero_coef = None;
        let mut non_zero_coefs_count = 0;

        for (v, c) in &self.expr.coefs {
            if *c != 0 {
                non_zero_coefs_count += 1;
                if non_zero_coefs_count == 1 {
                    non_zero_coef = Some(v.clone());
                }
            }
        }

        match non_zero_coefs_count {
            0 => {
                let val = self
                    .eval(&BTreeMap::new())
                    .expect("constraint should evaluate since it has no non-zero coefs");

                if val {
                    SimpleSolution::Solution(None)
                } else {
                    SimpleSolution::NoSolution
                }
            }
            1 => {
                let v = non_zero_coef.expect("There should be a non-zero coefficient");

                let false_is_solution = self
                    .eval(&BTreeMap::from([(v.clone(), false)]))
                    .expect("constraint should evaluate with v given");
                let true_is_solution = self
                    .eval(&BTreeMap::from([(v.clone(), true)]))
                    .expect("constraint should evaluate with v given");

                if true_is_solution {
                    if false_is_solution {
                        SimpleSolution::NotSimpleSolvable
                    } else {
                        SimpleSolution::Solution(Some((v.clone(), true)))
                    }
                } else if false_is_solution {
                    SimpleSolution::Solution(Some((v.clone(), false)))
                } else {
                    SimpleSolution::NoSolution
                }
            }
            _ => SimpleSolution::NotSimpleSolvable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleSolution<V: VariableName> {
    NotSimpleSolvable,
    NoSolution,
    Solution(Option<(V, bool)>),
}

impl<V: VariableName> std::fmt::Display for Expr<V> {
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

impl<V: VariableName> std::ops::Add<&i32> for &Expr<V> {
    type Output = Expr<V>;

    fn add(self, rhs: &i32) -> Self::Output {
        self + Expr::constant(*rhs)
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

impl<V: VariableName> std::ops::Mul<&Expr<V>> for &i32 {
    type Output = Expr<V>;

    fn mul(self, rhs: &Expr<V>) -> Self::Output {
        let mut output = rhs.clone();

        for (_key, value) in output.coefs.iter_mut() {
            *value *= *self;
        }

        output.constant *= *self;

        output
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
        (-1) * self
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
        self + (-1) * rhs
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

impl<V: VariableName> std::ops::Sub<&i32> for &Expr<V> {
    type Output = Expr<V>;

    fn sub(self, rhs: &i32) -> Self::Output {
        self + (-*rhs)
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
