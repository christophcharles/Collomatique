//! Objective module
//!
//! This modules mainly defines [ObjectiveSense] and [Objective].
//!
//! [ObjectiveSense] is a simple enum to describe optimization direction (minimize or maximize).
//! [Objective] is a structure to simplify the handling of linear objectives by bundling
//! a linear expression with the desired optimization direction.

use super::{LinExpr, UsableData};

/// Sense for the objectiove function
///
/// This enum represents the sense in which
/// we try to optimize the objective function
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ObjectiveSense {
    /// Minimize the objective function (default)
    #[default]
    Minimize,
    /// Maximize the objective function
    Maximize,
}

impl std::fmt::Display for ObjectiveSense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectiveSense::Minimize => write!(f, "Minimize"),
            ObjectiveSense::Maximize => write!(f, "Maximize"),
        }
    }
}

impl ObjectiveSense {
    /// Returns the reversed sense of optimisation.
    ///
    /// ```
    /// # use collomatique_ilp::ObjectiveSense;
    /// assert_eq!(ObjectiveSense::Maximize.reverse(), ObjectiveSense::Minimize);
    /// assert_eq!(ObjectiveSense::Minimize.reverse(), ObjectiveSense::Maximize);
    /// ```
    pub fn reverse(self) -> ObjectiveSense {
        match self {
            ObjectiveSense::Maximize => ObjectiveSense::Minimize,
            ObjectiveSense::Minimize => ObjectiveSense::Maximize,
        }
    }
}

/// Represents a linear objective
///
/// Objective consists of two parts: a linear function that
/// must be optimized and a direction of optimization (maximize or minimize).
/// This structure bundles the two together.
///
/// This is useful because the linear function usually does not mean much without
/// the direction of optimization.
///
/// Such an object is built with [Objective::new].
///
/// These object are also useful because they are made to compose easily.
/// For instance, you can simply add two obective together:
/// ```
/// # use collomatique_ilp::{LinExpr, Objective, ObjectiveSense};
/// let func1 = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
/// let sense1 = ObjectiveSense::Maximize;
/// let obj1 = Objective::new(func1, sense1);
///
/// let func2 = LinExpr::<String>::var("b") + 2.0*LinExpr::<String>::var("c");
/// let sense2 = ObjectiveSense::Maximize;
/// let obj2 = Objective::new(func2, sense2);
///
/// let expected_func = LinExpr::<String>::var("a") + 3.0*LinExpr::<String>::var("b") + 2.0*LinExpr::<String>::var("c");
/// let expected_sense = ObjectiveSense::Maximize;
/// let expected_obj = Objective::new(expected_func, expected_sense);
///
/// assert_eq!(obj1 + obj2, expected_obj);
/// ```
/// Note that if the optimization directions are different, this is automatically taken into account:
/// ```
/// # use collomatique_ilp::{LinExpr, Objective, ObjectiveSense};
/// let func1 = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
/// let sense1 = ObjectiveSense::Maximize;
/// let obj1 = Objective::new(func1, sense1);
///
/// let func2 = LinExpr::<String>::var("b") + 2.0*LinExpr::<String>::var("c");
/// let sense2 = ObjectiveSense::Minimize;
/// let obj2 = Objective::new(func2, sense2);
///
/// let expected_func = LinExpr::<String>::var("a") + LinExpr::<String>::var("b") - 2.0*LinExpr::<String>::var("c");
/// let expected_sense = ObjectiveSense::Maximize;
/// let expected_obj = Objective::new(expected_func, expected_sense);
///
/// assert_eq!(obj1 + obj2, expected_obj);
/// ```
/// The sense that is taken is the one in the first objective and the other one is flipped as needed:
/// ```
/// # use collomatique_ilp::{LinExpr, Objective, ObjectiveSense};
/// let func1 = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
/// let sense1 = ObjectiveSense::Minimize;
/// let obj1 = Objective::new(func1, sense1);
///
/// let func2 = LinExpr::<String>::var("b") + 2.0*LinExpr::<String>::var("c");
/// let sense2 = ObjectiveSense::Maximize;
/// let obj2 = Objective::new(func2, sense2);
///
/// let expected_func = LinExpr::<String>::var("a") + LinExpr::<String>::var("b") - 2.0*LinExpr::<String>::var("c");
/// let expected_sense = ObjectiveSense::Minimize;
/// let expected_obj = Objective::new(expected_func, expected_sense);
///
/// assert_eq!(obj1 + obj2, expected_obj);
/// ```
/// It is also possible to multiple an objective by a floating number:
/// ```
/// # use collomatique_ilp::{LinExpr, Objective, ObjectiveSense};
/// let func = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
/// let sense = ObjectiveSense::Maximize;
/// let obj = Objective::new(func, sense);
///
/// let expected_func = 2.0*LinExpr::<String>::var("a") + 4.0*LinExpr::<String>::var("b");
/// let expected_sense = ObjectiveSense::Maximize;
/// let expected_obj = Objective::new(expected_func, expected_sense);
///
/// assert_eq!(2.0*obj, expected_obj);
/// ```
/// Beware however, if the number you are multiplying is negative, the sense of optimization is also changed:
/// ```
/// # use collomatique_ilp::{LinExpr, Objective, ObjectiveSense};
/// let func = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
/// let sense = ObjectiveSense::Maximize;
/// let obj = Objective::new(func, sense);
///
/// let expected_func = -2.0*LinExpr::<String>::var("a") - 4.0*LinExpr::<String>::var("b");
/// let expected_sense = ObjectiveSense::Minimize;
/// let expected_obj = Objective::new(expected_func, expected_sense);
///
/// assert_eq!(-2.0*obj, expected_obj);
/// ```
/// This leads to a rather strange behaviour. If you substract two objectives, you end up getting the sum of them!
/// ```
/// # use collomatique_ilp::{LinExpr, Objective, ObjectiveSense};
/// let func1 = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
/// let sense1 = ObjectiveSense::Maximize;
/// let obj1 = Objective::new(func1, sense1);
///
/// let func2 = LinExpr::<String>::var("b") + 2.0*LinExpr::<String>::var("c");
/// let sense2 = ObjectiveSense::Maximize;
/// let obj2 = Objective::new(func2, sense2);
///
/// let expected_func = LinExpr::<String>::var("a") + 3.0*LinExpr::<String>::var("b") + 2.0*LinExpr::<String>::var("c");
/// let expected_sense = ObjectiveSense::Maximize;
/// let expected_obj = Objective::new(expected_func, expected_sense);
///
/// assert_eq!(obj1 - obj2, expected_obj);
/// ```
/// This is because you usually do not want to reverse an objective optimization sense. In effect, this means that the coefficients
/// are taken as their absolute values.
///
/// If you still want to reverse an objective, you can by using [Objective::reverse] or [Objective::reversed].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Objective<V: UsableData> {
    /// linear expression to optimize
    func: LinExpr<V>,
    /// objective sense for the objective
    sense: ObjectiveSense,
}

impl<V: UsableData> Objective<V> {
    /// Builds a new objective from a linear expression and an optimisation direction.
    pub fn new(func: LinExpr<V>, sense: ObjectiveSense) -> Self {
        Objective { func, sense }
    }

    /// Returns the internal expression to optimize
    pub fn get_function(&self) -> &LinExpr<V> {
        &self.func
    }

    /// Returns the optimisation direction
    pub fn get_sense(&self) -> ObjectiveSense {
        self.sense
    }

    /// Returns the reversed objective.
    ///
    /// This keeps the same expression but reverses the sense of optimization.
    /// 
    /// It is similar to [Objective::reverse] but builds a new objective.
    /// 
    /// **Beware !** This is almost always *not* what you want to do.
    ///
    /// ```
    /// # use collomatique_ilp::{LinExpr, Objective, ObjectiveSense};
    /// let func = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
    /// let sense = ObjectiveSense::Maximize;
    /// let obj = Objective::new(func, sense);
    ///
    /// let expected_func = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
    /// let expected_sense = ObjectiveSense::Minimize;
    /// let expected_obj = Objective::new(expected_func, expected_sense);
    ///
    /// assert_eq!(obj.reverse(), expected_obj);
    /// ```
    pub fn reversed(&self) -> Objective<V> {
        self.clone().reverse()
    }

    /// Returns the reversed objective.
    /// 
    /// This keeps the same expression but reverses the sense of optimization.
    /// 
    /// It is similar to [Objective::reversed] but consumes the objective.
    /// 
    /// **Beware !** This is almost always *not* what you want to do.
    ///
    /// ```
    /// # use collomatique_ilp::{LinExpr, Objective, ObjectiveSense};
    /// let func = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
    /// let sense = ObjectiveSense::Maximize;
    /// let obj = Objective::new(func, sense);
    ///
    /// let expected_func = LinExpr::<String>::var("a") + 2.0*LinExpr::<String>::var("b");
    /// let expected_sense = ObjectiveSense::Minimize;
    /// let expected_obj = Objective::new(expected_func, expected_sense);
    ///
    /// assert_eq!(obj.reversed(), expected_obj);
    /// ```
    pub fn reverse(self) -> Objective<V> {
        Objective {
            func: self.func,
            sense: self.sense.reverse(),
        }
    }
}

impl<V: UsableData> std::ops::Add for &Objective<V> {
    type Output = Objective<V>;

    fn add(self, rhs: &Objective<V>) -> Self::Output {
        Objective {
            func: if self.sense == rhs.sense {
                &self.func + &rhs.func
            } else {
                &self.func - &rhs.func
            },
            sense: self.sense,
        }
    }
}

impl<V: UsableData> std::ops::Add for Objective<V> {
    type Output = Objective<V>;

    fn add(self, rhs: Objective<V>) -> Self::Output {
        &self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<Objective<V>> for &Objective<V> {
    type Output = Objective<V>;

    fn add(self, rhs: Objective<V>) -> Self::Output {
        self + &rhs
    }
}

impl<V: UsableData> std::ops::Add<&Objective<V>> for Objective<V> {
    type Output = Objective<V>;

    fn add(self, rhs: &Objective<V>) -> Self::Output {
        &self + rhs
    }
}

impl<V: UsableData> std::ops::Mul<&Objective<V>> for &f64 {
    type Output = Objective<V>;

    fn mul(self, rhs: &Objective<V>) -> Self::Output {
        Objective {
            func: self * &rhs.func,
            sense: if self.is_sign_positive() {
                rhs.sense
            } else {
                rhs.sense.reverse()
            },
        }
    }
}

impl<V: UsableData> std::ops::Mul<&Objective<V>> for f64 {
    type Output = Objective<V>;

    fn mul(self, rhs: &Objective<V>) -> Self::Output {
        (&self) * rhs
    }
}

impl<V: UsableData> std::ops::Mul<Objective<V>> for &f64 {
    type Output = Objective<V>;

    fn mul(self, rhs: Objective<V>) -> Self::Output {
        self * &rhs
    }
}

impl<V: UsableData> std::ops::Mul<Objective<V>> for f64 {
    type Output = Objective<V>;

    fn mul(self, rhs: Objective<V>) -> Self::Output {
        &self * &rhs
    }
}

impl<V: UsableData> std::ops::Mul<&Objective<V>> for &i32 {
    type Output = Objective<V>;

    fn mul(self, rhs: &Objective<V>) -> Self::Output {
        f64::from(*self) * rhs
    }
}

impl<V: UsableData> std::ops::Mul<&Objective<V>> for i32 {
    type Output = Objective<V>;

    fn mul(self, rhs: &Objective<V>) -> Self::Output {
        (&self) * rhs
    }
}

impl<V: UsableData> std::ops::Mul<Objective<V>> for &i32 {
    type Output = Objective<V>;

    fn mul(self, rhs: Objective<V>) -> Self::Output {
        self * &rhs
    }
}

impl<V: UsableData> std::ops::Mul<Objective<V>> for i32 {
    type Output = Objective<V>;

    fn mul(self, rhs: Objective<V>) -> Self::Output {
        &self * &rhs
    }
}

impl<V: UsableData> std::ops::Neg for &Objective<V> {
    type Output = Objective<V>;

    fn neg(self) -> Self::Output {
        (-1.0) * self
    }
}

impl<V: UsableData> std::ops::Neg for Objective<V> {
    type Output = Objective<V>;

    fn neg(self) -> Self::Output {
        -&self
    }
}

impl<V: UsableData> std::ops::Sub for &Objective<V> {
    type Output = Objective<V>;

    fn sub(self, rhs: &Objective<V>) -> Self::Output {
        self + (-1.0) * rhs
    }
}

impl<V: UsableData> std::ops::Sub for Objective<V> {
    type Output = Objective<V>;

    fn sub(self, rhs: Objective<V>) -> Self::Output {
        &self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<Objective<V>> for &Objective<V> {
    type Output = Objective<V>;

    fn sub(self, rhs: Objective<V>) -> Self::Output {
        self - &rhs
    }
}

impl<V: UsableData> std::ops::Sub<&Objective<V>> for Objective<V> {
    type Output = Objective<V>;

    fn sub(self, rhs: &Objective<V>) -> Self::Output {
        &self - rhs
    }
}
