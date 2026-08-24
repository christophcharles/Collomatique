// Shared test helper: author a MIP with a dense, row-major constraint matrix
// and convert it to the compressed-sparse-column form `ProblemDesc` expects.
//
// Included with `#[path = "common/builder.rs"] mod builder;` from each
// integration test that needs it. Not every test uses every method, hence the
// blanket dead-code allowance.
#![allow(dead_code)]

use collo_cbc::ProblemDesc;

pub struct Builder {
    pub num_cols: usize,
    pub obj_sense: i32, // 1 = minimize, -1 = maximize
    pub obj: Vec<f64>,
    pub col_lb: Vec<f64>,
    pub col_ub: Vec<f64>,
    pub is_integer: Vec<i32>,
    pub rows: Vec<(Vec<f64>, f64, f64)>, // (coeffs over all cols, row_lb, row_ub)
}

impl Builder {
    pub fn new(num_cols: usize, obj_sense: i32, obj: Vec<f64>) -> Self {
        assert_eq!(obj.len(), num_cols);
        Builder {
            num_cols,
            obj_sense,
            obj,
            col_lb: vec![0.0; num_cols],
            col_ub: vec![1.0; num_cols],
            is_integer: vec![1; num_cols],
            rows: Vec::new(),
        }
    }

    pub fn fix(&mut self, col: usize, value: f64) {
        self.col_lb[col] = value;
        self.col_ub[col] = value;
    }

    pub fn add_row(&mut self, coeffs: Vec<f64>, lb: f64, ub: f64) {
        assert_eq!(coeffs.len(), self.num_cols);
        self.rows.push((coeffs, lb, ub));
    }

    pub fn build(&self) -> ProblemDesc {
        let num_rows = self.rows.len();
        let mut col_entries: Vec<Vec<(i32, f64)>> = vec![vec![]; self.num_cols];
        for (r, (coeffs, _, _)) in self.rows.iter().enumerate() {
            for (c, &v) in coeffs.iter().enumerate() {
                if v != 0.0 {
                    col_entries[c].push((r as i32, v));
                }
            }
        }

        let mut mat_start = vec![0i32];
        let mut mat_index = Vec::new();
        let mut mat_value = Vec::new();
        for col in &col_entries {
            for &(r, v) in col {
                mat_index.push(r);
                mat_value.push(v);
            }
            mat_start.push(mat_index.len() as i32);
        }

        ProblemDesc {
            num_cols: self.num_cols as i32,
            num_rows: num_rows as i32,
            obj_sense: self.obj_sense,
            col_lb: self.col_lb.clone(),
            col_ub: self.col_ub.clone(),
            obj_coeffs: self.obj.clone(),
            is_integer: self.is_integer.clone(),
            mat_start,
            mat_index,
            mat_value,
            row_lb: self.rows.iter().map(|(_, lb, _)| *lb).collect(),
            row_ub: self.rows.iter().map(|(_, _, ub)| *ub).collect(),
        }
    }

    /// Assert a candidate solution is in original column space and feasible.
    pub fn assert_feasible(&self, sol: &[f64]) {
        assert_eq!(
            sol.len(),
            self.num_cols,
            "solution must be in original column space"
        );
        for c in 0..self.num_cols {
            assert!(
                sol[c] >= self.col_lb[c] - 1e-6 && sol[c] <= self.col_ub[c] + 1e-6,
                "col {c} = {} out of bounds [{}, {}]",
                sol[c],
                self.col_lb[c],
                self.col_ub[c]
            );
            if self.is_integer[c] != 0 {
                assert!(
                    (sol[c] - sol[c].round()).abs() < 1e-6,
                    "integer col {c} not integral: {}",
                    sol[c]
                );
            }
        }
        for (coeffs, lb, ub) in &self.rows {
            let lhs: f64 = coeffs.iter().zip(sol).map(|(a, x)| a * x).sum();
            assert!(
                lhs >= lb - 1e-6 && lhs <= ub + 1e-6,
                "constraint violated: {lhs} not in [{lb}, {ub}]"
            );
        }
    }
}
