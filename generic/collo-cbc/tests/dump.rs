// A dumped model must come back exactly as it went out. The dump is meant to
// be a *faithful* reproducer of a solve that misbehaves in the field, so
// anything the text format rounds, reorders or drops would send the replay
// chasing a different problem than the one that failed.

use std::path::{Path, PathBuf};

use collo_cbc::{Model, ProblemDesc, Status};

#[path = "common/builder.rs"]
mod builder;
use builder::Builder;

/// A path under the system temp dir that removes itself when the test ends,
/// pass or fail.
struct TempPath(PathBuf);

impl TempPath {
    fn new(name: &str) -> Self {
        TempPath(std::env::temp_dir().join(format!("collo-cbc-{}-{name}", std::process::id())))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A model whose values exercise the parts of the format most likely to be
/// lossy: infinite row bounds on both sides, a column fixed by its bounds, a
/// continuous column among the integers, a negative coefficient, and objective
/// coefficients that have no exact short decimal form.
fn awkward_model() -> Builder {
    let mut b = Builder::new(6, -1, vec![5.0, 1.0 / 3.0, 0.1, 7.0, -8.5, 1e-9]);
    b.fix(5, 1.0);
    b.is_integer[4] = 0;
    b.col_ub[4] = 2.5;
    // A one-sided row each way, so both `inf` and `-inf` have to survive.
    b.add_row(vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0], f64::NEG_INFINITY, 3.0);
    b.add_row(vec![0.0, 0.0, 1.0, 1.0, 1.0, 0.0], 1.0, f64::INFINITY);
    // An equality with a negative coefficient.
    b.add_row(vec![1.0, -1.0, 0.0, 0.0, 0.0, 0.0], 0.0, 0.0);
    b
}

#[test]
fn model_round_trips_through_a_dump() {
    let b = awkward_model();
    let desc = b.build();

    let file = TempPath::new("round-trip.collomodel");
    desc.write_to(file.path()).expect("write model");
    let reread = ProblemDesc::read_from(file.path()).expect("read model");

    // Field by field first, so a failure says which one drifted; then the whole
    // struct, so a field added later without a format entry cannot slip through.
    assert_eq!(reread.num_cols, desc.num_cols);
    assert_eq!(reread.num_rows, desc.num_rows);
    assert_eq!(reread.obj_sense, desc.obj_sense);
    assert_eq!(reread.col_lb, desc.col_lb);
    assert_eq!(reread.col_ub, desc.col_ub);
    assert_eq!(reread.obj_coeffs, desc.obj_coeffs);
    assert_eq!(reread.is_integer, desc.is_integer);
    assert_eq!(reread.mat_start, desc.mat_start);
    assert_eq!(reread.mat_index, desc.mat_index);
    assert_eq!(reread.mat_value, desc.mat_value);
    assert_eq!(reread.row_lb, desc.row_lb);
    assert_eq!(reread.row_ub, desc.row_ub);
    assert_eq!(reread, desc);

    // The infinities specifically: `assert_eq!` above would also pass if both
    // sides had been clamped to the same finite value by the writer.
    assert_eq!(reread.row_lb[0], f64::NEG_INFINITY);
    assert_eq!(reread.row_ub[1], f64::INFINITY);
}

#[test]
fn mip_start_round_trips_through_a_dump() {
    let values = vec![1.0, 1.0, 0.0, 1.0, 2.5, 1.0, -0.0, 1.0 / 3.0];

    let file = TempPath::new("round-trip.collomipstart");
    collo_cbc::write_mip_start(file.path(), &values).expect("write mip start");
    let reread = collo_cbc::read_mip_start(file.path()).expect("read mip start");

    assert_eq!(reread, values);
}

#[test]
fn a_reread_model_solves_the_same_way() {
    let b = awkward_model();
    let desc = b.build();

    let file = TempPath::new("solve.collomodel");
    desc.write_to(file.path()).expect("write model");
    let reread = ProblemDesc::read_from(file.path()).expect("read model");

    let solve = |desc: &ProblemDesc| {
        let mut model = Model::new();
        model.load_problem(desc);
        model.set_log_level(0);
        model.solve()
    };

    let original = solve(&desc);
    let replayed = solve(&reread);

    assert_eq!(original.status, Status::Optimal);
    assert_eq!(replayed.status, original.status);
    assert_eq!(replayed.obj_value, original.obj_value);
    b.assert_feasible(replayed.solution.as_ref().expect("a solution"));
}

#[test]
fn a_truncated_dump_is_rejected() {
    let desc = awkward_model().build();

    let file = TempPath::new("truncated.collomodel");
    desc.write_to(file.path()).expect("write model");

    // Drop the last line (`row_ub`). A reader that silently accepted a short
    // dump would replay a model with fewer rows than the one that failed.
    let text = std::fs::read_to_string(file.path()).expect("read back");
    let mut lines: Vec<&str> = text.lines().collect();
    lines.pop();
    std::fs::write(file.path(), lines.join("\n") + "\n").expect("truncate");

    let err = ProblemDesc::read_from(file.path()).expect_err("a truncated dump must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}
