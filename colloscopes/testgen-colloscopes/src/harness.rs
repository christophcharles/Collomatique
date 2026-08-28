//! Property-test harness
//!
//! Runs each property over a fixed range of seeds with a fully
//! deterministic RNG ([ChaCha8Rng] is documented as portable and
//! reproducible). On any panic the seed and the full op log are printed
//! so the failure replays exactly.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    AssignmentOp, Data, NewId, Op, PeriodOp, StudentOp, SubjectOp, TeacherOp, WeekOp,
    ids::{StudentId, WeekId},
};

use crate::generator::CATEGORIES;
use crate::synth;

/// Configuration for one property run: how many seeds, how many ops per
/// seed, and what fraction of generated ops should deliberately break a
/// constraint. Each consuming test declares its own `RunConfig` constant.
pub struct RunConfig {
    pub seeds: u64,
    pub ops_per_run: usize,
    pub invalid_fraction: f64,
}

/// Log of the operations generated during one run, for failure replay
#[derive(Default)]
pub struct OpLog {
    entries: Vec<String>,
}

impl OpLog {
    pub fn push(&mut self, category: &'static str, op: &Op) {
        let mut debug = format!("{op:?}");
        if debug.len() > 400 {
            debug.truncate(400);
            debug.push_str("…");
        }
        self.entries
            .push(format!("[{}] {category}: {debug}", self.entries.len()));
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    fn render(&self) -> String {
        self.entries.join("\n")
    }
}

/// Per-run op statistics, used for the honesty guards
#[derive(Default)]
pub struct RunStats {
    attempted: BTreeMap<&'static str, usize>,
    succeeded: BTreeMap<&'static str, usize>,
}

impl RunStats {
    pub fn record(&mut self, category: &'static str, success: bool) {
        *self.attempted.entry(category).or_default() += 1;
        if success {
            *self.succeeded.entry(category).or_default() += 1;
        }
    }

    fn totals(&self) -> (usize, usize) {
        (self.attempted.values().sum(), self.succeeded.values().sum())
    }
}

/// Runs `body` once on `seed`, with the failure report and the per-run
/// honesty guard
///
/// `at` names the run in both messages: `"seed 3"` for a plain seed loop, or
/// `"start hogwarts, seed 3"` when the run also has a start point. Returns
/// the op categories this run attempted, for the caller's global guard.
fn run_one_seed(
    name: &str,
    at: &str,
    seed: u64,
    body: impl FnOnce(&mut ChaCha8Rng, &mut OpLog, &mut RunStats),
) -> BTreeSet<&'static str> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut log = OpLog::default();
    let mut stats = RunStats::default();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        body(&mut rng, &mut log, &mut stats)
    }));

    if let Err(payload) = result {
        eprintln!(
            "property `{name}`: {at} FAILED after {} generated ops. Op log:\n{}",
            log.len(),
            log.render(),
        );
        std::panic::resume_unwind(payload);
    }

    let (attempts, successes) = stats.totals();
    assert!(
        attempts == 0 || successes * 2 >= attempts,
        "property `{name}` {at}: only {successes}/{attempts} generated ops succeeded \
         — the generator degenerated into an error loop",
    );

    stats.attempted.keys().copied().collect()
}

/// The second honesty guard: over a whole run, every op category must have
/// been attempted somewhere.
fn assert_every_category_attempted(name: &str, attempted: &BTreeSet<&'static str>) {
    for category in CATEGORIES {
        assert!(
            attempted.contains(category),
            "property `{name}`: op category `{category}` was never attempted",
        );
    }
}

/// Runs `body` for every seed in the configuration
///
/// Honesty guards prevent the properties from silently degenerating:
/// - per run, at least half the generated ops must apply successfully;
/// - over the whole seed range, every op category must have been attempted.
pub fn for_each_seed(
    name: &str,
    cfg: &RunConfig,
    body: impl Fn(&mut ChaCha8Rng, &mut OpLog, &mut RunStats),
) {
    let mut global_attempted: BTreeSet<&'static str> = BTreeSet::new();
    let start = std::time::Instant::now();

    for seed in 0..cfg.seeds {
        global_attempted.extend(run_one_seed(name, &format!("seed {seed}"), seed, &body));
    }

    assert_every_category_attempted(name, &global_attempted);

    eprintln!(
        "property `{name}`: {} seeds × {} ops in {:.2?}",
        cfg.seeds,
        cfg.ops_per_run,
        start.elapsed(),
    );
}

/// Like [for_each_seed], but runs `body` once per (start point, seed)
///
/// The start type is the caller's business — the harness only displays it,
/// in the failure report beside the seed and the op log, and in the timing
/// line. That keeps the harness free of any knowledge of what a document is
/// or where one is loaded from.
///
/// `seeds_for` is how a per-start seed budget is expressed: the caller
/// decides how many seeds each start deserves, which is what lets several
/// expensive starts cost about as much as one cheap one.
///
/// The two honesty guards are the same as [for_each_seed]'s, with the first
/// applied per (start, seed) rather than per seed.
pub fn for_each_start_and_seed<S: std::fmt::Display>(
    name: &str,
    cfg: &RunConfig,
    starts: &[S],
    seeds_for: impl Fn(&S) -> u64,
    body: impl Fn(&S, &mut ChaCha8Rng, &mut OpLog, &mut RunStats),
) {
    let mut global_attempted: BTreeSet<&'static str> = BTreeSet::new();
    let overall = std::time::Instant::now();
    let mut timings = vec![];

    for start in starts {
        let seeds = seeds_for(start);
        let started = std::time::Instant::now();

        for seed in 0..seeds {
            global_attempted.extend(run_one_seed(
                name,
                &format!("start {start}, seed {seed}"),
                seed,
                |rng, log, stats| body(start, rng, log, stats),
            ));
        }

        timings.push(format!(
            "{start}: {seeds} seeds in {:.2?}",
            started.elapsed(),
        ));
    }

    assert_every_category_attempted(name, &global_attempted);

    // Per-start times, so a start point that blows the walk's time budget is
    // visible in the run output without instrumenting anything.
    eprintln!(
        "property `{name}`: {} ops per seed — {} — total {:.2?}",
        cfg.ops_per_run,
        timings.join(", "),
        overall.elapsed(),
    );
}

/// How big a bootstrapped document is
///
/// [Default] is the small document the walks have always started from.
/// A caller that wants a bigger one — the fixture generator — asks for it
/// here, so that the assignment rows have a student pool worth filling:
/// a row drifts toward 0.6 × the student count, and slowly, so the pool has
/// to be big at op zero rather than grown along the way.
pub struct BootstrapScale {
    pub periods: RangeInclusive<u32>,
    pub students: RangeInclusive<u32>,
    pub subjects: RangeInclusive<u32>,
    pub teachers: RangeInclusive<u32>,
    /// Chance that a given student joins a given (period, subject) row, or
    /// [None] to create no assignment rows at all
    ///
    /// [bootstrap] asks for none, which is why the walks' documents have never
    /// had a row worth splitting into groups. A row cannot be *grown* into one
    /// either: the generator adds one student at a time while a single period
    /// or subject removal wipes a whole row, and it removes far more often than
    /// 259 assignment ops can refill. So a fixture-grade document has to arrive
    /// with its rows already full, exactly as a real one does.
    pub assigned_fraction: Option<f64>,
}

impl Default for BootstrapScale {
    fn default() -> BootstrapScale {
        BootstrapScale {
            periods: 1..=3,
            students: 3..=8,
            subjects: 2..=4,
            teachers: 2..=3,
            assigned_fraction: None,
        }
    }
}

/// Builds a small but non-degenerate document through the gated op path
///
/// Returns the state and one [Data] snapshot per history position
/// (starting with the empty document), so undo/redo walks can compare
/// against every point of the history including the bootstrap ops.
pub fn bootstrap(rng: &mut ChaCha8Rng) -> (AppState<Data, String>, Vec<Data>) {
    bootstrap_with(rng, &BootstrapScale::default())
}

/// [bootstrap], at a chosen scale
///
/// The draw order and the distributions are the same whatever the scale, so
/// [bootstrap] — this function at [BootstrapScale::default] — consumes the
/// RNG exactly as it always has, and every existing walk is unchanged.
pub fn bootstrap_with(
    rng: &mut ChaCha8Rng,
    scale: &BootstrapScale,
) -> (AppState<Data, String>, Vec<Data>) {
    use rand::Rng;

    let mut state = AppState::<_, String>::new(Data::new());
    let mut snapshots = vec![state.get_data().clone()];

    let apply = |state: &mut AppState<Data, String>,
                 snapshots: &mut Vec<Data>,
                 op: Op,
                 desc: &str|
     -> Option<NewId> {
        let new_id = state
            .apply(op, desc.to_string())
            .unwrap_or_else(|e| panic!("bootstrap op `{desc}` failed: {e}"));
        snapshots.push(state.get_data().clone());
        new_id
    };

    // Periods (created empty, then their weeks spliced in one at a time —
    // periods no longer carry a week payload).
    let mut period_ids = vec![];
    for _ in 0..rng.random_range(scale.periods.clone()) {
        let op = match period_ids.last() {
            Some(&last) => PeriodOp::AddAfter(last),
            None => PeriodOp::AddFront,
        };
        let Some(NewId::PeriodId(id)) = apply(&mut state, &mut snapshots, Op::Period(op), "period")
        else {
            panic!("bootstrap: adding a period should return a period id");
        };
        period_ids.push(id);

        let mut prev_week: Option<WeekId> = None;
        for desc in synth::week_desc_vec(rng) {
            let op = match prev_week {
                Some(week) => WeekOp::AddAfter(week, desc),
                None => WeekOp::AddFront(id, desc),
            };
            let Some(NewId::WeekId(week_id)) =
                apply(&mut state, &mut snapshots, Op::Week(op), "week")
            else {
                panic!("bootstrap: adding a week should return a week id");
            };
            prev_week = Some(week_id);
        }
    }

    // Students (no period exclusions during bootstrap: keep it simple)
    for _ in 0..rng.random_range(scale.students.clone()) {
        apply(
            &mut state,
            &mut snapshots,
            Op::Student(StudentOp::Add(synth::student(rng, &[]))),
            "student",
        );
    }

    // Subjects: the first one always has interrogations so that slots,
    // teachers and colloscope ops have targets
    let mut interrogation_subject_ids = vec![];
    for i in 0..rng.random_range(scale.subjects.clone()) {
        let with_interrogation = i == 0 || rng.random_bool(0.7);
        let Some(NewId::SubjectId(id)) = apply(
            &mut state,
            &mut snapshots,
            Op::Subject(SubjectOp::AddAfter(None, {
                let mut subject = synth::subject(rng, &[], with_interrogation);
                subject.excluded_periods.clear();
                subject
            })),
            "subject",
        ) else {
            panic!("bootstrap: adding a subject should return a subject id");
        };
        if with_interrogation {
            interrogation_subject_ids.push(id);
        }
    }

    // Teachers: each teaches at least one interrogation subject
    for _ in 0..rng.random_range(scale.teachers.clone()) {
        let mut teacher = synth::teacher(rng, &interrogation_subject_ids);
        if teacher.subjects.is_empty() {
            teacher
                .subjects
                .insert(synth::pick(rng, &interrogation_subject_ids));
        }
        apply(
            &mut state,
            &mut snapshots,
            Op::Teacher(TeacherOp::Add(teacher)),
            "teacher",
        );
    }

    // Assignment rows, only if the scale asks for them. At
    // [BootstrapScale::default] this whole block is skipped, RNG draws
    // included, so [bootstrap] consumes the stream exactly as it always has.
    if let Some(fraction) = scale.assigned_fraction {
        // Rows are wanted on every subject, not only the ones that drew
        // interrogations, so the two lists are concatenated.
        let mut subject_ids = interrogation_subject_ids.clone();
        subject_ids.extend(
            state
                .get_data()
                .get_inner_data()
                .params
                .subjects
                .ordered_subject_list
                .iter()
                .map(|(id, _)| id)
                .filter(|id| !interrogation_subject_ids.contains(id)),
        );
        let student_ids: Vec<StudentId> = state
            .get_data()
            .get_inner_data()
            .params
            .students
            .student_map
            .iter()
            .map(|(id, _)| id)
            .collect();

        for &period in &period_ids {
            for &subject in &subject_ids {
                let row: BTreeSet<StudentId> = student_ids
                    .iter()
                    .copied()
                    .filter(|_| rng.random_bool(fraction))
                    .collect();
                if row.is_empty() {
                    continue;
                }
                apply(
                    &mut state,
                    &mut snapshots,
                    Op::Assignment(AssignmentOp::SetRow(period, subject, row)),
                    "assignment",
                );
            }
        }
    }

    (state, snapshots)
}
