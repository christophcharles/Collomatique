
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct SqMat {
    n: usize,
    coefs: Vec<i32>,
}

impl std::fmt::Display for SqMat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut widths = vec![0;self.n];

        for j in 0..self.n {
            let mut max = 0;
            for i in 0..self.n {
                let txt = format!("{}", self.coefs[i*self.n + j]);
                let len = txt.len() + 1;
                if len > max {
                    max = len;
                }
            }
            widths[j] = max;
        }

        for i in 0..self.n {
            write!(f, "[")?;
            for j in 0..self.n {
                write!(f, "{:>width$}", self.coefs[i*self.n + j], width=widths[j])?;
            }
            write!(f, " ]")?;
            if i != self.n-1 {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}

impl SqMat {
    pub fn new(n: usize) -> SqMat {
        SqMat {
            n,
            coefs: vec![0;n*n],
        }
    }

    pub fn get_mut(&mut self, i: usize, j: usize) -> &mut i32 {
        assert!(i < self.n);
        assert!(j < self.n);

        &mut self.coefs[i*self.n + j]
    }

    pub fn get(&self, i: usize, j: usize) -> &i32 {
        assert!(i < self.n);
        assert!(j < self.n);

        &self.coefs[i*self.n + j]
    }
}

impl std::ops::Index<(usize,usize)> for SqMat {
    type Output = i32;

    fn index(&self, index: (usize,usize)) -> &Self::Output {
        self.get(index.0, index.1)
    }
}

impl std::ops::IndexMut<(usize,usize)> for SqMat {
    fn index_mut(&mut self, index: (usize,usize)) -> &mut Self::Output {
        self.get_mut(index.0, index.1)
    }
}

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Vect {
    coefs: Vec<i32>,
}

impl std::fmt::Display for Vect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.coefs.len();

        let mut width = 0;

        for i in 0..n {
            let txt = format!("{}", self.coefs[i]);
            let len = txt.len();
            if len > width {
                width = len;
            }
        }

        for i in 0..n {
            write!(f, "[ {:>w$} ]", self.coefs[i], w=width)?;
            
            if i != n-1 {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}

impl Vect {
    pub fn new(n: usize) -> Vect {
        Vect {
            coefs: vec![0;n],
        }
    }

    pub fn get_mut(&mut self, i: usize) -> &mut i32 {
        assert!(i < self.coefs.len());

        &mut self.coefs[i]
    }

    pub fn get(&self, i: usize) -> &i32 {
        assert!(i < self.coefs.len());

        &self.coefs[i]
    }

    pub fn is_zero(&self) -> bool {
        for value in &self.coefs {
            if *value != 0 {
                return false;
            }
        }
        true
    }

    pub fn is_negative(&self) -> bool {
        for value in &self.coefs {
            if *value > 0 {
                return false;
            }
        }
        true
    }
}

impl std::ops::Index<usize> for Vect {
    type Output = i32;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
    }
}

impl std::ops::IndexMut<usize> for Vect {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
    }
}

impl std::ops::Add for &SqMat {
    type Output = SqMat;

    fn add(self, rhs: &SqMat) -> SqMat {
        assert_eq!(self.n, rhs.n);

        let n = self.n;
        let mut output = SqMat::new(n);

        for i in 0..n {
            for j in 0..n {
                output[(i,j)] = self[(i,j)] + rhs[(i,j)];
            }
        }

        output
    }
}

impl std::ops::Add<SqMat> for &SqMat {
    type Output = SqMat;

    fn add(self, rhs: SqMat) -> SqMat {
        self + &rhs
    }
}

impl std::ops::Add<&SqMat> for SqMat {
    type Output = SqMat;

    fn add(self, rhs: &SqMat) -> SqMat {
        &self + rhs
    }
}

impl std::ops::Add<SqMat> for SqMat {
    type Output = SqMat;

    fn add(self, rhs: SqMat) -> SqMat {
        &self + &rhs
    }
}

impl std::ops::Add for &Vect {
    type Output = Vect;

    fn add(self, rhs: &Vect) -> Vect {
        assert_eq!(self.coefs.len(), rhs.coefs.len());

        let n = self.coefs.len();
        let mut output = Vect::new(n);

        for i in 0..n {
            output[i] = self[i] + rhs[i];
        }

        output
    }
}

impl std::ops::Add<Vect> for &Vect {
    type Output = Vect;

    fn add(self, rhs: Vect) -> Vect {
        self + &rhs
    }
}

impl std::ops::Add<&Vect> for Vect {
    type Output = Vect;

    fn add(self, rhs: &Vect) -> Vect {
        &self + rhs
    }
}

impl std::ops::Add<Vect> for Vect {
    type Output = Vect;

    fn add(self, rhs: Vect) -> Vect {
        &self + &rhs
    }
}

impl std::ops::Mul for &SqMat {
    type Output = SqMat;

    fn mul(self, rhs: &SqMat) -> SqMat {
        assert_eq!(self.n, rhs.n);

        let n = self.n;
        let mut output = SqMat::new(n);

        for i in 0..n {
            for j in 0..n {
                for k in 0 ..n {
                    output[(i,j)] += self[(i,k)]*rhs[(k,j)];
                }
            }
        }

        output
    }
}

impl std::ops::Mul<SqMat> for &SqMat {
    type Output = SqMat;

    fn mul(self, rhs: SqMat) -> SqMat {
        self * &rhs
    }
}

impl std::ops::Mul<&SqMat> for SqMat {
    type Output = SqMat;

    fn mul(self, rhs: &SqMat) -> SqMat {
        &self * rhs
    }
}

impl std::ops::Mul<SqMat> for SqMat {
    type Output = SqMat;

    fn mul(self, rhs: SqMat) -> SqMat {
        &self * &rhs
    }
}

impl std::ops::Mul<&Vect> for &SqMat {
    type Output = Vect;

    fn mul(self, rhs: &Vect) -> Vect {
        assert_eq!(self.n, rhs.coefs.len());

        let n = self.n;
        let mut output = Vect::new(n);

        for i in 0..n {
            for k in 0 ..n {
                output[i] += self[(i,k)]*rhs[k];
            }
        }

        output
    }
}

impl std::ops::Mul<Vect> for &SqMat {
    type Output = Vect;

    fn mul(self, rhs: Vect) -> Vect {
        self * &rhs
    }
}

impl std::ops::Mul<&Vect> for SqMat {
    type Output = Vect;

    fn mul(self, rhs: &Vect) -> Vect {
        &self * rhs
    }
}

impl std::ops::Mul<Vect> for SqMat {
    type Output = Vect;

    fn mul(self, rhs: Vect) -> Vect {
        &self * &rhs
    }
}

