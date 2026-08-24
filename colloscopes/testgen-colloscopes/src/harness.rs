//! Property-test harness
//!
//! Runs each property over a fixed range of seeds with a fully
//! deterministic RNG ([ChaCha8Rng] is documented as portable and
//! reproducible). On any panic the seed and the full op log are printed
//! so the failure replays exactly.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, NewId, Op, PeriodOp, StudentOp, SubjectOp, TeacherOp, WeekOp, ids::WeekId,
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
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut log = OpLog::default();
        let mut stats = RunStats::default();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            body(&mut rng, &mut log, &mut stats)
        }));

        if let Err(payload) = result {
            eprintln!(
                "property `{name}`: seed {seed} FAILED after {} generated ops. Op log:\n{}",
                log.len(),
                log.render(),
            );
            std::panic::resume_unwind(payload);
        }

        let (attempts, successes) = stats.totals();
        assert!(
            attempts == 0 || successes * 2 >= attempts,
            "property `{name}` seed {seed}: only {successes}/{attempts} generated ops succeeded \
             — the generator degenerated into an error loop",
        );
        global_attempted.extend(stats.attempted.keys());
    }

    for category in CATEGORIES {
        assert!(
            global_attempted.contains(category),
            "property `{name}`: op category `{category}` was never attempted",
        );
    }

    eprintln!(
        "property `{name}`: {} seeds × {} ops in {:.2?}",
        cfg.seeds,
        cfg.ops_per_run,
        start.elapsed(),
    );
}

/// Builds a small but non-degenerate document through the gated op path
///
/// Returns the state and one [Data] snapshot per history position
/// (starting with the empty document), so undo/redo walks can compare
/// against every point of the history including the bootstrap ops.
pub fn bootstrap(rng: &mut ChaCha8Rng) -> (AppState<Data, String>, Vec<Data>) {
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
    for _ in 0..rng.random_range(1..=3) {
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
    for _ in 0..rng.random_range(3..=8) {
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
    for i in 0..rng.random_range(2..=4) {
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
    for _ in 0..rng.random_range(2..=3) {
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

    (state, snapshots)
}
