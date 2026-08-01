//! Property fuzz over the fifteen user-facing [`UpdateOp`] families, driven
//! through the new cascade path.
//!
//! `ops/` had no fuzz at all until this file: every user-facing op was pinned
//! by hand-written fixtures on documents the test author chose. Fixtures say
//! what an op *should* do; they cannot say what happens on the documents nobody
//! thought of. This walk does the other half — it fires random [`UpdateOp`]s at
//! a random document and asserts only what must hold everywhere:
//!
//! * **no panic.** Implicit in the test passing, and the whole point. The new
//!   bodies keep a residual catch-all `panic!` per translation site, each
//!   meaning "the state layer produced an error this op cannot produce". Those
//!   arms are claims about reachability, and a fuzz walk is the only thing that
//!   argues with them. (Commit 5 will extend the loop to render every warning,
//!   so the renderer's own lookup panics get fuzzed for free.)
//! * **`Ok` ⇒ the committed document is valid.** The composite's own elementary
//!   ops and every repair they cascaded land together, so the state the caller
//!   is handed must satisfy the whole-model checker — no dangling reference, no
//!   convergence break, no logic error.
//! * **`Err` is fine.** A clean rejection is a legitimate outcome (bad address,
//!   convicted target); the walk never predicts which ops will land.
//!
//! **Two deliberate deviations from `state-colloscopes/tests/property_cascade.rs`**,
//! whose shape this file otherwise follows. First, the seed loop is written out
//! here instead of calling [`harness::for_each_seed`]: that helper's cross-seed
//! guard asserts every entry of `generator::CATEGORIES` — the seventeen
//! *elementary* op categories — was attempted, and its `OpLog::push` is typed to
//! the elementary [`collomatique_state_colloscopes::Op`]. Both are the wrong
//! vocabulary here; the fifteen families get their own coverage guard below.
//! Second, [`harness::bootstrap`] builds an `AppState<Data, String>`, so the
//! document it grows is re-homed onto this crate's `Desc`.
//!
//! **Coverage guards, because a green run is not by itself evidence.** A walk
//! whose every op was rejected, or that never made the cascade repair anything,
//! would pass just as happily while proving nothing. So the run counts what it
//! saw and insists on all three outcomes — ops that landed, ops that cascaded a
//! repair, ops that were rejected — plus at least one attempt per family.
//!
//! On failure the seed and the full op log are printed, so the sequence replays
//! exactly.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use collomatique_ops::{
    AssignmentsUpdateOp, BalancingUpdateOp, ColloscopeUpdateOp, Desc, ExportConfigUpdateOp,
    GeneralPlanningUpdateOp, GroupListsUpdateOp, IncompatibilitiesUpdateOp, PairingsUpdateOp,
    SettingsUpdateOp, SlotPairingsUpdateOp, SlotsUpdateOp, StudentsUpdateOp, SubjectsUpdateOp,
    TeachersUpdateOp, UpdateOp, WeekPatternsUpdateOp,
};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, InnerData,
    group_lists::{GroupList, GroupListFilling},
    ids::{
        GroupListId, Id, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
        SubjectId, TeacherId, WeekId, WeekPatternId,
    },
};
use collomatique_testgen_colloscopes::rand::{Rng, SeedableRng};
use collomatique_testgen_colloscopes::{ChaCha8Rng, harness, synth};

use harness::RunConfig;
use synth::pick;

/// The same width as the two elementary walks (`property_ops.rs` and
/// `property_apply_gate.rs` run 100 × 1000), and wider than the cascade
/// harness's 50 × 500 — every [`UpdateOp`] here is a *composite*, so one
/// generated op is several elementary ops and several cascades, and the walk
/// buys more coverage per op than either of those. Shrinking this is a later
/// decision, to be justified the way `property_ops.rs:32-34` justifies its own
/// — not a knob to reach for the first time the suite feels slow.
const CONFIG: RunConfig = RunConfig {
    seeds: 100,
    ops_per_run: 500,
    invalid_fraction: 0.15,
};

/// The fifteen families, for the coverage guard. Same names, same order as
/// [`UpdateOp`]'s own variants.
const FAMILIES: [&str; 15] = [
    "general_planning",
    "subjects",
    "teachers",
    "students",
    "assignments",
    "week_patterns",
    "slots",
    "incompatibilities",
    "pairings",
    "slot_pairings",
    "group_lists",
    "settings",
    "balancing",
    "colloscope",
    "export_config",
];

/// Base for ids that are guaranteed dangling — the same convention as
/// `testgen`'s own generator. Real ids are issued sequentially from 0 and never
/// reach this range.
const DANGLING_BASE: u64 = 1 << 40;

fn dangling(rng: &mut ChaCha8Rng) -> u64 {
    DANGLING_BASE + rng.random_range(0..1_000_000)
}

/// Picks an index according to `weights` (a zero weight is never picked).
fn weighted(rng: &mut ChaCha8Rng, weights: &[u32]) -> usize {
    let total: u32 = weights.iter().sum();
    assert!(total > 0, "at least one weight must be non-zero");
    let mut roll = rng.random_range(0..total);
    for (i, w) in weights.iter().enumerate() {
        if roll < *w {
            return i;
        }
        roll -= w;
    }
    unreachable!()
}

// ============================================================================
// Id pools
// ============================================================================

/// Live ids read from the current document before every op — the state itself
/// is the source of truth, exactly as in the elementary generator.
struct Pools {
    period_ids: Vec<PeriodId>,
    week_ids: Vec<WeekId>,
    student_ids: Vec<StudentId>,
    subject_ids: Vec<SubjectId>,
    interrogation_subject_ids: Vec<SubjectId>,
    teacher_ids: Vec<TeacherId>,
    week_pattern_ids: Vec<WeekPatternId>,
    slot_ids: Vec<SlotId>,
    incompat_ids: Vec<IncompatId>,
    group_list_ids: Vec<GroupListId>,
    pairing_rule_ids: Vec<PairingRuleId>,
    slot_pairing_rule_ids: Vec<SlotPairingRuleId>,
}

impl Pools {
    fn extract(inner: &InnerData) -> Pools {
        let params = &inner.params;
        Pools {
            period_ids: params.periods.period_ids().collect(),
            week_ids: params.week_ids().collect(),
            student_ids: params.students.student_map.keys().collect(),
            subject_ids: params
                .subjects
                .ordered_subject_list
                .iter()
                .map(|(id, _)| id)
                .collect(),
            interrogation_subject_ids: params
                .subjects
                .ordered_subject_list
                .iter()
                .filter(|(_, s)| s.parameters.interrogation_parameters.is_some())
                .map(|(id, _)| id)
                .collect(),
            teacher_ids: params.teachers.teacher_map.keys().collect(),
            week_pattern_ids: params.week_patterns.week_pattern_map.keys().collect(),
            slot_ids: params.slots.all_slots().map(|(id, _)| *id).collect(),
            incompat_ids: params.incompats.incompat_map.keys().collect(),
            group_list_ids: params.group_lists.group_list_map.keys().collect(),
            pairing_rule_ids: params.pairings.pairing_rule_map.keys().collect(),
            slot_pairing_rule_ids: params.slot_pairings.slot_pairing_rule_map.keys().collect(),
        }
    }
}

// A live id where there is one, a dangling id where the pool has run dry. The
// walk never predicts an outcome, so an address nothing answers to is a
// perfectly good draw — it exercises the family's address check instead of its
// body.
fn some_period(rng: &mut ChaCha8Rng, pools: &Pools) -> PeriodId {
    if pools.period_ids.is_empty() {
        unsafe { PeriodId::new(dangling(rng)) }
    } else {
        pick(rng, &pools.period_ids)
    }
}

fn some_subject(rng: &mut ChaCha8Rng, pools: &Pools) -> SubjectId {
    if pools.subject_ids.is_empty() {
        unsafe { SubjectId::new(dangling(rng)) }
    } else {
        pick(rng, &pools.subject_ids)
    }
}

fn some_interrogation_subject(rng: &mut ChaCha8Rng, pools: &Pools) -> SubjectId {
    if pools.interrogation_subject_ids.is_empty() {
        some_subject(rng, pools)
    } else {
        pick(rng, &pools.interrogation_subject_ids)
    }
}

fn some_student(rng: &mut ChaCha8Rng, pools: &Pools) -> StudentId {
    if pools.student_ids.is_empty() {
        unsafe { StudentId::new(dangling(rng)) }
    } else {
        pick(rng, &pools.student_ids)
    }
}

fn some_group_list(rng: &mut ChaCha8Rng, pools: &Pools) -> GroupListId {
    if pools.group_list_ids.is_empty() {
        unsafe { GroupListId::new(dangling(rng)) }
    } else {
        pick(rng, &pools.group_list_ids)
    }
}

/// The teachers who declare `subject_id` — the only ones a slot on that subject
/// may name.
fn teachers_for_subject(inner: &InnerData, subject_id: SubjectId) -> Vec<TeacherId> {
    inner
        .params
        .teachers
        .teacher_map
        .iter()
        .filter(|(_, teacher)| teacher.subjects.contains(&subject_id))
        .map(|(id, _)| id)
        .collect()
}

/// Interrogation subjects that have at least one teacher — the ones a slot can
/// currently be added to.
fn addable_slot_subjects(inner: &InnerData, pools: &Pools) -> Vec<SubjectId> {
    pools
        .interrogation_subject_ids
        .iter()
        .copied()
        .filter(|subject_id| !teachers_for_subject(inner, *subject_id).is_empty())
        .collect()
}

/// Two distinct elements of a slice of at least two — the shape both pairing
/// rule constructors demand (they reject a rule whose two sides are the same
/// entity).
fn distinct_pair<T: Copy>(rng: &mut ChaCha8Rng, items: &[T]) -> (T, T) {
    assert!(items.len() >= 2, "callers check the length first");
    let first = rng.random_range(0..items.len());
    let mut second = rng.random_range(0..items.len() - 1);
    if second >= first {
        second += 1;
    }
    (items[first], items[second])
}

// ============================================================================
// The generator
// ============================================================================

/// Generates the next user-facing op from the current document.
///
/// With probability `invalid_fraction` the draw is deliberately bad — an
/// address nothing answers to, or a payload the state layer will refuse. The
/// weights favour the destructive ops (deletes, cuts, merges, shrinks), because
/// those are the ones that make the cascade repair something, and a walk that
/// only ever adds material never exercises the map at all.
fn gen_update_op(
    rng: &mut ChaCha8Rng,
    data: &Data,
    invalid_fraction: f64,
) -> (&'static str, UpdateOp) {
    let inner = data.get_inner_data();
    let pools = Pools::extract(inner);
    let invalid = rng.random_bool(invalid_fraction);

    let weights: [u32; 15] = [
        6, // general_planning
        7, // subjects
        6, // teachers
        7, // students
        6, // assignments
        5, // week_patterns
        7, // slots
        4, // incompatibilities
        4, // pairings
        3, // slot_pairings
        7, // group_lists
        3, // settings
        3, // balancing
        6, // colloscope
        2, // export_config
    ];
    let family = FAMILIES[weighted(rng, &weights)];

    let op = match family {
        "general_planning" => {
            UpdateOp::GeneralPlanning(gen_general_planning(rng, inner, &pools, invalid))
        }
        "subjects" => UpdateOp::Subjects(gen_subjects(rng, &pools, invalid)),
        "teachers" => UpdateOp::Teachers(gen_teachers(rng, &pools, invalid)),
        "students" => UpdateOp::Students(gen_students(rng, &pools, invalid)),
        "assignments" => UpdateOp::Assignments(gen_assignments(rng, &pools, invalid)),
        "week_patterns" => UpdateOp::WeekPatterns(gen_week_patterns(rng, &pools, invalid)),
        "slots" => UpdateOp::Slots(gen_slots(rng, inner, &pools, invalid)),
        "incompatibilities" => {
            UpdateOp::Incompatibilities(gen_incompatibilities(rng, &pools, invalid))
        }
        "pairings" => UpdateOp::Pairings(gen_pairings(rng, &pools, invalid)),
        "slot_pairings" => UpdateOp::SlotPairings(gen_slot_pairings(rng, &pools, invalid)),
        "group_lists" => UpdateOp::GroupLists(gen_group_lists(rng, inner, &pools, invalid)),
        "settings" => UpdateOp::Settings(gen_settings(rng, &pools, invalid)),
        "balancing" => UpdateOp::Balancing(gen_balancing(rng, &pools, invalid)),
        "colloscope" => UpdateOp::Colloscope(gen_colloscope(rng, inner, &pools, invalid)),
        "export_config" => UpdateOp::ExportConfig(gen_export_config(rng)),
        _ => unreachable!(),
    };

    (family, op)
}

fn gen_general_planning(
    rng: &mut ChaCha8Rng,
    inner: &InnerData,
    pools: &Pools,
    invalid: bool,
) -> GeneralPlanningUpdateOp {
    if invalid {
        // A period id nothing answers to, on each of the arms that takes one.
        let ghost = unsafe { PeriodId::new(dangling(rng)) };
        return match rng.random_range(0..5) {
            0 => GeneralPlanningUpdateOp::DeletePeriodAndWeeks(ghost),
            1 => GeneralPlanningUpdateOp::UpdatePeriodWeekCount(ghost, rng.random_range(0..=5)),
            2 => GeneralPlanningUpdateOp::CutPeriod(ghost, rng.random_range(0..=5)),
            3 => GeneralPlanningUpdateOp::MergeWithPreviousPeriod(ghost),
            _ => GeneralPlanningUpdateOp::UpdateWeekStatus(
                ghost,
                rng.random_range(0..5),
                rng.random_bool(0.5),
            ),
        };
    }

    let period = some_period(rng, pools);
    // The period's own length, so week-indexed arms mostly aim inside it —
    // `max(1)` keeps the range non-empty for the periods that have no week yet,
    // in which case the draw is an out-of-range address, which is fine.
    let week_count = inner
        .params
        .weeks
        .week_count_for_period(period)
        .unwrap_or(0)
        .max(1);
    let period_w = if pools.period_ids.is_empty() { 0 } else { 3 };

    match weighted(
        rng,
        &[
            1, 1, 3, period_w, period_w, period_w, period_w, period_w, period_w,
        ],
    ) {
        0 => GeneralPlanningUpdateOp::DeleteFirstWeek,
        1 => GeneralPlanningUpdateOp::UpdateFirstWeek(synth::week_start(rng)),
        2 => GeneralPlanningUpdateOp::AddNewPeriod(rng.random_range(1..=4)),
        3 => GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period, rng.random_range(0..=6)),
        4 => GeneralPlanningUpdateOp::DeletePeriodAndWeeks(period),
        5 => GeneralPlanningUpdateOp::CutPeriod(period, rng.random_range(0..=week_count)),
        6 => GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period),
        7 => GeneralPlanningUpdateOp::UpdateWeekStatus(
            period,
            rng.random_range(0..week_count),
            rng.random_bool(0.4),
        ),
        _ => GeneralPlanningUpdateOp::UpdateWeekAnnotation(
            period,
            rng.random_range(0..week_count),
            rng.random_bool(0.5).then(|| {
                non_empty_string::NonEmptyString::new("note".to_string())
                    .expect("statically non-empty")
            }),
        ),
    }
}

fn gen_subjects(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> SubjectsUpdateOp {
    if invalid {
        let ghost = unsafe { SubjectId::new(dangling(rng)) };
        return match rng.random_range(0..3) {
            0 => SubjectsUpdateOp::DeleteSubject(ghost),
            1 => SubjectsUpdateOp::UpdateSubject(
                ghost,
                synth::subject(rng, &pools.period_ids, true).parameters,
            ),
            _ => SubjectsUpdateOp::UpdatePeriodStatus(
                ghost,
                some_period(rng, pools),
                rng.random_bool(0.5),
            ),
        };
    }

    let n = pools.subject_ids.len();
    let add_w = if n < 6 { 6 } else { 2 };
    let live_w = if n > 0 { 3 } else { 0 };
    let move_w = if n > 0 { 1 } else { 0 };
    let status_w = if n > 0 && !pools.period_ids.is_empty() {
        3
    } else {
        0
    };

    match weighted(rng, &[add_w, live_w, live_w, move_w, move_w, status_w]) {
        0 => {
            let with_interrogation = rng.random_bool(0.75);
            SubjectsUpdateOp::AddNewSubject(
                synth::subject(rng, &pools.period_ids, with_interrogation).parameters,
            )
        }
        1 => {
            let subject_id = pick(rng, &pools.subject_ids);
            // Interrogation-ness is deliberately re-drawn: turning it off is
            // what makes the cascade drop the subject's slots, its balancing
            // options and its group-list associations.
            let with_interrogation = rng.random_bool(0.6);
            SubjectsUpdateOp::UpdateSubject(
                subject_id,
                synth::subject(rng, &pools.period_ids, with_interrogation).parameters,
            )
        }
        2 => SubjectsUpdateOp::DeleteSubject(pick(rng, &pools.subject_ids)),
        3 => SubjectsUpdateOp::MoveSubjectUp(pick(rng, &pools.subject_ids)),
        4 => SubjectsUpdateOp::MoveSubjectDown(pick(rng, &pools.subject_ids)),
        _ => SubjectsUpdateOp::UpdatePeriodStatus(
            pick(rng, &pools.subject_ids),
            pick(rng, &pools.period_ids),
            rng.random_bool(0.5),
        ),
    }
}

fn gen_teachers(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> TeachersUpdateOp {
    if invalid {
        let ghost = unsafe { TeacherId::new(dangling(rng)) };
        return if rng.random_bool(0.5) {
            TeachersUpdateOp::DeleteTeacher(ghost)
        } else {
            // A teacher who declares a subject that does not exist.
            let mut teacher = synth::teacher(rng, &pools.interrogation_subject_ids);
            teacher
                .subjects
                .insert(unsafe { SubjectId::new(dangling(rng)) });
            TeachersUpdateOp::AddNewTeacher(teacher)
        };
    }

    let n = pools.teacher_ids.len();
    let add_w = if n < 5 { 5 } else { 2 };
    let live_w = if n > 0 { 3 } else { 0 };

    match weighted(rng, &[add_w, live_w, live_w]) {
        0 => TeachersUpdateOp::AddNewTeacher(synth::teacher(rng, &pools.interrogation_subject_ids)),
        1 => TeachersUpdateOp::UpdateTeacher(
            pick(rng, &pools.teacher_ids),
            synth::teacher(rng, &pools.interrogation_subject_ids),
        ),
        _ => TeachersUpdateOp::DeleteTeacher(pick(rng, &pools.teacher_ids)),
    }
}

fn gen_students(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> StudentsUpdateOp {
    if invalid {
        let ghost = unsafe { StudentId::new(dangling(rng)) };
        return if rng.random_bool(0.5) {
            StudentsUpdateOp::DeleteStudent(ghost)
        } else {
            let mut student = synth::student(rng, &pools.period_ids);
            student
                .excluded_periods
                .insert(unsafe { PeriodId::new(dangling(rng)) });
            StudentsUpdateOp::AddNewStudent(student)
        };
    }

    let n = pools.student_ids.len();
    let add_w = if n < 10 { 6 } else { 2 };
    let live_w = if n > 0 { 3 } else { 0 };

    match weighted(rng, &[add_w, live_w, live_w]) {
        0 => StudentsUpdateOp::AddNewStudent(synth::student(rng, &pools.period_ids)),
        1 => StudentsUpdateOp::UpdateStudent(
            pick(rng, &pools.student_ids),
            synth::student(rng, &pools.period_ids),
        ),
        _ => StudentsUpdateOp::DeleteStudent(pick(rng, &pools.student_ids)),
    }
}

fn gen_assignments(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> AssignmentsUpdateOp {
    if invalid {
        return AssignmentsUpdateOp::Assign(
            some_period(rng, pools),
            unsafe { StudentId::new(dangling(rng)) },
            some_subject(rng, pools),
            rng.random_bool(0.5),
        );
    }

    match weighted(rng, &[6, 2, 3]) {
        0 => AssignmentsUpdateOp::Assign(
            some_period(rng, pools),
            some_student(rng, pools),
            some_subject(rng, pools),
            rng.random_bool(0.6),
        ),
        1 => AssignmentsUpdateOp::DuplicatePreviousPeriod(some_period(rng, pools)),
        _ => AssignmentsUpdateOp::AssignAll(
            some_period(rng, pools),
            some_subject(rng, pools),
            rng.random_bool(0.6),
        ),
    }
}

fn gen_week_patterns(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> WeekPatternsUpdateOp {
    if invalid {
        let ghost_week = unsafe { WeekId::new(dangling(rng)) };
        return if rng.random_bool(0.5) {
            WeekPatternsUpdateOp::DeleteWeekPattern(unsafe { WeekPatternId::new(dangling(rng)) })
        } else {
            WeekPatternsUpdateOp::AddNewWeekPattern(synth::week_pattern_excluding(rng, ghost_week))
        };
    }

    let n = pools.week_pattern_ids.len();
    let add_w = if n < 4 { 5 } else { 2 };
    let live_w = if n > 0 { 3 } else { 0 };

    match weighted(rng, &[add_w, live_w, live_w]) {
        0 => WeekPatternsUpdateOp::AddNewWeekPattern(synth::week_pattern(rng, &pools.week_ids)),
        1 => WeekPatternsUpdateOp::UpdateWeekPattern(
            pick(rng, &pools.week_pattern_ids),
            synth::week_pattern(rng, &pools.week_ids),
        ),
        _ => WeekPatternsUpdateOp::DeleteWeekPattern(pick(rng, &pools.week_pattern_ids)),
    }
}

fn gen_slots(
    rng: &mut ChaCha8Rng,
    inner: &InnerData,
    pools: &Pools,
    invalid: bool,
) -> SlotsUpdateOp {
    let addable = addable_slot_subjects(inner, pools);

    if invalid {
        let ghost = unsafe { SlotId::new(dangling(rng)) };
        return match rng.random_range(0..3) {
            0 => SlotsUpdateOp::DeleteSlot(ghost),
            1 => SlotsUpdateOp::MoveSlotDown(ghost),
            // A slot on a live subject, but taught by nobody.
            _ => {
                let subject_id = some_interrogation_subject(rng, pools);
                let teacher_id = unsafe { TeacherId::new(dangling(rng)) };
                SlotsUpdateOp::AddNewSlot(
                    subject_id,
                    synth::slot(rng, subject_id, teacher_id, &pools.week_pattern_ids),
                )
            }
        };
    }

    let n = pools.slot_ids.len();
    let add_w = if addable.is_empty() {
        0
    } else if n < 8 {
        6
    } else {
        2
    };
    let update_w = if n > 0 { 3 } else { 0 };
    let delete_w = if n > 0 { 3 } else { 0 };
    let move_w = if n > 0 { 1 } else { 0 };

    if add_w + update_w + delete_w + move_w == 0 {
        // No slot to touch and no subject a slot could be added to (the walk
        // deleted the last one that had interrogations *and* a teacher). A dead
        // address is the only op this family can express here.
        return SlotsUpdateOp::DeleteSlot(unsafe { SlotId::new(dangling(rng)) });
    }

    match weighted(rng, &[add_w, update_w, delete_w, move_w, move_w]) {
        0 => {
            let subject_id = pick(rng, &addable);
            let teacher_id = pick(rng, &teachers_for_subject(inner, subject_id));
            SlotsUpdateOp::AddNewSlot(
                subject_id,
                synth::slot(rng, subject_id, teacher_id, &pools.week_pattern_ids),
            )
        }
        1 => {
            let slot_id = pick(rng, &pools.slot_ids);
            let (subject_id, _pos) = inner
                .params
                .slots
                .find_slot_subject_and_position(slot_id)
                .expect("the slot id comes from the live pool");
            let teachers = teachers_for_subject(inner, subject_id);
            let teacher_id = if teachers.is_empty() {
                // Nobody teaches the subject anymore: the draw is an invalid
                // one, which the family answers with its own error.
                unsafe { TeacherId::new(dangling(rng)) }
            } else {
                pick(rng, &teachers)
            };
            SlotsUpdateOp::UpdateSlot(
                slot_id,
                synth::slot(rng, subject_id, teacher_id, &pools.week_pattern_ids),
            )
        }
        2 => SlotsUpdateOp::DeleteSlot(pick(rng, &pools.slot_ids)),
        3 => SlotsUpdateOp::MoveSlotUp(pick(rng, &pools.slot_ids)),
        _ => SlotsUpdateOp::MoveSlotDown(pick(rng, &pools.slot_ids)),
    }
}

fn gen_incompatibilities(
    rng: &mut ChaCha8Rng,
    pools: &Pools,
    invalid: bool,
) -> IncompatibilitiesUpdateOp {
    if invalid {
        return if rng.random_bool(0.5) {
            IncompatibilitiesUpdateOp::DeleteIncompat(unsafe { IncompatId::new(dangling(rng)) })
        } else {
            let ghost = unsafe { SubjectId::new(dangling(rng)) };
            IncompatibilitiesUpdateOp::AddNewIncompat(synth::incompatibility(
                rng,
                ghost,
                &pools.week_pattern_ids,
            ))
        };
    }

    let n = pools.incompat_ids.len();
    let add_w = if pools.subject_ids.is_empty() {
        0
    } else if n < 3 {
        5
    } else {
        2
    };
    let update_w = if n > 0 && !pools.subject_ids.is_empty() {
        3
    } else {
        0
    };
    let remove_w = if n > 0 { 2 } else { 0 };

    if add_w + update_w + remove_w == 0 {
        // No subject to hang an incompatibility on and none to remove.
        return IncompatibilitiesUpdateOp::DeleteIncompat(unsafe {
            IncompatId::new(dangling(rng))
        });
    }

    match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => {
            let subject_id = pick(rng, &pools.subject_ids);
            IncompatibilitiesUpdateOp::AddNewIncompat(synth::incompatibility(
                rng,
                subject_id,
                &pools.week_pattern_ids,
            ))
        }
        1 => {
            let incompat_id = pick(rng, &pools.incompat_ids);
            let subject_id = pick(rng, &pools.subject_ids);
            IncompatibilitiesUpdateOp::UpdateIncompat(
                incompat_id,
                synth::incompatibility(rng, subject_id, &pools.week_pattern_ids),
            )
        }
        _ => IncompatibilitiesUpdateOp::DeleteIncompat(pick(rng, &pools.incompat_ids)),
    }
}

fn gen_pairings(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> PairingsUpdateOp {
    // Both rule constructors reject a rule naming the same entity twice, so
    // every Add/Update draw needs two distinct subjects to build from.
    let can_build = pools.subject_ids.len() >= 2;

    if invalid {
        return if can_build && rng.random_bool(0.6) {
            let real = pick(rng, &pools.subject_ids);
            let ghost = unsafe { SubjectId::new(dangling(rng)) };
            PairingsUpdateOp::AddNewPairingRule(synth::pairing_rule(
                rng,
                real,
                ghost,
                &pools.period_ids,
            ))
        } else {
            PairingsUpdateOp::DeletePairingRule(unsafe { PairingRuleId::new(dangling(rng)) })
        };
    }

    let n = pools.pairing_rule_ids.len();
    let add_w = if !can_build {
        0
    } else if n < 3 {
        5
    } else {
        2
    };
    let update_w = if n > 0 && can_build { 3 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };

    if add_w + update_w + remove_w == 0 {
        // Nothing to build from and nothing to remove: a dead address is the
        // only op this family can express here.
        return PairingsUpdateOp::DeletePairingRule(unsafe { PairingRuleId::new(dangling(rng)) });
    }

    match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => {
            let (antecedent, consequent) = distinct_pair(rng, &pools.subject_ids);
            PairingsUpdateOp::AddNewPairingRule(synth::pairing_rule(
                rng,
                antecedent,
                consequent,
                &pools.period_ids,
            ))
        }
        1 => {
            let (antecedent, consequent) = distinct_pair(rng, &pools.subject_ids);
            PairingsUpdateOp::UpdatePairingRule(
                pick(rng, &pools.pairing_rule_ids),
                synth::pairing_rule(rng, antecedent, consequent, &pools.period_ids),
            )
        }
        _ => PairingsUpdateOp::DeletePairingRule(pick(rng, &pools.pairing_rule_ids)),
    }
}

fn gen_slot_pairings(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> SlotPairingsUpdateOp {
    let can_build = pools.slot_ids.len() >= 2;

    if invalid {
        return if !pools.slot_ids.is_empty() && rng.random_bool(0.6) {
            let real = pick(rng, &pools.slot_ids);
            let ghost = unsafe { SlotId::new(dangling(rng)) };
            SlotPairingsUpdateOp::AddNewSlotPairingRule(synth::slot_pairing_rule(
                rng,
                real,
                ghost,
                &pools.period_ids,
            ))
        } else {
            SlotPairingsUpdateOp::DeleteSlotPairingRule(unsafe {
                SlotPairingRuleId::new(dangling(rng))
            })
        };
    }

    let n = pools.slot_pairing_rule_ids.len();
    let add_w = if !can_build {
        0
    } else if n < 3 {
        5
    } else {
        2
    };
    let update_w = if n > 0 && can_build { 3 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };

    if add_w + update_w + remove_w == 0 {
        return SlotPairingsUpdateOp::DeleteSlotPairingRule(unsafe {
            SlotPairingRuleId::new(dangling(rng))
        });
    }

    match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => {
            let (antecedent, consequent) = distinct_pair(rng, &pools.slot_ids);
            SlotPairingsUpdateOp::AddNewSlotPairingRule(synth::slot_pairing_rule(
                rng,
                antecedent,
                consequent,
                &pools.period_ids,
            ))
        }
        1 => {
            let (antecedent, consequent) = distinct_pair(rng, &pools.slot_ids);
            SlotPairingsUpdateOp::UpdateSlotPairingRule(
                pick(rng, &pools.slot_pairing_rule_ids),
                synth::slot_pairing_rule(rng, antecedent, consequent, &pools.period_ids),
            )
        }
        _ => SlotPairingsUpdateOp::DeleteSlotPairingRule(pick(rng, &pools.slot_pairing_rule_ids)),
    }
}

fn gen_group_lists(
    rng: &mut ChaCha8Rng,
    inner: &InnerData,
    pools: &Pools,
    invalid: bool,
) -> GroupListsUpdateOp {
    if invalid {
        let ghost = unsafe { GroupListId::new(dangling(rng)) };
        return if rng.random_bool(0.5) {
            GroupListsUpdateOp::DeleteGroupList(ghost)
        } else {
            GroupListsUpdateOp::AssignGroupListToSubject(
                some_period(rng, pools),
                some_subject(rng, pools),
                Some(ghost),
            )
        };
    }

    let n = pools.group_list_ids.len();
    let add_w = if n < 4 { 5 } else { 2 };
    let update_w = if n > 0 { 4 } else { 0 };
    let assign_w = if !pools.period_ids.is_empty() && !pools.subject_ids.is_empty() {
        4
    } else {
        0
    };
    let remove_w = if n > 0 { 3 } else { 0 };
    let duplicate_w = if pools.period_ids.is_empty() { 0 } else { 2 };

    match weighted(rng, &[add_w, update_w, assign_w, remove_w, duplicate_w]) {
        0 => {
            let group_count = rng.random_range(2..=5);
            let params = synth::group_list_parameters(rng, group_count);
            let filling = if rng.random_bool(0.5) {
                synth::prefilled_filling(rng, group_count, &pools.student_ids)
            } else {
                synth::automatic_filling(rng, &pools.student_ids)
            };
            GroupListsUpdateOp::AddNewGroupList(
                GroupList::new(params, filling).expect("the group count matches the parameters"),
            )
        }
        1 => {
            // The whole list travels in one payload (parameters *and* filling),
            // so the shrink is expressed by simply drawing fewer groups — the
            // dropped students are the caller's own edit.
            let group_list_id = pick(rng, &pools.group_list_ids);
            let current = inner
                .params
                .group_lists
                .group_list_map
                .get(&group_list_id)
                .expect("the group list id comes from the live pool");
            let group_count = match current.filling() {
                GroupListFilling::Prefilled { groups } => {
                    if rng.random_bool(0.4) {
                        rng.random_range(1..=groups.len().max(1))
                    } else {
                        groups.len()
                    }
                }
                GroupListFilling::Automatic { .. } => rng.random_range(1..=5),
            };
            let params = synth::group_list_parameters(rng, group_count);
            let filling = if rng.random_bool(0.5) {
                synth::prefilled_filling(rng, group_count, &pools.student_ids)
            } else {
                synth::automatic_filling(rng, &pools.student_ids)
            };
            GroupListsUpdateOp::UpdateGroupList(
                group_list_id,
                GroupList::new(params, filling).expect("the group count matches the parameters"),
            )
        }
        2 => GroupListsUpdateOp::AssignGroupListToSubject(
            pick(rng, &pools.period_ids),
            some_interrogation_subject(rng, pools),
            (n > 0 && rng.random_bool(0.8)).then(|| pick(rng, &pools.group_list_ids)),
        ),
        3 => GroupListsUpdateOp::DeleteGroupList(pick(rng, &pools.group_list_ids)),
        _ => GroupListsUpdateOp::DuplicatePreviousPeriod(pick(rng, &pools.period_ids)),
    }
}

fn gen_settings(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> SettingsUpdateOp {
    if invalid {
        return SettingsUpdateOp::UpdateStudentLimits(
            unsafe { StudentId::new(dangling(rng)) },
            synth::limits(rng),
        );
    }

    let set_w = if pools.student_ids.is_empty() { 0 } else { 4 };
    let remove_w = if pools.student_ids.is_empty() { 0 } else { 2 };

    match weighted(rng, &[3, set_w, remove_w]) {
        0 => SettingsUpdateOp::UpdateGlobalLimits(synth::limits(rng)),
        1 => {
            SettingsUpdateOp::UpdateStudentLimits(pick(rng, &pools.student_ids), synth::limits(rng))
        }
        _ => SettingsUpdateOp::RemoveStudentLimits(pick(rng, &pools.student_ids)),
    }
}

fn gen_balancing(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> BalancingUpdateOp {
    if invalid {
        return BalancingUpdateOp::UpdateSubjectOptions(
            unsafe { SubjectId::new(dangling(rng)) },
            synth::balancing_options(rng),
        );
    }

    let live_w = if pools.interrogation_subject_ids.is_empty() {
        0
    } else {
        4
    };

    match weighted(rng, &[3, live_w, live_w / 2]) {
        0 => BalancingUpdateOp::UpdateGlobalOptions(synth::balancing_options(rng)),
        1 => BalancingUpdateOp::UpdateSubjectOptions(
            pick(rng, &pools.interrogation_subject_ids),
            synth::balancing_options(rng),
        ),
        _ => BalancingUpdateOp::RemoveSubjectOptions(pick(rng, &pools.interrogation_subject_ids)),
    }
}

fn gen_colloscope(
    rng: &mut ChaCha8Rng,
    inner: &InnerData,
    pools: &Pools,
    invalid: bool,
) -> ColloscopeUpdateOp {
    if invalid {
        return if rng.random_bool(0.5) {
            ColloscopeUpdateOp::UpdateColloscopeGroupList(
                some_group_list(rng, pools),
                BTreeMap::from([(unsafe { StudentId::new(dangling(rng)) }, 0)]),
            )
        } else {
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                unsafe { SlotId::new(dangling(rng)) },
                unsafe { WeekId::new(dangling(rng)) },
                BTreeSet::from([0]),
            )
        };
    }

    let group_list_w = if pools.group_list_ids.is_empty() {
        0
    } else {
        5
    };
    let interrogation_w = if pools.slot_ids.is_empty() || pools.week_ids.is_empty() {
        0
    } else {
        5
    };

    match weighted(rng, &[group_list_w, interrogation_w, 1, 1]) {
        0 => {
            let group_list_id = pick(rng, &pools.group_list_ids);
            let group_list = inner
                .params
                .group_lists
                .group_list_map
                .get(&group_list_id)
                .expect("the group list id comes from the live pool");
            let group_count = group_list.params().group_names.len() as u32;
            let mut placements = BTreeMap::new();
            if group_count > 0 {
                for student_id in synth::subset(rng, &pools.student_ids, 0.5) {
                    placements.insert(student_id, rng.random_range(0..group_count));
                }
            }
            ColloscopeUpdateOp::UpdateColloscopeGroupList(group_list_id, placements)
        }
        1 => {
            let slot_id = pick(rng, &pools.slot_ids);
            let week_id = pick(rng, &pools.week_ids);
            let mut assigned_groups = BTreeSet::new();
            for _ in 0..rng.random_range(0..=2u32) {
                assigned_groups.insert(rng.random_range(0..3u32));
            }
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(slot_id, week_id, assigned_groups)
        }
        2 => ColloscopeUpdateOp::EraseColloscope,
        _ => ColloscopeUpdateOp::EraseGroupLists,
    }
}

fn gen_export_config(rng: &mut ChaCha8Rng) -> ExportConfigUpdateOp {
    match rng.random_range(0..11) {
        0 => ExportConfigUpdateOp::UpdateGlobalConfig(synth::global_config(rng)),
        1 => ExportConfigUpdateOp::UpdateColloscopeEnabled(rng.random_bool(0.5)),
        2 => ExportConfigUpdateOp::UpdateAllGroupsEnabled(rng.random_bool(0.5)),
        3 => ExportConfigUpdateOp::UpdatePrefilledGroupsEnabled(rng.random_bool(0.5)),
        4 => ExportConfigUpdateOp::UpdateAutomaticGroupsEnabled(rng.random_bool(0.5)),
        5 => ExportConfigUpdateOp::UpdatePerGroupListEnabled(rng.random_bool(0.5)),
        6 => ExportConfigUpdateOp::UpdateColloscopeConfig(synth::colloscope_config(rng)),
        7 => ExportConfigUpdateOp::UpdateAllGroupsConfig(synth::per_student_groups_config(rng)),
        8 => {
            ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(synth::per_student_groups_config(rng))
        }
        9 => {
            ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(synth::per_student_groups_config(rng))
        }
        _ => ExportConfigUpdateOp::UpdatePerGroupListConfig(synth::per_group_list_config(rng)),
    }
}

// ============================================================================
// The walk
// ============================================================================

/// Cross-seed tallies. The seed body is called from inside `catch_unwind`, so
/// these need interior mutability.
#[derive(Default)]
struct Counters {
    landed: Cell<usize>,   // ops whose whole composite landed
    warned: Cell<usize>,   // landings that made the cascade repair something
    warnings: Cell<usize>, // repairs in total
    errored: Cell<usize>,  // ops the ops layer or the engine rejected
    /// Per family: ops drawn, and ops that landed. Printed rather than
    /// asserted (beyond "was it drawn at all"): the numbers are what tells a
    /// reader the walk did not degenerate into one family's error loop.
    per_family: RefCell<BTreeMap<&'static str, (usize, usize)>>,
}

impl Counters {
    fn bump(cell: &Cell<usize>, by: usize) {
        cell.set(cell.get() + by);
    }

    fn record(&self, family: &'static str, landed: bool) {
        let mut per_family = self.per_family.borrow_mut();
        let entry = per_family.entry(family).or_default();
        entry.0 += 1;
        if landed {
            entry.1 += 1;
        }
    }

    /// The guards that make a green run mean something.
    fn assert_covered(&self) {
        assert!(
            self.landed.get() > 0,
            "no op ever landed across all seeds — the walk asserted nothing about \
             the state the new path produces",
        );
        assert!(
            self.warned.get() > 0,
            "no op ever made the cascade repair anything across all seeds — the \
             composites were only ever exercised on documents where nothing had \
             to be cleaned up, so a green run here proves nothing",
        );
        assert!(
            self.errored.get() > 0,
            "no op was ever rejected across all seeds — the error translations \
             were never reached",
        );
        let per_family = self.per_family.borrow();
        for family in FAMILIES {
            let (attempted, landed) = per_family.get(family).copied().unwrap_or_default();
            assert!(
                attempted > 0,
                "family `{family}` was never attempted — its bodies are unfuzzed",
            );
            assert!(
                landed > 0,
                "family `{family}` never landed an op — it was drawn {attempted} \
                 times and rejected every time, so only its address checks were \
                 fuzzed and its bodies were not",
            );
        }
    }

    fn report(&self) {
        eprintln!(
            "update-op fuzz: {} landed ({} with repairs, {} repairs in total), {} rejected",
            self.landed.get(),
            self.warned.get(),
            self.warnings.get(),
            self.errored.get(),
        );
        for (family, (attempted, landed)) in self.per_family.borrow().iter() {
            eprintln!("  {family}: {landed} landed / {attempted} drawn");
        }
    }
}

/// The invariant oracle: what the new path commits must satisfy the whole-model
/// checker.
fn assert_clean(data: &Data) {
    assert_eq!(
        data.get_inner_data().broken_invariants(),
        Ok(BTreeSet::new()),
        "cascade_dry_apply returned Ok but the committed document is not valid",
    );
}

/// A document grown through the elementary gated walk, re-homed onto this
/// crate's `Desc`. [`harness::bootstrap`] hands back an `AppState<Data, String>`
/// and its own history; only the document is kept.
fn bootstrap(rng: &mut ChaCha8Rng) -> AppState<Data, Desc> {
    let (state, _snapshots) = harness::bootstrap(rng);

    AppState::new(state.get_data().clone())
}

#[test]
fn update_ops_never_panic_and_land_valid() {
    let counters = Counters::default();
    let start = std::time::Instant::now();

    for seed in 0..CONFIG.seeds {
        let log: RefCell<Vec<String>> = RefCell::new(Vec::new());

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let mut state = bootstrap(&mut rng);
            assert_clean(state.get_data());

            for _ in 0..CONFIG.ops_per_run {
                let (family, op) =
                    gen_update_op(&mut rng, state.get_data(), CONFIG.invalid_fraction);

                let mut entry = format!("{op:?}");
                if entry.len() > 400 {
                    entry.truncate(400);
                    entry.push('…');
                }
                let position = log.borrow().len();
                log.borrow_mut()
                    .push(format!("[{position}] {family}: {entry}"));

                match op.cascade_dry_apply(&state) {
                    Ok(result) => {
                        assert_clean(result.new_state.get_data());
                        Counters::bump(&counters.landed, 1);
                        if !result.warnings.is_empty() {
                            Counters::bump(&counters.warned, 1);
                            Counters::bump(&counters.warnings, result.warnings.len());
                        }
                        counters.record(family, true);
                        state = result.new_state;
                    }
                    // A clean rejection is a legitimate outcome — the op named a
                    // dead address, or the cascade convicted its target.
                    Err(_) => {
                        Counters::bump(&counters.errored, 1);
                        counters.record(family, false);
                    }
                }
            }
        }));

        if let Err(payload) = result {
            let log = log.borrow();
            eprintln!(
                "update-op fuzz: seed {seed} FAILED after {} generated ops. Op log:\n{}",
                log.len(),
                log.join("\n"),
            );
            std::panic::resume_unwind(payload);
        }
    }

    counters.assert_covered();
    counters.report();
    eprintln!(
        "update-op fuzz: {} seeds × {} ops in {:.2?}",
        CONFIG.seeds,
        CONFIG.ops_per_run,
        start.elapsed(),
    );
}
