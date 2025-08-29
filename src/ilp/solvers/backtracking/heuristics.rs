use super::VariableName;
use crate::ilp::Config;

use std::collections::{BTreeMap, BTreeSet};

use ordered_float::NotNan;

use crate::ilp::mat_repr::ProblemRepr;

pub trait Heuristic: std::fmt::Debug + Clone {
    fn compute_guess_list<'a, V: VariableName, P: ProblemRepr<V>>(
        &self,
        config: &Config<'a, V, P>,
        available_variables: &BTreeSet<V>,
    ) -> Vec<V>;
}

pub trait HeuristicWithScores: std::fmt::Debug + Clone {
    type Score: Ord + Clone;

    fn compute_scores<'a, V: VariableName, P: ProblemRepr<V>>(
        &self,
        config: &Config<'a, V, P>,
        available_variables: &BTreeSet<V>,
    ) -> BTreeMap<V, Self::Score>;
}

impl<T: HeuristicWithScores> Heuristic for T {
    fn compute_guess_list<'a, V: VariableName, P: ProblemRepr<V>>(
        &self,
        config: &Config<'a, V, P>,
        available_variables: &BTreeSet<V>,
    ) -> Vec<V> {
        let mut scores_vec: Vec<_> = self
            .compute_scores(config, available_variables)
            .into_iter()
            .collect();
        scores_vec.sort_by_key(|(_v, s)| s.clone());
        scores_vec.into_iter().rev().map(|(v, _s)| v).collect()
    }
}

fn is_var_helpful<V: VariableName>(
    constraint: &crate::ilp::linexpr::Constraint<V>,
    lhs: i32,
    coef: i32,
    current_val: bool,
) -> bool {
    use crate::ilp::linexpr::Sign;
    match constraint.get_sign() {
        Sign::LessThan => {
            if current_val == true {
                coef > 0
            } else {
                coef < 0
            }
        }
        Sign::Equals => {
            if current_val == true {
                coef * lhs > 0
            } else {
                coef * lhs < 0
            }
        }
    }
}

fn compute_help<'a, V: VariableName, P: ProblemRepr<V>>(
    config: &Config<'a, V, P>,
    available_variables: &BTreeSet<V>,
    constraint: &crate::ilp::linexpr::Constraint<V>,
    lhs: i32,
) -> i32 {
    let variables = constraint.variables();
    let mut help = 0;
    for var in variables.intersection(available_variables) {
        let coef = constraint.get_var(var).expect("Variable should be valid");
        let current_val = config.get(var).expect("Variable should be valid");

        if is_var_helpful(constraint, lhs, coef, current_val) {
            help += coef.abs();
        }
    }
    help
}

/// Heuristic offered in Conolly's 1992 paper (<https://doi.org/10.1057/jors.1992.75>).
/// This is an implementation of his GETSWOP function in the
/// context of our backtracking algorithm
#[derive(Debug, Clone, Default)]
pub struct Connolly1992 {}

impl Connolly1992 {
    pub fn new() -> Self {
        Connolly1992 {}
    }
}

impl HeuristicWithScores for Connolly1992 {
    type Score = NotNan<f64>;

    fn compute_scores<'a, V: VariableName, P: ProblemRepr<V>>(
        &self,
        config: &Config<'a, V, P>,
        available_variables: &BTreeSet<V>,
    ) -> BTreeMap<V, Self::Score> {
        let lhs_map = config.compute_lhs();

        let mut scores = BTreeMap::<V, NotNan<f64>>::new();

        for (c, lhs) in &lhs_map {
            use crate::ilp::linexpr::Sign;
            let infeasability = match c.get_sign() {
                Sign::LessThan => (*lhs).max(0),
                Sign::Equals => (*lhs).abs(),
            };
            assert!(infeasability >= 0);

            if infeasability == 0 {
                continue;
            }

            let help = compute_help(config, available_variables, c, *lhs);
            assert!(help >= 0);

            if infeasability > help {
                return BTreeMap::new();
            }

            let inf_f64 = NotNan::new(f64::from(infeasability)).unwrap();
            let help_f64 = NotNan::new(f64::from(help)).unwrap();

            let k = inf_f64 / help_f64;

            let variables = c.variables();
            for var in variables.intersection(available_variables) {
                let coef = c.get_var(var).expect("Variable should be valid");
                let current_val = config.get(var).expect("Variable should be valid");
                if !is_var_helpful(c, *lhs, coef, current_val) {
                    continue;
                }

                let coef_abs = coef.abs();
                if infeasability > help - coef_abs {
                    return BTreeMap::from([(var.clone(), NotNan::new(0.0).unwrap())]);
                }

                let coef_f64 = NotNan::new(f64::from(coef_abs)).unwrap();
                let temp_score = k * coef_f64;
                match scores.get_mut(var) {
                    Some(score) => {
                        *score += temp_score;
                    }
                    None => {
                        scores.insert(var.clone(), temp_score);
                    }
                }
            }
        }

        scores
    }
}

/// Heuristic inspired from Knuth's heuristic for his DLX algorithm
/// In <https://arxiv.org/pdf/cs/0011047>, page 6, Knuth proposes that we minimize
/// the branching factor by choosing a column with a minimal number of ones
///
/// In our context, there are several possible generalizations. A possible way to interpret
/// that is to take the constraint with the highest criticality and try to solve that one first.
/// It does reduce to Knuth's heuristic when applying this a cover problem.
#[derive(Debug, Clone, Default)]
pub struct Knuth2000 {}

impl Knuth2000 {
    pub fn new() -> Self {
        Knuth2000 {}
    }
}

impl HeuristicWithScores for Knuth2000 {
    type Score = NotNan<f64>;

    fn compute_scores<'a, V: VariableName, P: ProblemRepr<V>>(
        &self,
        config: &Config<'a, V, P>,
        available_variables: &BTreeSet<V>,
    ) -> BTreeMap<V, Self::Score> {
        let lhs_map = config.compute_lhs();

        let mut max_criticality = NotNan::new(-1.0).unwrap();
        let mut max_constraint = config
            .get_problem()
            .get_constraints()
            .first()
            .expect("There should be constraints to solve")
            .clone();

        let mut scores = BTreeMap::<V, NotNan<f64>>::new();
        for (c, lhs) in &lhs_map {
            use crate::ilp::linexpr::Sign;
            let infeasability = match c.get_sign() {
                Sign::LessThan => (*lhs).max(0),
                Sign::Equals => (*lhs).abs(),
            };
            assert!(infeasability >= 0);

            if infeasability == 0 {
                continue;
            }

            let help = compute_help(config, available_variables, c, *lhs);
            assert!(help >= 0);

            if infeasability > help {
                return BTreeMap::new();
            }

            let inf_f64 = NotNan::new(f64::from(infeasability)).unwrap();
            let help_f64 = NotNan::new(f64::from(help)).unwrap();

            let k = inf_f64 / help_f64;
            if k > max_criticality {
                max_criticality = k;
                max_constraint = c.clone();
            }

            let variables = c.variables();
            for var in variables.intersection(available_variables) {
                let coef = c.get_var(var).expect("Variable should be valid");
                let current_val = config.get(var).expect("Variable should be valid");
                if !is_var_helpful(c, *lhs, coef, current_val) {
                    continue;
                }

                let coef_abs = coef.abs();
                if infeasability > help - coef_abs {
                    return BTreeMap::from([(var.clone(), NotNan::new(0.0).unwrap())]);
                }

                let coef_f64 = NotNan::new(f64::from(coef_abs)).unwrap();
                let temp_score = k * coef_f64;
                match scores.get_mut(var) {
                    Some(score) => {
                        *score += temp_score;
                    }
                    None => {
                        scores.insert(var.clone(), temp_score);
                    }
                }
            }
        }

        scores
            .into_iter()
            .filter(|(v, _s)| max_constraint.get_var(v).is_some())
            .collect()
    }
}
