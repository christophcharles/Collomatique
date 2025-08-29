use super::VariableName;
use crate::ilp::Config;

use std::collections::{BTreeMap, BTreeSet};

pub trait Heuristic: std::fmt::Debug + Clone {
    fn compute_guess_list<'a, V: VariableName>(
        &self,
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
    ) -> Vec<V>;
}

pub trait HeuristicWithScores: std::fmt::Debug + Clone {
    type Score: Ord + Clone;

    fn compute_scores<'a, V: VariableName>(
        &self,
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
    ) -> BTreeMap<V, Self::Score>;
}

impl<T: HeuristicWithScores> Heuristic for T {
    fn compute_guess_list<'a, V: VariableName>(
        &self,
        config: &Config<'a, V>,
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

#[derive(Debug, Clone, Default)]
pub struct Connolly1992 {}

impl Connolly1992 {
    pub fn new() -> Self {
        Connolly1992 {}
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

    fn compute_help<'a, V: VariableName>(
        config: &Config<'a, V>,
        available_variables: &BTreeSet<V>,
        constraint: &crate::ilp::linexpr::Constraint<V>,
        lhs: i32,
    ) -> i32 {
        let variables = constraint.variables();
        let mut help = 0;
        for var in variables.intersection(available_variables) {
            let coef = constraint.get_var(var).expect("Variable should be valid");
            let current_val = config.get(var).expect("Variable should be valid");

            if Self::is_var_helpful(constraint, lhs, coef, current_val) {
                help += coef.abs();
            }
        }
        help
    }
}

use ordered_float::NotNan;
impl HeuristicWithScores for Connolly1992 {
    type Score = NotNan<f64>;

    fn compute_scores<'a, V: VariableName>(
        &self,
        config: &Config<'a, V>,
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

            let help = Self::compute_help(config, available_variables, c, *lhs);
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
                if !Self::is_var_helpful(c, *lhs, coef, current_val) {
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
