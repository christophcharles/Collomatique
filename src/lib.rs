use std::collections::BTreeMap;

#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct LinExpr {
    coefs: BTreeMap<String, i32>,
}

impl std::fmt::Display for LinExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.coefs.is_empty() {
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
            
            if it.peek().is_some() {
                write!(f, " + ")?;
            }
        }
        Ok(())
    }
}

impl LinExpr {
    pub fn from_var<T: Into<String>>(name: T) -> LinExpr {
        return LinExpr {
            coefs: BTreeMap::from([
                (name.into(), 1)
            ]),
        }
    }
}

impl std::ops::Add for &LinExpr {
    type Output = LinExpr;

    fn add(self, rhs: &LinExpr) -> Self::Output {
        let mut output = LinExpr {
            coefs: self.coefs.clone(),
        };

        for (key, value) in rhs.coefs.iter() {
            if let Some(coef) = output.coefs.get_mut(key) {
                *coef += value;
            } else {
                output.coefs.insert(key.clone(), *value);
            }
        }

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

impl std::ops::Mul<&LinExpr> for &i32 {
    type Output = LinExpr;

    fn mul(self, rhs: &LinExpr) -> Self::Output {
        let mut output = rhs.clone();

        for (_key, value) in output.coefs.iter_mut() {
            *value *= *self;
        }

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
