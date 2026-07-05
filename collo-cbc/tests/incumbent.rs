// Tests that incumbents reported during the solve (on `solution` events) are
// handed back in the *original* column space of the loaded problem — i.e. same
// columns, ordering and count as what we passed to `load_problem` — even when
// CBC's preprocessing reduces the model internally.
//
// Each problem below is crafted to *force* preprocessing to drop columns:
//   - a variable fixed by its bounds (presolve removes it), and
//   - an equality `x_i - x_j = 0` (a doubleton substitution removes a column).
// So the preprocessed column count is strictly smaller than the original one,
// and a naive forwarding of CBC's internal `bestSolution()` would have the
// wrong length/ordering.

use collo_cbc::{IncumbentEvent, Model, ProblemDesc, Status};

/// Small helper to author a MIP with a dense, row-major constraint matrix and
/// convert it to the compressed-sparse-column form `ProblemDesc` expects.
struct Builder {
    num_cols: usize,
    obj_sense: i32, // 1 = minimize, -1 = maximize
    obj: Vec<f64>,
    col_lb: Vec<f64>,
    col_ub: Vec<f64>,
    is_integer: Vec<i32>,
    rows: Vec<(Vec<f64>, f64, f64)>, // (coeffs over all cols, row_lb, row_ub)
}

impl Builder {
    fn new(num_cols: usize, obj_sense: i32, obj: Vec<f64>) -> Self {
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

    fn fix(&mut self, col: usize, value: f64) {
        self.col_lb[col] = value;
        self.col_ub[col] = value;
    }

    fn add_row(&mut self, coeffs: Vec<f64>, lb: f64, ub: f64) {
        assert_eq!(coeffs.len(), self.num_cols);
        self.rows.push((coeffs, lb, ub));
    }

    fn build(&self) -> ProblemDesc {
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
    fn assert_feasible(&self, sol: &[f64]) {
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

/// Run the model and collect the incumbents observed on `solution` events,
/// asserting each one is in original column space and feasible.
fn solve_collecting_incumbents(builder: &Builder) -> (Status, Vec<f64>, usize) {
    let desc = builder.build();
    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    // Force branch-and-bound to produce the incumbents (rather than a heuristic
    // solving the whole thing up front), and force real branching.
    model.set_disable_heuristics(true);
    model.set_disable_cuts(true);

    let mut events = 0usize;
    let mut last_incumbent: Vec<f64> = Vec::new();
    let result = model.solve_with_callback(|p| {
        if let IncumbentEvent::Reconstructed { solution, .. } = &p.incumbent {
            builder.assert_feasible(solution);
            last_incumbent = solution.clone();
            events += 1;
        }
        true
    });

    (result.status, last_incumbent, events)
}

#[test]
fn incumbent_is_in_original_column_space() {
    // 6 binaries, maximize. Column reductions are forced by:
    //   - x5 fixed to 1 (presolve drops it),
    //   - x0 == x1 via equality (doubleton substitution drops a column).
    let mut b = Builder::new(6, -1, vec![5.0, 4.0, 3.0, 7.0, 8.0, 6.0]);
    b.fix(5, 1.0);
    b.add_row(vec![3.0, 3.0, 4.0, 5.0, 6.0, 4.0], f64::NEG_INFINITY, 11.0); // knapsack
    b.add_row(vec![1.0, -1.0, 0.0, 0.0, 0.0, 0.0], 0.0, 0.0); // x0 - x1 = 0

    let (status, last_incumbent, events) = solve_collecting_incumbents(&b);

    assert_eq!(status, Status::Optimal);
    assert!(
        events >= 1,
        "expected at least one solution event carrying an incumbent"
    );

    // Final solution is in original space and feasible, and matches the last
    // incumbent we observed during the solve.
    let desc = b.build();
    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    model.set_disable_heuristics(true);
    model.set_disable_cuts(true);
    let final_result = model.solve();
    let final_sol = final_result.solution.expect("a final solution");
    b.assert_feasible(&final_sol);

    for (a, c) in last_incumbent.iter().zip(final_sol.iter()) {
        assert!(
            (a - c).abs() < 1e-6,
            "last incumbent {last_incumbent:?} should match final solution {final_sol:?}"
        );
    }
}

#[test]
fn warm_start_with_reduction() {
    // Same reducing problem as above (6 cols -> 2 preprocessed cols), but with a
    // MIP start given in *original* column space. This exercises the original->
    // preprocessed MIP-start translation (via originalColumns()), which only runs
    // when the preprocessed model still has columns for CbcMain1 to branch on.
    let mut b = Builder::new(6, -1, vec![5.0, 4.0, 3.0, 7.0, 8.0, 6.0]);
    b.fix(5, 1.0);
    b.add_row(vec![3.0, 3.0, 4.0, 5.0, 6.0, 4.0], f64::NEG_INFINITY, 11.0);
    b.add_row(vec![1.0, -1.0, 0.0, 0.0, 0.0, 0.0], 0.0, 0.0);

    let desc = b.build();
    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    // Feasible original-space start: only the fixed x5 = 1 (weight 4 <= 11),
    // x0 == x1 == 0 satisfies the equality.
    model.set_mip_start(&[0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);

    let result = model.solve();
    assert_eq!(result.status, Status::Optimal);
    let sol = result.solution.expect("a solution");
    b.assert_feasible(&sol);
}

#[test]
fn multiple_distinct_incumbents_reconstruct_correctly() {
    // The decisive regression test: a problem that produces SEVERAL distinct
    // improving incumbents during branch-and-bound, with column reduction. The
    // earlier (own-preprocessing + repeated postProcess) approach corrupted state
    // across distinct incumbents and looped; this must reconstruct every one.
    //
    // A "hard" knapsack (values == weights, capacity ~ half the total) forces
    // real branching and a sequence of improving integer solutions. Plus a fixed
    // var and an equality so preprocessing reduces the column count.
    let weights = [
        15.0, 14.0, 13.0, 12.0, 11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 17.0, 19.0, 23.0, 29.0, 31.0, 37.0,
        41.0, 43.0, 16.0, 18.0,
    ];
    let n = weights.len();
    let capacity: f64 = weights.iter().sum::<f64>() / 2.0 - 1.0;

    let mut b = Builder::new(n, -1, weights.to_vec()); // maximize value == weight
    b.fix(0, 1.0); // a fixed column (preprocessing drops it)
    b.add_row(weights.to_vec(), f64::NEG_INFINITY, capacity); // knapsack
    let mut eq = vec![0.0; n];
    eq[1] = 1.0;
    eq[2] = -1.0;
    b.add_row(eq, 0.0, 0.0); // x1 - x2 = 0 (doubleton -> drops a column)

    let desc = b.build();
    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    model.set_disable_heuristics(true);
    model.set_disable_cuts(true);

    let mut objectives: Vec<f64> = Vec::new();
    let result = model.solve_with_callback(|p| {
        if let IncumbentEvent::Reconstructed {
            objective,
            solution,
        } = &p.incumbent
        {
            // Every reconstructed incumbent must be original-space + feasible.
            b.assert_feasible(solution);
            objectives.push(*objective);
        }
        true
    });

    assert_eq!(result.status, Status::Optimal);
    let final_sol = result.solution.expect("a final solution");
    b.assert_feasible(&final_sol);

    // We must have reconstructed at least one incumbent; if several distinct
    // ones occurred, all were handled without infeasibility (the old bug).
    assert!(!objectives.is_empty(), "expected at least one incumbent");
    let distinct = {
        let mut v = objectives.clone();
        v.sort_by(|a, c| a.partial_cmp(c).unwrap());
        v.dedup_by(|a, c| (*a - *c).abs() < 1e-6);
        v.len()
    };
    eprintln!(
        "[test] incumbents seen: {}, distinct objectives: {}",
        objectives.len(),
        distinct
    );
    // Regression guard: the old approach corrupted state across distinct
    // incumbents. This instance reliably yields several; require at least two so
    // the repeatable-reconstruction path is actually exercised.
    assert!(
        distinct >= 2,
        "expected multiple distinct incumbents to exercise repeatable reconstruction, got {distinct}"
    );
}

#[test]
fn repeated_postsolve_is_stable() {
    // A larger knapsack with two reduction structures, to exercise the postsolve
    // path across (potentially) several incumbents. Note: even a single solution
    // event already triggers postsolve at least twice (once in the callback, once
    // for the final solution), so the postsolve chain's repeatability is covered
    // regardless of how many incumbents CBC happens to report.
    let mut b = Builder::new(
        10,
        -1,
        vec![8.0, 6.0, 5.0, 9.0, 7.0, 4.0, 10.0, 3.0, 6.0, 5.0],
    );
    b.fix(9, 1.0); // drop a column
    b.add_row(
        vec![4.0, 3.0, 3.0, 5.0, 4.0, 2.0, 6.0, 2.0, 3.0, 4.0],
        f64::NEG_INFINITY,
        18.0,
    ); // knapsack
    b.add_row(
        vec![1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        0.0,
        0.0,
    ); // x0 - x1 = 0 (doubleton)
    b.add_row(
        vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0],
        0.0,
        0.0,
    ); // x2 - x8 = 0 (doubleton)

    let (status, _last, events) = solve_collecting_incumbents(&b);
    assert_eq!(status, Status::Optimal);
    assert!(events >= 1, "expected at least one incumbent");
}

#[test]
fn best_bound_is_in_original_column_space() {
    // The bound (`getBestPossibleObjValue`) is a dual value with no vector to
    // reconstruct, so — unlike the incumbent objective — we cannot rebuild it in
    // original space; we transmit CBC's value directly and rely on its
    // objective-offset bookkeeping. This test pins that reliance: fix a variable
    // with a *large* objective coefficient so preprocessing introduces a big
    // objective offset. If the bound were reported in CBC's preprocessed space,
    // it would be offset *down* by that fixed contribution and fall below the
    // true optimum. Asserting the bound stays a valid upper bound on the
    // original-space objective at every event proves the offset bookkeeping is
    // live — the same mechanism that keeps the bound in original units.
    let mut b = Builder::new(6, -1, vec![100.0, 4.0, 3.0, 7.0, 8.0, 6.0]); // maximize
    b.fix(0, 1.0); // fixed var, objective coeff 100 -> offset of 100
    b.add_row(vec![3.0, 3.0, 4.0, 5.0, 6.0, 4.0], f64::NEG_INFINITY, 11.0); // knapsack
    b.add_row(vec![0.0, 1.0, -1.0, 0.0, 0.0, 0.0], 0.0, 0.0); // x1 - x2 = 0 (doubleton)

    let desc = b.build();

    // Final original-space optimum.
    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    model.set_disable_heuristics(true);
    model.set_disable_cuts(true);
    let final_result = model.solve();
    assert_eq!(final_result.status, Status::Optimal);
    let final_obj = final_result.obj_value; // original space (post-processed)
    b.assert_feasible(&final_result.solution.expect("a final solution"));
    // The fixed var alone contributes 100; a preprocessed-space value would be
    // ~100 lower. Sanity-check the optimum is well above that offset.
    assert!(
        final_obj >= 100.0,
        "final objective {final_obj} should include the fixed variable's 100"
    );

    // Collect the bound at every event and check it never drops below the
    // original-space optimum (it is an upper bound for a maximization problem).
    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    model.set_disable_heuristics(true);
    model.set_disable_cuts(true);
    let mut bounds: Vec<f64> = Vec::new();
    let result = model.solve_with_callback(|p| {
        bounds.push(p.best_bound);
        true
    });
    assert_eq!(result.status, Status::Optimal);
    assert!(
        !bounds.is_empty(),
        "expected progress events carrying a bound"
    );
    for bound in &bounds {
        assert!(
            *bound >= final_obj - 1e-6,
            "best_bound {bound} is below the original-space optimum {final_obj}; \
             it looks offset into preprocessed space"
        );
    }
    // Proven optimal: the final bound meets the original-space optimum.
    assert!(
        (result.best_bound - final_obj).abs() < 1e-6,
        "final best_bound {} should equal the original-space optimum {final_obj}",
        result.best_bound
    );
}

#[test]
fn best_bound_is_in_original_column_space_minimize() {
    // Minimization counterpart of best_bound_is_in_original_column_space. CBC
    // minimizes natively, so the preprocessed model keeps the user's sense and
    // the bound is NOT flipped — this guards that the sense-correction leaves the
    // minimize path untouched, and that the bound still reflects the original
    // objective offset introduced by a fixed variable.
    let mut b = Builder::new(6, 1, vec![100.0, 4.0, 3.0, 7.0, 8.0, 6.0]); // minimize
    b.fix(0, 1.0); // fixed var, objective coeff 100 -> offset of 100
    b.add_row(vec![3.0, 3.0, 4.0, 5.0, 6.0, 4.0], 10.0, f64::INFINITY); // covering
    b.add_row(vec![0.0, 1.0, -1.0, 0.0, 0.0, 0.0], 0.0, 0.0); // x1 - x2 = 0 (doubleton)

    let desc = b.build();

    // Final original-space optimum.
    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    model.set_disable_heuristics(true);
    model.set_disable_cuts(true);
    let final_result = model.solve();
    assert_eq!(final_result.status, Status::Optimal);
    let final_obj = final_result.obj_value; // original space (post-processed)
    b.assert_feasible(&final_result.solution.expect("a final solution"));
    assert!(
        final_obj >= 100.0,
        "final objective {final_obj} should include the fixed variable's 100"
    );

    // Collect the bound at every event. For a minimization problem it is a *lower*
    // bound: it must never exceed the optimum, and it must include the fixed
    // variable's offset (a preprocessed-space bound would omit the 100 and sit
    // far below it).
    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);
    model.set_disable_heuristics(true);
    model.set_disable_cuts(true);
    let mut bounds: Vec<f64> = Vec::new();
    let result = model.solve_with_callback(|p| {
        bounds.push(p.best_bound);
        true
    });
    assert_eq!(result.status, Status::Optimal);
    assert!(
        !bounds.is_empty(),
        "expected progress events carrying a bound"
    );
    for bound in &bounds {
        assert!(
            *bound <= final_obj + 1e-6,
            "best_bound {bound} exceeds the original-space optimum {final_obj}"
        );
        assert!(
            *bound >= 100.0 - 1e-6,
            "best_bound {bound} omits the fixed variable's offset; it looks like \
             preprocessed space"
        );
    }
    assert!(
        (result.best_bound - final_obj).abs() < 1e-6,
        "final best_bound {} should equal the original-space optimum {final_obj}",
        result.best_bound
    );
}
