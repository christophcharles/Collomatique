use std::collections::BTreeMap;

#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct Expr {
    coefs: BTreeMap<String, i32>,
    constant: i32,
}

#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub enum Sign {
    Equals,
    #[default]
    LessThan,
}

#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct Constraint {
    expr: Expr,
    sign: Sign,
}

impl Expr {
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
}

impl std::fmt::Display for Expr {
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

impl std::fmt::Display for Sign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
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

impl From<String> for Expr {
    fn from(name: String) -> Self {
        Expr {
            coefs: BTreeMap::from([
                (name, 1)
            ]),
            constant: 0,
        }
    }
}

impl From<&str> for Expr {
    fn from(name: &str) -> Self {
        Expr {
            coefs: BTreeMap::from([
                (String::from(name), 1)
            ]),
            constant: 0,
        }
    }
}

impl From<i32> for Expr {
    fn from(number: i32) -> Self {
        Expr {
            coefs: BTreeMap::new(),
            constant: number,
        }
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
        self + Expr::from(*rhs)
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
        - &self
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
        self + (- *rhs)
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
