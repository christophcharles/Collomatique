use super::super::*;

pub struct Solver<'a> {
    problem: &'a Problem,
    leq_mat: ndarray::Array2<i32>,
    eq_mat: ndarray::Array2<i32>,
}

impl<'a> Solver<'a> {
    pub fn new(problem: &'a Problem) -> Solver<'a> {
        let mut leq_mat = ndarray::arr2(&[[0]]);
        let mut eq_mat = ndarray::arr2(&[[0]]);

        Solver {
            problem,
            leq_mat,
            eq_mat,
        }
    }
}
