use super::super::*;

use ndarray::{Array, Array2, ArrayView};

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Solver<'a> {
    problem: &'a Problem,
    leq_mat: Array2<i32>,
    eq_mat: Array2<i32>,
}

impl<'a> Solver<'a> {
    pub fn new(problem: &'a Problem) -> Solver<'a> {
        let p = problem.variables.len() +1;

        let mut leq_mat = Array2::zeros((0,p));
        let mut eq_mat = Array2::zeros((0,p));

        for c in &problem.constraints {
            let mut current_row = Array::zeros(p);
            for (j,var) in problem.variables.iter().enumerate() {
                if let Some(val) = c.get_var(var) {
                    current_row[j] = val;
                }
            }
            current_row[p-1] = c.get_constant();
            match c.get_sign() {
                linexpr::Sign::Equals => eq_mat.push_row(ArrayView::from(&current_row)).unwrap(),
                linexpr::Sign::LessThan => leq_mat.push_row(ArrayView::from(&current_row)).unwrap(),
            }
        }

        Solver {
            problem,
            leq_mat,
            eq_mat,
        }
    }
}
