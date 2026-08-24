// Stopping a solve from the progress callback must be reported as
// `Status::Stopped`.
//
// Everything downstream keys off that status: `ilp` derives
// `stopped_by_callback` from it, and when that is false a deadline stop is
// laundered into "proven optimal" (with a solution) or "infeasible" (without
// one) by the time it reaches the strategies — and an infeasible epoch aborts a
// whole incremental run. So a mislabelled stop is not a cosmetic problem.

use collo_cbc::{Model, Status};

#[path = "common/builder.rs"]
mod builder;
use builder::Builder;

/// A market-split instance: `num_rows` equalities over `num_cols` binaries,
/// each right-hand side half the row's coefficient sum. This is the standard
/// small-but-hard MIP family — CBC finds no feasible point quickly and searches
/// a large tree, which is what gives the callback something to stop.
///
/// The coefficients come from a fixed-seed generator rather than a random one,
/// so every run gets the same instance; a fresh draw could land on an easy one
/// that CBC finishes before the callback ever fires.
fn market_split(num_cols: usize, num_rows: usize) -> Builder {
    let mut seed: u64 = 0x5eed_1234;
    let mut next = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) % 100) as f64
    };

    let mut b = Builder::new(num_cols, 1, vec![1.0; num_cols]);
    for _ in 0..num_rows {
        let coeffs: Vec<f64> = (0..num_cols).map(|_| next()).collect();
        let rhs = (coeffs.iter().sum::<f64>() / 2.0).floor();
        b.add_row(coeffs, rhs, rhs);
    }
    b
}

#[test]
fn stopping_from_the_callback_reports_stopped() {
    let desc = market_split(40, 4).build();

    let mut model = Model::new();
    model.load_problem(&desc);
    model.set_log_level(0);

    let mut events = 0usize;
    let result = model.solve_with_callback(|_| {
        events += 1;
        false // stop on the very first progress event
    });

    // If this fires, the test proves nothing about the status mapping: the
    // solve finished without ever asking us whether to continue.
    assert!(
        events >= 1,
        "expected at least one progress event to stop on"
    );
    assert_eq!(
        result.status,
        Status::Stopped,
        "a solve stopped from the callback must report Stopped"
    );
}
