use std::collections::BTreeMap;

#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct LinExpr {
    coefs: BTreeMap<String, i32>,
    constant: i32,
}

#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub enum ConstraintSign {
    Equals,
    #[default]
    LessThan,
}

#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct LinConstraint {
    expr: LinExpr,
    sign: ConstraintSign,
}

impl LinExpr {
    pub fn leq(&self, rhs: &LinExpr) -> LinConstraint {
        LinConstraint {
            expr: self - rhs,
            sign: ConstraintSign::LessThan,
        }
    }

    pub fn geq(&self, rhs: &LinExpr) -> LinConstraint {
        LinConstraint {
            expr: rhs - self,
            sign: ConstraintSign::LessThan,
        }
    }

    pub fn eq(&self, rhs: &LinExpr) -> LinConstraint {
        LinConstraint {
            expr: self - rhs,
            sign: ConstraintSign::Equals,
        }
    }
}

impl std::fmt::Display for LinExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.coefs.is_empty() && self.constant == 0 {
            write!(f, "0")?;
            return Ok(());
        }

        let mut it = self.coefs.iter().peekable();
        while let Some((key,value)) = it.next() {
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

impl std::fmt::Display for ConstraintSign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "{}",
            match self {
                ConstraintSign::Equals => "=",
                ConstraintSign::LessThan => "<=",
            }
        )
    }
}

impl std::fmt::Display for LinConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} 0", self.expr, self.sign)
    }
}

impl From<String> for LinExpr {
    fn from(name: String) -> Self {
        LinExpr {
            coefs: BTreeMap::from([
                (name, 1)
            ]),
            constant: 0,
        }
    }
}

impl From<&str> for LinExpr {
    fn from(name: &str) -> Self {
        LinExpr {
            coefs: BTreeMap::from([
                (String::from(name), 1)
            ]),
            constant: 0,
        }
    }
}

impl From<i32> for LinExpr {
    fn from(number: i32) -> Self {
        LinExpr {
            coefs: BTreeMap::new(),
            constant: number,
        }
    }
}

impl std::ops::Add for &LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: &LinExpr) -> Self::Output {
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

impl std::ops::Add for LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: LinExpr) -> Self::Output {
        &self + &rhs
    }
}

impl std::ops::Add<LinExpr> for &LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: LinExpr) -> Self::Output {
        self + &rhs
    }
}

impl std::ops::Add<&LinExpr> for LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: &LinExpr) -> Self::Output {
        &self + rhs
    }
}

impl std::ops::Add<&i32> for &LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: &i32) -> Self::Output {
        self + LinExpr::from(*rhs)
    }
}

impl std::ops::Add<i32> for &LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: i32) -> Self::Output {
        self + &rhs
    }
}

impl std::ops::Add<&i32> for LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: &i32) -> Self::Output {
        &self + rhs
    }
}

impl std::ops::Add<i32> for LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: i32) -> Self::Output {
        &self + &rhs
    }
}

impl std::ops::Add<&LinExpr> for &i32 {
    type Output = LinExpr;

    fn add(self, rhs: &LinExpr) -> Self::Output {
        rhs + self
    }
}

impl std::ops::Add<LinExpr> for &i32 {
    type Output = LinExpr;

    fn add(self, rhs: LinExpr) -> Self::Output {
        self + &rhs
    }
}

impl std::ops::Add<&LinExpr> for i32 {
    type Output = LinExpr;

    fn add(self, rhs: &LinExpr) -> Self::Output {
        &self + rhs
    }
}

impl std::ops::Add<LinExpr> for i32 {
    type Output = LinExpr;

    fn add(self, rhs: LinExpr) -> Self::Output {
        &self + &rhs
    }
}


impl std::ops::Mul<&LinExpr> for &i32 {
    type Output = LinExpr;

    fn mul(self, rhs: &LinExpr) -> Self::Output {
        let mut output = rhs.clone();

        for (_key, value) in output.coefs.iter_mut() {
            *value *= *self;
        }

        output.constant *= *self;

        output
    }
}

impl std::ops::Mul<&LinExpr> for i32 {
    type Output = LinExpr;

    fn mul(self, rhs: &LinExpr) -> Self::Output {
        (&self) * rhs
    }
}

impl std::ops::Mul<LinExpr> for &i32 {
    type Output = LinExpr;

    fn mul(self, rhs: LinExpr) -> Self::Output {
        self * &rhs
    }
}

impl std::ops::Mul<LinExpr> for i32 {
    type Output = LinExpr;

    fn mul(self, rhs: LinExpr) -> Self::Output {
        &self * &rhs
    }
}

impl std::ops::Neg for &LinExpr {
    type Output = LinExpr;

    fn neg(self) -> Self::Output {
        (-1) * self
    }
}

impl std::ops::Neg for LinExpr {
    type Output = LinExpr;

    fn neg(self) -> Self::Output {
        - &self
    }
}

impl std::ops::Sub for &LinExpr {
    type Output = LinExpr;

    fn sub(self, rhs: &LinExpr) -> Self::Output {
        self + (-1) * rhs
    }
}

impl std::ops::Sub for LinExpr {
    type Output = LinExpr;

    fn sub(self, rhs: LinExpr) -> Self::Output {
        &self - &rhs
    }
}

impl std::ops::Sub<LinExpr> for &LinExpr {
    type Output = LinExpr;

    fn sub(self, rhs: LinExpr) -> Self::Output {
        self - &rhs
    }
}

impl std::ops::Sub<&LinExpr> for LinExpr {
    type Output = LinExpr;

    fn sub(self, rhs: &LinExpr) -> Self::Output {
        &self - rhs
    }
}

impl std::ops::Sub<&i32> for &LinExpr {
    type Output = LinExpr;

    fn sub(self, rhs: &i32) -> Self::Output {
        self + (- *rhs)
    }
}

impl std::ops::Sub<&i32> for LinExpr {
    type Output = LinExpr;

    fn sub(self, rhs: &i32) -> Self::Output {
        &self - rhs
    }
}

impl std::ops::Sub<i32> for &LinExpr {
    type Output = LinExpr;

    fn sub(self, rhs: i32) -> Self::Output {
        self - &rhs
    }
}

impl std::ops::Sub<i32> for LinExpr {
    type Output = LinExpr;

    fn sub(self, rhs: i32) -> Self::Output {
        &self - &rhs
    }
}

impl std::ops::Sub<&LinExpr> for &i32 {
    type Output = LinExpr;

    fn sub(self, rhs: &LinExpr) -> Self::Output {
        -rhs + self
    }
}

impl std::ops::Sub<&LinExpr> for i32 {
    type Output = LinExpr;

    fn sub(self, rhs: &LinExpr) -> Self::Output {
        &self - rhs
    }
}

impl std::ops::Sub<LinExpr> for &i32 {
    type Output = LinExpr;

    fn sub(self, rhs: LinExpr) -> Self::Output {
        self - &rhs
    }
}

impl std::ops::Sub<LinExpr> for i32 {
    type Output = LinExpr;

    fn sub(self, rhs: LinExpr) -> Self::Output {
        &self - &rhs
    }
}
