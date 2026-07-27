//! Random operation generator for the property tests
//!
//! Pools of valid ids are read live from the current [InnerData] before
//! every operation: the state itself is the source of truth. The generator
//! covers all 16 [Op] categories, favors Add-flavored ops while pools are
//! small, and deliberately breaks one constraint with probability
//! `invalid_fraction`. The harness never predicts whether an op will
//! succeed: it asserts the right property in either case.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};

use collomatique_state_colloscopes::{
    AssignmentOp, BalancingOp, ColloscopeOp, ExportConfigOp, GroupListOp, IncompatOp, InnerData,
    Op, PairingOp, PeriodOp, SettingsOp, SlotOp, SlotPairingOp, StudentOp, SubjectOp, TeacherOp,
    WeekOp, WeekPatternOp,
    group_lists::{GroupList, GroupListFilling, PrefilledGroup},
    ids::{
        GroupListId, Id, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
        SubjectId, TeacherId, WeekId, WeekPatternId,
    },
    students::Student,
    weeks::WeekDesc,
};

use crate::synth;
use crate::synth::pick;

/// All op categories, used for coverage tracking
pub const CATEGORIES: [&str; 17] = [
    "student",
    "period",
    "week",
    "subject",
    "teacher",
    "assignment",
    "week_pattern",
    "slot",
    "incompat",
    "group_list",
    "settings",
    "pairing",
    "slot_pairing",
    "balancing",
    "colloscope",
    "export_config",
    "global_update",
];

/// Base for ids that are guaranteed dangling
///
/// Real ids are issued sequentially from 0 and never reach this range
/// (the only op that could advance the issuer this far is a corrupted
/// GlobalUpdate, and those are built from duplicated ids instead).
const DANGLING_BASE: u64 = 1 << 40;

fn dangling(rng: &mut ChaCha8Rng) -> u64 {
    DANGLING_BASE + rng.random_range(0..1_000_000)
}

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

/// Id pools extracted from the current state
struct Pools {
    period_ids: Vec<PeriodId>,
    week_ids: Vec<WeekId>,
    student_ids: Vec<StudentId>,
    subject_ids: Vec<SubjectId>,
    interrogation_subject_ids: Vec<SubjectId>,
    non_interrogation_subject_ids: Vec<SubjectId>,
    teacher_ids: Vec<TeacherId>,
    week_pattern_ids: Vec<WeekPatternId>,
    /// Subjects that own at least one slot, with their slots
    slots_by_subject: Vec<(SubjectId, Vec<SlotId>)>,
    slot_ids: Vec<SlotId>,
    incompat_ids: Vec<IncompatId>,
    group_list_ids: Vec<GroupListId>,
    pairing_rule_ids: Vec<PairingRuleId>,
    slot_pairing_rule_ids: Vec<SlotPairingRuleId>,
    /// Non-prefilled group lists registered in the colloscope
    colloscope_group_list_ids: Vec<GroupListId>,
    /// (period, slot, weeks-in-period holding an interrogation)
    colloscope_targets: Vec<(PeriodId, SlotId, Vec<usize>)>,
}

impl Pools {
    fn extract(inner: &InnerData) -> Pools {
        let params = &inner.params;
        let period_ids: Vec<_> = params.periods.period_ids().collect();
        let week_ids: Vec<_> = params.week_ids().collect();
        let subject_ids: Vec<_> = params
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(id, _)| id)
            .collect();
        let interrogation_subject_ids: Vec<_> = params
            .subjects
            .ordered_subject_list
            .iter()
            .filter(|(_, s)| s.parameters.interrogation_parameters.is_some())
            .map(|(id, _)| id)
            .collect();
        let non_interrogation_subject_ids: Vec<_> = params
            .subjects
            .ordered_subject_list
            .iter()
            .filter(|(_, s)| s.parameters.interrogation_parameters.is_none())
            .map(|(id, _)| id)
            .collect();
        let slots_by_subject: Vec<_> = params
            .slots
            .subjects_with_slots()
            .filter_map(|subject_id| {
                let slot_ids: Vec<SlotId> = params
                    .slots
                    .slots_for_subject(subject_id)
                    .expect("subject comes from subjects_with_slots")
                    .map(|(id, _)| *id)
                    .collect();
                if slot_ids.is_empty() {
                    None
                } else {
                    Some((subject_id, slot_ids))
                }
            })
            .collect();
        let slot_ids: Vec<_> = slots_by_subject
            .iter()
            .flat_map(|(_, slots): &(_, Vec<SlotId>)| slots.iter().copied())
            .collect();
        // Possible interrogation cells, re-derived from the parameters: for each
        // period × slot, the positional week indices where an interrogation can
        // happen (`is_interrogation_possible` mirrors the dense skeleton's
        // Some-cell rule). A slot whose subject does not run on a period yields
        // no possible weeks and is dropped, matching the old skeleton walk.
        let mut colloscope_targets: Vec<(PeriodId, SlotId, Vec<usize>)> = Vec::new();
        for period_id in params.periods.period_ids() {
            let week_ids: Vec<WeekId> = params
                .weeks
                .weeks_for_period(period_id)
                .into_iter()
                .flatten()
                .map(|(week_id, _week)| *week_id)
                .collect();
            for (slot_id, _slot) in params.slots.all_slots() {
                let weeks: Vec<usize> = week_ids
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &week_id)| {
                        params
                            .is_interrogation_possible(*slot_id, week_id)
                            .then_some(i)
                    })
                    .collect();
                if !weeks.is_empty() {
                    colloscope_targets.push((period_id, *slot_id, weeks));
                }
            }
        }

        Pools {
            period_ids,
            week_ids,
            student_ids: params.students.student_map.keys().collect(),
            subject_ids,
            interrogation_subject_ids,
            non_interrogation_subject_ids,
            teacher_ids: params.teachers.teacher_map.keys().collect(),
            week_pattern_ids: params.week_patterns.week_pattern_map.keys().collect(),
            slots_by_subject,
            slot_ids,
            incompat_ids: params.incompats.incompat_map.keys().collect(),
            group_list_ids: params.group_lists.group_list_map.keys().collect(),
            pairing_rule_ids: params.pairings.pairing_rule_map.keys().collect(),
            slot_pairing_rule_ids: params.slot_pairings.slot_pairing_rule_map.keys().collect(),
            colloscope_group_list_ids: params
                .group_lists
                .group_list_map
                .iter()
                .filter(|(_id, group_list)| !group_list.is_prefilled())
                .map(|(id, _)| id)
                .collect(),
            colloscope_targets,
        }
    }
}

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

/// Interrogation subjects for which a slot can currently be added
fn addable_slot_subjects(inner: &InnerData, pools: &Pools) -> Vec<SubjectId> {
    pools
        .interrogation_subject_ids
        .iter()
        .copied()
        .filter(|subject_id| !teachers_for_subject(inner, *subject_id).is_empty())
        .collect()
}

/// Generates the next operation from the current state
///
/// `snapshots` are earlier valid [InnerData] states of this same run,
/// used as replay material for GlobalUpdate ops.
pub fn gen_op(
    rng: &mut ChaCha8Rng,
    inner: &InnerData,
    snapshots: &[InnerData],
    invalid_fraction: f64,
) -> (&'static str, Op) {
    let pools = Pools::extract(inner);
    let invalid = rng.random_bool(invalid_fraction);

    let can_pair_slots = pools
        .slots_by_subject
        .iter()
        .any(|(_, slots)| slots.len() >= 2);
    let eligible: Vec<(&'static str, u32)> = [
        ("student", 8u32),
        ("period", 5),
        ("week", 6),
        ("subject", 8),
        ("teacher", 6),
        ("assignment", 8),
        ("week_pattern", 5),
        ("slot", 8),
        ("incompat", 4),
        ("group_list", 8),
        ("settings", 3),
        ("pairing", 4),
        ("slot_pairing", 3),
        ("balancing", 3),
        ("colloscope", 6),
        ("export_config", 3),
        ("global_update", 2),
    ]
    .into_iter()
    .filter(|(name, _)| match *name {
        // AddFront needs a period; every other week op needs an existing week
        // (there always is one when a period exists — periods are non-empty).
        "week" => !pools.period_ids.is_empty(),
        "assignment" => {
            !pools.period_ids.is_empty()
                && !pools.student_ids.is_empty()
                && !pools.subject_ids.is_empty()
        }
        "slot" => !addable_slot_subjects(inner, &pools).is_empty() || !pools.slot_ids.is_empty(),
        "incompat" => !pools.subject_ids.is_empty() || !pools.incompat_ids.is_empty(),
        "pairing" => pools.subject_ids.len() >= 2 || !pools.pairing_rule_ids.is_empty(),
        "slot_pairing" => can_pair_slots || !pools.slot_pairing_rule_ids.is_empty(),
        "colloscope" => {
            !pools.colloscope_group_list_ids.is_empty() || !pools.colloscope_targets.is_empty()
        }
        _ => true,
    })
    .collect();

    let weights: Vec<u32> = eligible.iter().map(|(_, w)| *w).collect();
    let category = eligible[weighted(rng, &weights)].0;

    let op = match category {
        "student" => gen_student(rng, &pools, invalid),
        "period" => gen_period(rng, &pools, invalid),
        "week" => gen_week(rng, &pools, invalid),
        "subject" => gen_subject(rng, inner, &pools, invalid),
        "teacher" => gen_teacher(rng, &pools, invalid),
        "assignment" => gen_assignment(rng, inner, &pools, invalid),
        "week_pattern" => gen_week_pattern(rng, &pools, invalid),
        "slot" => gen_slot(rng, inner, &pools, invalid),
        "incompat" => gen_incompat(rng, &pools, invalid),
        "group_list" => gen_group_list(rng, inner, &pools, invalid),
        "settings" => gen_settings(rng, &pools, invalid),
        "pairing" => gen_pairing(rng, &pools, invalid),
        "slot_pairing" => gen_slot_pairing(rng, &pools, invalid),
        "balancing" => gen_balancing(rng, &pools, invalid),
        "colloscope" => gen_colloscope(rng, inner, &pools, invalid),
        "export_config" => gen_export_config(rng),
        "global_update" => gen_global_update(rng, inner, &pools, snapshots, invalid),
        _ => unreachable!(),
    };

    (category, op)
}

fn gen_student(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        let op = if rng.random_bool(0.5) {
            StudentOp::Remove(unsafe { StudentId::new(dangling(rng)) })
        } else {
            StudentOp::Add(Student {
                desc: synth::person(rng),
                excluded_periods: BTreeSet::from([unsafe { PeriodId::new(dangling(rng)) }]),
            })
        };
        return Op::Student(op);
    }
    let n = pools.student_ids.len();
    let add_w = if n < 10 { 6 } else { 2 };
    let update_w = if n > 0 { 3 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };
    let op = match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => StudentOp::Add(synth::student(rng, &pools.period_ids)),
        1 => StudentOp::Update(
            pick(rng, &pools.student_ids),
            synth::student(rng, &pools.period_ids),
        ),
        _ => StudentOp::Remove(pick(rng, &pools.student_ids)),
    };
    Op::Student(op)
}

fn gen_period(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        let op = if rng.random_bool(0.5) {
            PeriodOp::Remove(unsafe { PeriodId::new(dangling(rng)) })
        } else {
            PeriodOp::AddAfter(unsafe { PeriodId::new(dangling(rng)) })
        };
        return Op::Period(op);
    }
    let n = pools.period_ids.len();
    let add_w = if n < 4 { 4 } else { 1 };
    let remove_w = if n > 0 { 2 } else { 0 };
    // Periods are created empty (weeks are spliced in by the WeekOp family,
    // driven by `gen_week`). This is the valid walk, applied through the gate:
    // removing a week-non-empty period bounces as `Error::BrokenInvariants` (the
    // weeks' `period_id` FKs would dangle) — a legitimate error the harness
    // tolerates like any other. (The force path lands those dangles; the
    // corruption arm exploits it to reach the dangling landing — see
    // `gen_corruption_op`.)
    let op = match weighted(rng, &[2, add_w, remove_w]) {
        0 => PeriodOp::ChangeStartDate(if rng.random_bool(0.7) {
            Some(synth::week_start(rng))
        } else {
            None
        }),
        1 => {
            if pools.period_ids.is_empty() || rng.random_bool(0.3) {
                PeriodOp::AddFront
            } else {
                PeriodOp::AddAfter(pick(rng, &pools.period_ids))
            }
        }
        _ => PeriodOp::Remove(pick(rng, &pools.period_ids)),
    };
    Op::Period(op)
}

fn gen_week(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        let op = match rng.random_range(0..3) {
            0 => WeekOp::AddFront(
                unsafe { PeriodId::new(dangling(rng)) },
                synth::week_desc(rng),
            ),
            1 => WeekOp::Remove(unsafe { WeekId::new(dangling(rng)) }),
            _ => WeekOp::Move(
                unsafe { WeekId::new(dangling(rng)) },
                pick(rng, &pools.period_ids),
                rng.random_range(0..3),
            ),
        };
        return Op::Week(op);
    }
    let has_weeks = !pools.week_ids.is_empty();
    let add_front_w = 4u32;
    let add_after_w = if has_weeks { 4 } else { 0 };
    let update_w = if has_weeks { 3 } else { 0 };
    let move_w = if has_weeks { 2 } else { 0 };
    let remove_w = if has_weeks { 2 } else { 0 };
    let op = match weighted(rng, &[add_front_w, add_after_w, update_w, move_w, remove_w]) {
        0 => WeekOp::AddFront(pick(rng, &pools.period_ids), synth::week_desc(rng)),
        1 => WeekOp::AddAfter(pick(rng, &pools.week_ids), synth::week_desc(rng)),
        2 => WeekOp::Update(pick(rng, &pools.week_ids), synth::week_desc(rng)),
        3 => WeekOp::Move(
            pick(rng, &pools.week_ids),
            pick(rng, &pools.period_ids),
            rng.random_range(0..3),
        ),
        _ => WeekOp::Remove(pick(rng, &pools.week_ids)),
    };
    Op::Week(op)
}

fn gen_subject(rng: &mut ChaCha8Rng, inner: &InnerData, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        let op = match rng.random_range(0..2) {
            0 if !pools.subject_ids.is_empty() => SubjectOp::ChangePosition(
                pick(rng, &pools.subject_ids),
                pools.subject_ids.len() + 5,
            ),
            _ => SubjectOp::Remove(unsafe { SubjectId::new(dangling(rng)) }),
        };
        return Op::Subject(op);
    }
    let n = pools.subject_ids.len();
    let add_w = if n < 6 { 6 } else { 2 };
    let update_w = if n > 0 { 3 } else { 0 };
    let move_w = if n > 0 { 1 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };
    let op = match weighted(rng, &[add_w, update_w, move_w, remove_w]) {
        0 => {
            let anchor = if !pools.subject_ids.is_empty() && rng.random_bool(0.5) {
                Some(pick(rng, &pools.subject_ids))
            } else {
                None
            };
            let with_interrogation = rng.random_bool(0.75);
            SubjectOp::AddAfter(
                anchor,
                synth::subject(rng, &pools.period_ids, with_interrogation),
            )
        }
        1 => {
            let subject_id = pick(rng, &pools.subject_ids);
            // Keep the interrogation-ness stable so updates mostly succeed
            let with_interrogation = inner
                .params
                .subjects
                .find_subject(subject_id)
                .expect("Subject id comes from the live pool")
                .parameters
                .interrogation_parameters
                .is_some();
            SubjectOp::Update(
                subject_id,
                synth::subject(rng, &pools.period_ids, with_interrogation),
            )
        }
        2 => SubjectOp::ChangePosition(
            pick(rng, &pools.subject_ids),
            rng.random_range(0..pools.subject_ids.len()),
        ),
        _ => SubjectOp::Remove(pick(rng, &pools.subject_ids)),
    };
    Op::Subject(op)
}

fn gen_teacher(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        let op = if !pools.non_interrogation_subject_ids.is_empty() && rng.random_bool(0.5) {
            // A teacher can only interrogate in subjects that have interrogations
            let mut teacher = synth::teacher(rng, &pools.interrogation_subject_ids);
            teacher
                .subjects
                .insert(pick(rng, &pools.non_interrogation_subject_ids));
            TeacherOp::Add(teacher)
        } else {
            let mut teacher = synth::teacher(rng, &pools.interrogation_subject_ids);
            teacher
                .subjects
                .insert(unsafe { SubjectId::new(dangling(rng)) });
            TeacherOp::Add(teacher)
        };
        return Op::Teacher(op);
    }
    let n = pools.teacher_ids.len();
    let add_w = if n < 5 { 5 } else { 2 };
    let update_w = if n > 0 { 3 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };
    let op = match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => TeacherOp::Add(synth::teacher(rng, &pools.interrogation_subject_ids)),
        1 => TeacherOp::Update(
            pick(rng, &pools.teacher_ids),
            synth::teacher(rng, &pools.interrogation_subject_ids),
        ),
        _ => TeacherOp::Remove(pick(rng, &pools.teacher_ids)),
    };
    Op::Teacher(op)
}

fn gen_assignment(rng: &mut ChaCha8Rng, inner: &InnerData, pools: &Pools, invalid: bool) -> Op {
    let period_id = pick(rng, &pools.period_ids);
    if invalid {
        let op = AssignmentOp::Assign(
            period_id,
            unsafe { StudentId::new(dangling(rng)) },
            pick(rng, &pools.subject_ids),
            rng.random_bool(0.5),
        );
        return Op::Assignment(op);
    }
    // Prefer subject/student combinations that are actually present on the period
    let period_subjects: Vec<SubjectId> = inner
        .params
        .subjects
        .ordered_subject_list
        .iter()
        .filter(|(_, s)| !s.excluded_periods.contains(&period_id))
        .map(|(id, _)| id)
        .collect();
    let period_students: Vec<StudentId> = inner
        .params
        .students
        .student_map
        .iter()
        .filter(|(_, s)| !s.excluded_periods.contains(&period_id))
        .map(|(id, _)| id)
        .collect();
    let subject_id = if period_subjects.is_empty() {
        pick(rng, &pools.subject_ids)
    } else {
        pick(rng, &period_subjects)
    };
    let student_id = if period_students.is_empty() {
        pick(rng, &pools.student_ids)
    } else {
        pick(rng, &period_students)
    };
    Op::Assignment(AssignmentOp::Assign(
        period_id,
        student_id,
        subject_id,
        rng.random_bool(0.6),
    ))
}

fn gen_week_pattern(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        let op = match rng.random_range(0..3) {
            0 => {
                let dangling_week = unsafe { WeekId::new(dangling(rng)) };
                WeekPatternOp::Add(synth::week_pattern_excluding(rng, dangling_week))
            }
            1 if !pools.week_pattern_ids.is_empty() => {
                let dangling_week = unsafe { WeekId::new(dangling(rng)) };
                WeekPatternOp::Update(
                    pick(rng, &pools.week_pattern_ids),
                    synth::week_pattern_excluding(rng, dangling_week),
                )
            }
            _ => WeekPatternOp::Remove(unsafe { WeekPatternId::new(dangling(rng)) }),
        };
        return Op::WeekPattern(op);
    }
    let n = pools.week_pattern_ids.len();
    let add_w = if n < 4 { 5 } else { 2 };
    let update_w = if n > 0 { 3 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };
    let op = match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => WeekPatternOp::Add(synth::week_pattern(rng, &pools.week_ids)),
        1 => WeekPatternOp::Update(
            pick(rng, &pools.week_pattern_ids),
            synth::week_pattern(rng, &pools.week_ids),
        ),
        _ => WeekPatternOp::Remove(pick(rng, &pools.week_pattern_ids)),
    };
    Op::WeekPattern(op)
}

fn gen_slot(rng: &mut ChaCha8Rng, inner: &InnerData, pools: &Pools, invalid: bool) -> Op {
    let addable = addable_slot_subjects(inner, pools);
    if invalid {
        let op = match rng.random_range(0..3) {
            0 if !addable.is_empty() => {
                // Valid teacher, but a start time crossing midnight
                let subject_id = pick(rng, &addable);
                let teacher_id = pick(rng, &teachers_for_subject(inner, subject_id));
                let mut slot = synth::slot(rng, subject_id, teacher_id, &pools.week_pattern_ids);
                slot.start_time = synth::slot_start_crossing_midnight(rng);
                SlotOp::AddAfter(None, slot)
            }
            1 if !pools.interrogation_subject_ids.is_empty() => {
                // Dangling teacher
                let subject_id = pick(rng, &pools.interrogation_subject_ids);
                let teacher_id = unsafe { TeacherId::new(dangling(rng)) };
                SlotOp::AddAfter(
                    None,
                    synth::slot(rng, subject_id, teacher_id, &pools.week_pattern_ids),
                )
            }
            _ => SlotOp::Remove(unsafe { SlotId::new(dangling(rng)) }),
        };
        return Op::Slot(op);
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
    let move_w = if n > 0 { 1 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };
    let op = match weighted(rng, &[add_w, update_w, move_w, remove_w]) {
        0 => {
            let subject_id = pick(rng, &addable);
            let teacher_id = pick(rng, &teachers_for_subject(inner, subject_id));
            let anchor = pools
                .slots_by_subject
                .iter()
                .find(|(id, _)| *id == subject_id)
                .filter(|_| rng.random_bool(0.5))
                .map(|(_, slots)| pick(rng, slots));
            SlotOp::AddAfter(
                anchor,
                synth::slot(rng, subject_id, teacher_id, &pools.week_pattern_ids),
            )
        }
        1 => {
            let slot_id = pick(rng, &pools.slot_ids);
            let (subject_id, _pos) = inner
                .params
                .slots
                .find_slot_subject_and_position(slot_id)
                .expect("Slot id comes from the live pool");
            let teachers = teachers_for_subject(inner, subject_id);
            if teachers.is_empty() {
                // No valid teacher available anymore: exercise the removal path instead
                SlotOp::Remove(slot_id)
            } else {
                let teacher_id = pick(rng, &teachers);
                SlotOp::Update(
                    slot_id,
                    synth::slot(rng, subject_id, teacher_id, &pools.week_pattern_ids),
                )
            }
        }
        2 => {
            let (_, subject_slots) =
                &pools.slots_by_subject[rng.random_range(0..pools.slots_by_subject.len())];
            SlotOp::ChangePosition(
                pick(rng, subject_slots),
                rng.random_range(0..subject_slots.len()),
            )
        }
        _ => SlotOp::Remove(pick(rng, &pools.slot_ids)),
    };
    Op::Slot(op)
}

fn gen_incompat(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        let subject_id = unsafe { SubjectId::new(dangling(rng)) };
        let op = IncompatOp::Add(synth::incompatibility(
            rng,
            subject_id,
            &pools.week_pattern_ids,
        ));
        return Op::Incompat(op);
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
    let op = match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => {
            let subject_id = pick(rng, &pools.subject_ids);
            IncompatOp::Add(synth::incompatibility(
                rng,
                subject_id,
                &pools.week_pattern_ids,
            ))
        }
        1 => {
            let incompat_id = pick(rng, &pools.incompat_ids);
            let subject_id = pick(rng, &pools.subject_ids);
            IncompatOp::Update(
                incompat_id,
                synth::incompatibility(rng, subject_id, &pools.week_pattern_ids),
            )
        }
        _ => IncompatOp::Remove(pick(rng, &pools.incompat_ids)),
    };
    Op::Incompat(op)
}

fn gen_group_list(rng: &mut ChaCha8Rng, inner: &InnerData, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        // The count-mismatch case is gone: a `GroupList` is sealed, so an
        // over-counted prefilled filling is unrepresentable at the op boundary.
        let op = match rng.random_range(0..3) {
            0 if !pools.period_ids.is_empty()
                && !pools.non_interrogation_subject_ids.is_empty() =>
            {
                // Associating a group list to a subject without interrogations
                GroupListOp::AssignToSubject(
                    pick(rng, &pools.period_ids),
                    pick(rng, &pools.non_interrogation_subject_ids),
                    None,
                )
            }
            1 if !pools.period_ids.is_empty() && !pools.interrogation_subject_ids.is_empty() => {
                // Associating a dangling group list id
                GroupListOp::AssignToSubject(
                    pick(rng, &pools.period_ids),
                    pick(rng, &pools.interrogation_subject_ids),
                    Some(unsafe { GroupListId::new(dangling(rng)) }),
                )
            }
            _ => GroupListOp::Remove(unsafe { GroupListId::new(dangling(rng)) }),
        };
        return Op::GroupList(op);
    }
    let n = pools.group_list_ids.len();
    let add_w = if n < 4 { 5 } else { 2 };
    let update_w = if n > 0 { 2 } else { 0 };
    let filling_w = if n > 0 { 3 } else { 0 };
    let assign_w =
        if n > 0 && !pools.period_ids.is_empty() && !pools.interrogation_subject_ids.is_empty() {
            3
        } else {
            0
        };
    let remove_w = if n > 0 { 2 } else { 0 };
    let op = match weighted(rng, &[add_w, update_w, filling_w, assign_w, remove_w]) {
        0 => {
            // A new list is always automatic (the default filling), which is
            // trivially consistent with any parameters.
            let group_count = rng.random_range(2..=5);
            let params = synth::group_list_parameters(rng, group_count);
            GroupListOp::Add(
                GroupList::new(params, GroupListFilling::default())
                    .expect("automatic filling is always consistent"),
            )
        }
        1 => {
            let group_list_id = pick(rng, &pools.group_list_ids);
            let current = inner
                .params
                .group_lists
                .group_list_map
                .get(&group_list_id)
                .expect("group list id from pool");
            // Keep the group count stable for prefilled lists so the reshaped
            // filling stays consistent with the new parameters; the whole value
            // is carried by the consolidated `Update` op.
            let (group_count, new_filling) = match current.filling() {
                GroupListFilling::Prefilled { groups } => (
                    groups.len(),
                    GroupListFilling::Prefilled {
                        groups: groups.clone(),
                    },
                ),
                GroupListFilling::Automatic { excluded_students } => (
                    rng.random_range(2..=5),
                    GroupListFilling::Automatic {
                        excluded_students: excluded_students.clone(),
                    },
                ),
            };
            let params = synth::group_list_parameters(rng, group_count);
            GroupListOp::Update(
                group_list_id,
                GroupList::new(params, new_filling).expect("group count kept in sync"),
            )
        }
        2 => {
            // Swap the filling while keeping the parameters — the ex-`SetFilling`
            // move, now expressed as a whole-value `Update`.
            let group_list_id = pick(rng, &pools.group_list_ids);
            let params = inner
                .params
                .group_lists
                .group_list_map
                .get(&group_list_id)
                .expect("group list id from pool")
                .params()
                .clone();
            let group_names_len = params.group_names.len();
            let filling = if rng.random_bool(0.5) {
                synth::prefilled_filling(rng, group_names_len, &pools.student_ids)
            } else {
                synth::automatic_filling(rng, &pools.student_ids)
            };
            GroupListOp::Update(
                group_list_id,
                GroupList::new(params, filling).expect("prefilled count matches group_names"),
            )
        }
        3 => {
            let period_id = pick(rng, &pools.period_ids);
            // Interrogation subjects that actually run on the chosen period
            let eligible_subjects: Vec<SubjectId> = inner
                .params
                .subjects
                .ordered_subject_list
                .iter()
                .filter(|(_, s)| {
                    s.parameters.interrogation_parameters.is_some()
                        && !s.excluded_periods.contains(&period_id)
                })
                .map(|(id, _)| id)
                .collect();
            let subject_id = if eligible_subjects.is_empty() {
                pick(rng, &pools.interrogation_subject_ids)
            } else {
                pick(rng, &eligible_subjects)
            };
            let group_list_id = if rng.random_bool(0.8) {
                Some(pick(rng, &pools.group_list_ids))
            } else {
                None
            };
            GroupListOp::AssignToSubject(period_id, subject_id, group_list_id)
        }
        _ => GroupListOp::Remove(pick(rng, &pools.group_list_ids)),
    };
    Op::GroupList(op)
}

fn gen_settings(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    let mut settings = synth::settings(rng, &pools.student_ids);
    if invalid {
        settings
            .students
            .insert(unsafe { StudentId::new(dangling(rng)) }, Default::default());
    }
    Op::Settings(SettingsOp::Update(settings))
}

fn gen_pairing(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    if invalid {
        let op = if !pools.subject_ids.is_empty() && rng.random_bool(0.6) {
            // Dangling subject in the consequent: `PairingRule::new` accepts the
            // value (the two ids are distinct), the gate rejects the op with
            // `Error::BrokenInvariants` (a dangling subject FK), and the force path
            // lands that dangle. (Before the seal this arm built a same-subject
            // rule; that value is now unrepresentable.)
            let real = pick(rng, &pools.subject_ids);
            let ghost = unsafe { SubjectId::new(dangling(rng)) };
            PairingOp::Add(synth::pairing_rule(rng, real, ghost, &pools.period_ids))
        } else {
            PairingOp::Remove(unsafe { PairingRuleId::new(dangling(rng)) })
        };
        return Op::Pairing(op);
    }
    let can_add = pools.subject_ids.len() >= 2;
    let n = pools.pairing_rule_ids.len();
    let add_w = if can_add {
        if n < 3 { 5 } else { 2 }
    } else {
        0
    };
    let update_w = if n > 0 && can_add { 3 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };
    let distinct_pair = |rng: &mut ChaCha8Rng| {
        let first = rng.random_range(0..pools.subject_ids.len());
        let mut second = rng.random_range(0..pools.subject_ids.len() - 1);
        if second >= first {
            second += 1;
        }
        (pools.subject_ids[first], pools.subject_ids[second])
    };
    let op = match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => {
            let (antecedent, consequent) = distinct_pair(rng);
            PairingOp::Add(synth::pairing_rule(
                rng,
                antecedent,
                consequent,
                &pools.period_ids,
            ))
        }
        1 => {
            let (antecedent, consequent) = distinct_pair(rng);
            PairingOp::Update(
                pick(rng, &pools.pairing_rule_ids),
                synth::pairing_rule(rng, antecedent, consequent, &pools.period_ids),
            )
        }
        _ => PairingOp::Remove(pick(rng, &pools.pairing_rule_ids)),
    };
    Op::Pairing(op)
}

fn gen_slot_pairing(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    let pairable: Vec<&(SubjectId, Vec<SlotId>)> = pools
        .slots_by_subject
        .iter()
        .filter(|(_, slots)| slots.len() >= 2)
        .collect();
    if invalid {
        let op = if !pools.slot_ids.is_empty() && rng.random_bool(0.6) {
            // Dangling slot in the consequent: `SlotPairingRule::new` accepts
            // the value (the two ids are distinct), the gate rejects the op
            // with `Error::BrokenInvariants` (a dangling slot FK), and the force path
            // lands that dangle. (Before the seal this arm built a same-slot
            // rule; that value is now unrepresentable.)
            let real = pick(rng, &pools.slot_ids);
            let ghost = unsafe { SlotId::new(dangling(rng)) };
            SlotPairingOp::Add(synth::slot_pairing_rule(
                rng,
                real,
                ghost,
                &pools.period_ids,
            ))
        } else {
            SlotPairingOp::Remove(unsafe { SlotPairingRuleId::new(dangling(rng)) })
        };
        return Op::SlotPairing(op);
    }
    let n = pools.slot_pairing_rule_ids.len();
    let add_w = if pairable.is_empty() {
        0
    } else if n < 3 {
        5
    } else {
        2
    };
    let update_w = if n > 0 && !pairable.is_empty() { 3 } else { 0 };
    let remove_w = if n > 0 { 2 } else { 0 };
    let distinct_slots = |rng: &mut ChaCha8Rng| {
        let (_, slots) = pairable[rng.random_range(0..pairable.len())];
        let first = rng.random_range(0..slots.len());
        let mut second = rng.random_range(0..slots.len() - 1);
        if second >= first {
            second += 1;
        }
        (slots[first], slots[second])
    };
    let op = match weighted(rng, &[add_w, update_w, remove_w]) {
        0 => {
            let (antecedent, consequent) = distinct_slots(rng);
            SlotPairingOp::Add(synth::slot_pairing_rule(
                rng,
                antecedent,
                consequent,
                &pools.period_ids,
            ))
        }
        1 => {
            let (antecedent, consequent) = distinct_slots(rng);
            SlotPairingOp::Update(
                pick(rng, &pools.slot_pairing_rule_ids),
                synth::slot_pairing_rule(rng, antecedent, consequent, &pools.period_ids),
            )
        }
        _ => SlotPairingOp::Remove(pick(rng, &pools.slot_pairing_rule_ids)),
    };
    Op::SlotPairing(op)
}

fn gen_balancing(rng: &mut ChaCha8Rng, pools: &Pools, invalid: bool) -> Op {
    let mut balancing = synth::balancing(rng, &pools.interrogation_subject_ids);
    if invalid {
        let subject_id = if !pools.non_interrogation_subject_ids.is_empty() && rng.random_bool(0.5)
        {
            pick(rng, &pools.non_interrogation_subject_ids)
        } else {
            unsafe { SubjectId::new(dangling(rng)) }
        };
        balancing.subjects.insert(subject_id, Default::default());
    }
    Op::Balancing(BalancingOp::Update(balancing))
}

fn gen_colloscope(rng: &mut ChaCha8Rng, inner: &InnerData, pools: &Pools, invalid: bool) -> Op {
    let use_group_list = if pools.colloscope_group_list_ids.is_empty() {
        false
    } else if pools.colloscope_targets.is_empty() {
        true
    } else {
        rng.random_bool(0.4)
    };

    if use_group_list {
        let group_list_id = pick(rng, &pools.colloscope_group_list_ids);
        let group_list = inner
            .params
            .group_lists
            .group_list_map
            .get(&group_list_id)
            .expect("group list id from pool");
        let group_count = group_list.params().group_names.len() as u32;
        let allowed_students: Vec<StudentId> = pools
            .student_ids
            .iter()
            .copied()
            .filter(|id| !group_list.filling().excluded_students().contains(id))
            .collect();
        let mut groups_for_students: BTreeMap<StudentId, u32> = BTreeMap::new();
        if group_count > 0 {
            for student_id in synth::subset(rng, &allowed_students, 0.5) {
                groups_for_students.insert(student_id, rng.random_range(0..group_count));
            }
        }
        if invalid {
            groups_for_students.insert(unsafe { StudentId::new(dangling(rng)) }, 0);
        }
        return Op::Colloscope(ColloscopeOp::SetGroupList(
            group_list_id,
            groups_for_students,
        ));
    }

    let (period_id, slot_id, weeks) =
        &pools.colloscope_targets[rng.random_range(0..pools.colloscope_targets.len())];
    let week_in_period = weeks[rng.random_range(0..weeks.len())];

    // Group numbers are bounded by the group list associated to the
    // slot's subject on this period (no association => no valid group)
    let (subject_id, _pos) = inner
        .params
        .slots
        .find_slot_subject_and_position(*slot_id)
        .expect("Slot id comes from the live colloscope");
    let group_bound: u32 = inner
        .params
        .group_lists
        .subjects_associations
        .get(&(*period_id, subject_id))
        .map(|group_list_id| {
            inner
                .params
                .group_lists
                .group_list_map
                .get(group_list_id)
                .expect("association references a live group list")
                .params()
                .group_names
                .len() as u32
        })
        .unwrap_or(0);

    let mut assigned_groups = BTreeSet::new();
    if invalid {
        assigned_groups.insert(group_bound + rng.random_range(0..10));
    } else if group_bound > 0 {
        for _ in 0..rng.random_range(0..=2u32) {
            assigned_groups.insert(rng.random_range(0..group_bound));
        }
    }

    let week_id = inner
        .params
        .weeks
        .week_id_at(*period_id, week_in_period)
        .expect("position within the period is valid");
    Op::Colloscope(ColloscopeOp::SetInterrogation(
        *slot_id,
        week_id,
        assigned_groups,
    ))
}

fn gen_export_config(rng: &mut ChaCha8Rng) -> Op {
    let op = match rng.random_range(0..11) {
        0 => ExportConfigOp::UpdateGlobalConfig(synth::global_config(rng)),
        1 => ExportConfigOp::UpdateColloscopeEnabled(rng.random_bool(0.5)),
        2 => ExportConfigOp::UpdateAllGroupsEnabled(rng.random_bool(0.5)),
        3 => ExportConfigOp::UpdatePrefilledGroupsEnabled(rng.random_bool(0.5)),
        4 => ExportConfigOp::UpdateAutomaticGroupsEnabled(rng.random_bool(0.5)),
        5 => ExportConfigOp::UpdatePerGroupListEnabled(rng.random_bool(0.5)),
        6 => ExportConfigOp::UpdateColloscopeConfig(synth::colloscope_config(rng)),
        7 => ExportConfigOp::UpdateAllGroupsConfig(synth::per_student_groups_config(rng)),
        8 => ExportConfigOp::UpdatePrefilledGroupsConfig(synth::per_student_groups_config(rng)),
        9 => ExportConfigOp::UpdateAutomaticGroupsConfig(synth::per_student_groups_config(rng)),
        _ => ExportConfigOp::UpdatePerGroupListConfig(synth::per_group_list_config(rng)),
    };
    Op::ExportConfig(op)
}

fn gen_global_update(
    rng: &mut ChaCha8Rng,
    inner: &InnerData,
    pools: &Pools,
    snapshots: &[InnerData],
    invalid: bool,
) -> Op {
    if invalid && !pools.subject_ids.is_empty() {
        // Corrupt a clone of the current state with a duplicated id.
        // A duplicated id (rather than a dangling one) keeps the maximum id
        // unchanged, so the failed op does not advance the id issuer into
        // the DANGLING_BASE range.
        let mut broken = inner.clone();
        let duplicated = unsafe { StudentId::new(pick(rng, &pools.subject_ids).inner()) };
        broken
            .params
            .students
            .student_map
            .insert(duplicated, Student::default());
        return Op::GlobalUpdate(broken);
    }
    if !snapshots.is_empty() && rng.random_bool(0.8) {
        Op::GlobalUpdate(snapshots[rng.random_range(0..snapshots.len())].clone())
    } else {
        Op::GlobalUpdate(inner.clone())
    }
}

// ============================================================================
// Gate-property fuzz: the corruption generator (born as the step-4
// differential fuzz)
// ============================================================================

/// Probe kinds for the gate-property fuzz (`property_apply_gate.rs`).
///
/// Every probe op is *carve-out-clean*: it targets a live entity, uses fresh
/// (dangling) or duplicated ids and in-bounds positions, so `force_apply` lands
/// it rather than bouncing off a kept precheck guard. All but [`Self::ForceValid`]
/// additionally aim at a *stripped* invariant, so the landed state is (usually)
/// broken — the depth-1 probe distribution the apply gate must reject and roll
/// back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorruptionKind {
    /// `Remove` of an existing (likely referenced) entity → dangling FKs.
    ForceRemove,
    /// `Update` whose payload embeds a dangling id.
    ForceRetarget,
    /// Valid-shaped op whose only obstacle was a stripped invariant guard.
    ForceSemantic,
    /// Op landing a `LogicError` state (dup-id `GlobalUpdate`). The
    /// same-subject pairing and same-slot slot-pairing flavors are gone: the
    /// sealed `PairingRule`/`SlotPairingRule` make those states unrepresentable.
    ForceLogic,
    /// Plain valid op — the clean-landing probe.
    ForceValid,
}

impl CorruptionKind {
    /// Every kind, for coverage assertions.
    pub const ALL: [CorruptionKind; 5] = [
        CorruptionKind::ForceRemove,
        CorruptionKind::ForceRetarget,
        CorruptionKind::ForceSemantic,
        CorruptionKind::ForceLogic,
        CorruptionKind::ForceValid,
    ];

    /// Label for the OpLog/RunStats category tracking (the harness keys on
    /// `&'static str`).
    pub fn label(self) -> &'static str {
        match self {
            CorruptionKind::ForceRemove => "force_remove",
            CorruptionKind::ForceRetarget => "force_retarget",
            CorruptionKind::ForceSemantic => "force_semantic",
            CorruptionKind::ForceLogic => "force_logic",
            CorruptionKind::ForceValid => "force_valid",
        }
    }

    /// The four kinds expected to be able to break a state (everything but
    /// [`Self::ForceValid`]).
    pub fn corrupting(self) -> bool {
        !matches!(self, CorruptionKind::ForceValid)
    }
}

/// Generates one probe op for the gate-property fuzz.
///
/// A [`CorruptionKind`] is chosen uniformly among the kinds that have material
/// in the current state (retarget and valid are always available); the returned
/// op is carve-out-clean so `force_apply` lands it. All but
/// [`CorruptionKind::ForceValid`] aim at a stripped invariant.
pub fn gen_corruption_op(rng: &mut ChaCha8Rng, inner: &InnerData) -> (CorruptionKind, Op) {
    let pools = Pools::extract(inner);
    let semantic = available_semantic_recipes(inner, &pools);
    let logic = available_logic_recipes(inner, &pools);

    let mut eligible: Vec<CorruptionKind> = Vec::new();
    if removable_present(&pools) {
        eligible.push(CorruptionKind::ForceRemove);
    }
    eligible.push(CorruptionKind::ForceRetarget); // settings/balancing always work
    if !semantic.is_empty() {
        eligible.push(CorruptionKind::ForceSemantic);
    }
    if !logic.is_empty() {
        eligible.push(CorruptionKind::ForceLogic);
    }
    eligible.push(CorruptionKind::ForceValid);

    let kind = eligible[rng.random_range(0..eligible.len())];
    let op = match kind {
        CorruptionKind::ForceRemove => {
            gen_force_remove(rng, &pools).expect("removable pool present")
        }
        CorruptionKind::ForceRetarget => gen_force_retarget(rng, inner, &pools),
        CorruptionKind::ForceSemantic => gen_force_semantic(rng, inner, &pools, &semantic),
        CorruptionKind::ForceLogic => gen_force_logic(rng, inner, &pools, &logic),
        CorruptionKind::ForceValid => gen_op(rng, inner, &[], 0.0).1,
    };
    (kind, op)
}

fn removable_present(pools: &Pools) -> bool {
    !pools.student_ids.is_empty()
        || !pools.period_ids.is_empty()
        || !pools.week_ids.is_empty()
        || !pools.subject_ids.is_empty()
        || !pools.teacher_ids.is_empty()
        || !pools.week_pattern_ids.is_empty()
        || !pools.slot_ids.is_empty()
        || !pools.incompat_ids.is_empty()
        || !pools.group_list_ids.is_empty()
        || !pools.pairing_rule_ids.is_empty()
        || !pools.slot_pairing_rule_ids.is_empty()
}

/// ForceRemove: one `Remove` of a live entity, drawn uniformly over the
/// non-empty pools. Highest-yield dangling-FK source; the period draw covers all
/// periods, so a period still holding weeks can be picked — the force path
/// dropped `PeriodStillHasWeeks`, so it lands broken (dangling `Week::period_id`
/// FKs) instead of bouncing.
fn gen_force_remove(rng: &mut ChaCha8Rng, pools: &Pools) -> Option<Op> {
    let mut candidates: Vec<Op> = Vec::new();
    if !pools.student_ids.is_empty() {
        candidates.push(Op::Student(StudentOp::Remove(pick(
            rng,
            &pools.student_ids,
        ))));
    }
    if !pools.period_ids.is_empty() {
        candidates.push(Op::Period(PeriodOp::Remove(pick(rng, &pools.period_ids))));
    }
    if !pools.week_ids.is_empty() {
        candidates.push(Op::Week(WeekOp::Remove(pick(rng, &pools.week_ids))));
    }
    if !pools.subject_ids.is_empty() {
        candidates.push(Op::Subject(SubjectOp::Remove(pick(
            rng,
            &pools.subject_ids,
        ))));
    }
    if !pools.teacher_ids.is_empty() {
        candidates.push(Op::Teacher(TeacherOp::Remove(pick(
            rng,
            &pools.teacher_ids,
        ))));
    }
    if !pools.week_pattern_ids.is_empty() {
        candidates.push(Op::WeekPattern(WeekPatternOp::Remove(pick(
            rng,
            &pools.week_pattern_ids,
        ))));
    }
    if !pools.slot_ids.is_empty() {
        candidates.push(Op::Slot(SlotOp::Remove(pick(rng, &pools.slot_ids))));
    }
    if !pools.incompat_ids.is_empty() {
        candidates.push(Op::Incompat(IncompatOp::Remove(pick(
            rng,
            &pools.incompat_ids,
        ))));
    }
    if !pools.group_list_ids.is_empty() {
        candidates.push(Op::GroupList(GroupListOp::Remove(pick(
            rng,
            &pools.group_list_ids,
        ))));
    }
    if !pools.pairing_rule_ids.is_empty() {
        candidates.push(Op::Pairing(PairingOp::Remove(pick(
            rng,
            &pools.pairing_rule_ids,
        ))));
    }
    if !pools.slot_pairing_rule_ids.is_empty() {
        candidates.push(Op::SlotPairing(SlotPairingOp::Remove(pick(
            rng,
            &pools.slot_pairing_rule_ids,
        ))));
    }
    if candidates.is_empty() {
        return None;
    }
    let idx = rng.random_range(0..candidates.len());
    Some(candidates.swap_remove(idx))
}

/// ForceRetarget: an `Update` on a live target whose payload embeds a dangling
/// id (stripped `validate_*` lets it land as a dangling FK). Settings and
/// balancing are always available (whole-config replace), so the candidate set
/// is never empty.
fn gen_force_retarget(rng: &mut ChaCha8Rng, inner: &InnerData, pools: &Pools) -> Op {
    let mut candidates: Vec<Op> = Vec::new();

    {
        let mut settings = synth::settings(rng, &pools.student_ids);
        settings
            .students
            .insert(unsafe { StudentId::new(dangling(rng)) }, Default::default());
        candidates.push(Op::Settings(SettingsOp::Update(settings)));
    }
    {
        let mut balancing = synth::balancing(rng, &pools.interrogation_subject_ids);
        balancing
            .subjects
            .insert(unsafe { SubjectId::new(dangling(rng)) }, Default::default());
        candidates.push(Op::Balancing(BalancingOp::Update(balancing)));
    }
    if !pools.student_ids.is_empty() {
        let id = pick(rng, &pools.student_ids);
        let mut student = synth::student(rng, &pools.period_ids);
        student
            .excluded_periods
            .insert(unsafe { PeriodId::new(dangling(rng)) });
        candidates.push(Op::Student(StudentOp::Update(id, student)));
    }
    if !pools.subject_ids.is_empty() {
        let id = pick(rng, &pools.subject_ids);
        let with_interrogation = rng.random_bool(0.7);
        let mut subject = synth::subject(rng, &pools.period_ids, with_interrogation);
        subject
            .excluded_periods
            .insert(unsafe { PeriodId::new(dangling(rng)) });
        candidates.push(Op::Subject(SubjectOp::Update(id, subject)));
    }
    if !pools.teacher_ids.is_empty() {
        let id = pick(rng, &pools.teacher_ids);
        let mut teacher = synth::teacher(rng, &pools.interrogation_subject_ids);
        teacher
            .subjects
            .insert(unsafe { SubjectId::new(dangling(rng)) });
        candidates.push(Op::Teacher(TeacherOp::Update(id, teacher)));
    }
    if !pools.incompat_ids.is_empty() {
        let id = pick(rng, &pools.incompat_ids);
        let dangling_subject = unsafe { SubjectId::new(dangling(rng)) };
        let incompat = synth::incompatibility(rng, dangling_subject, &pools.week_pattern_ids);
        candidates.push(Op::Incompat(IncompatOp::Update(id, incompat)));
    }
    if !pools.slot_ids.is_empty() {
        // Update must keep the slot's subject (the kept `CannotChangeSubject`
        // guard) and its teacher, so retarget the week pattern to a dangling id.
        let slot_id = pick(rng, &pools.slot_ids);
        let (subject_id, _pos) = inner
            .params
            .slots
            .find_slot_subject_and_position(slot_id)
            .expect("slot id from live pool");
        let teacher_id = inner
            .params
            .slots
            .find_slot(slot_id)
            .expect("slot id from live pool")
            .teacher_id;
        let mut slot = synth::slot(rng, subject_id, teacher_id, &[]);
        slot.week_pattern = Some(unsafe { WeekPatternId::new(dangling(rng)) });
        candidates.push(Op::Slot(SlotOp::Update(slot_id, slot)));
    }
    {
        // A dangling student id buried inside a prefilled group. The rebuilt
        // filling keeps `group_names.len()` groups so `GroupList::new` accepts it
        // (count matches, the single student is not duplicated); force's stripped
        // `validate_group_list` reference scan is what lets the dangling FK land.
        // Needs a list with at least one group to hold it.
        let lists = group_lists_with_min_groups(inner, 1);
        if !lists.is_empty() {
            let group_list_id = pick(rng, &lists);
            let params = inner
                .params
                .group_lists
                .group_list_map
                .get(&group_list_id)
                .expect("group list id from live pool")
                .params()
                .clone();
            let group_count = params.group_names.len();
            let mut groups: Vec<PrefilledGroup> = (0..group_count)
                .map(|_| PrefilledGroup::default())
                .collect();
            groups[0]
                .students
                .insert(unsafe { StudentId::new(dangling(rng)) });
            let group_list = GroupList::new(params, GroupListFilling::Prefilled { groups })
                .expect("count matches group_names and the lone student is not duplicated");
            candidates.push(Op::GroupList(GroupListOp::Update(
                group_list_id,
                group_list,
            )));
        }
    }

    let idx = rng.random_range(0..candidates.len());
    candidates.swap_remove(idx)
}

/// Live group lists whose `group_names` count is at least `min`. Used to gate
/// the prefilled-filling recipes, which need enough groups to place students in.
fn group_lists_with_min_groups(inner: &InnerData, min: usize) -> Vec<GroupListId> {
    inner
        .params
        .group_lists
        .group_list_map
        .iter()
        .filter(|(_, group_list)| group_list.params().group_names.len() >= min)
        .map(|(id, _)| id)
        .collect()
}

/// A ForceSemantic recipe whose material is present in the current state.
#[derive(Clone, Copy)]
enum SemanticRecipe {
    /// `Assign(period, student, subject, true)` of a student excluded on that
    /// period (stripped `StudentIsNotPresentOnPeriod`).
    ExcludedAssign,
    /// `TeacherUpdate` dropping a subject still bound to one of the teacher's
    /// slots (stripped slot-consistency scan).
    TeacherDrop,
    /// `SubjectUpdate` newly excluding a period that holds an assignment
    /// (stripped newly-excluded-period scan).
    SubjectExclude,
    /// `WeekUpdate` flipping interrogations off under a colloscope row
    /// (stripped silencing guard).
    WeekOff,
}

fn available_semantic_recipes(inner: &InnerData, pools: &Pools) -> Vec<SemanticRecipe> {
    let mut recipes = Vec::new();
    if !pools.subject_ids.is_empty()
        && inner
            .params
            .students
            .student_map
            .values()
            .any(|s| !s.excluded_periods.is_empty())
    {
        recipes.push(SemanticRecipe::ExcludedAssign);
    }
    if !pools.slot_ids.is_empty() {
        recipes.push(SemanticRecipe::TeacherDrop);
    }
    if inner.params.assignments.iter().next().is_some() {
        recipes.push(SemanticRecipe::SubjectExclude);
    }
    if !inner.colloscope.is_empty() {
        recipes.push(SemanticRecipe::WeekOff);
    }
    recipes
}

fn gen_force_semantic(
    rng: &mut ChaCha8Rng,
    inner: &InnerData,
    pools: &Pools,
    recipes: &[SemanticRecipe],
) -> Op {
    match recipes[rng.random_range(0..recipes.len())] {
        SemanticRecipe::ExcludedAssign => {
            let excluded_students: Vec<(StudentId, Vec<PeriodId>)> = inner
                .params
                .students
                .student_map
                .iter()
                .filter(|(_, s)| !s.excluded_periods.is_empty())
                .map(|(id, s)| (id, s.excluded_periods.iter().copied().collect()))
                .collect();
            let (student_id, periods) =
                &excluded_students[rng.random_range(0..excluded_students.len())];
            let period_id = periods[rng.random_range(0..periods.len())];
            // Prefer a subject that runs on that period, so the only breakage is
            // the excluded student (not also a non-running subject).
            let running: Vec<SubjectId> = inner
                .params
                .subjects
                .ordered_subject_list
                .iter()
                .filter(|(_, s)| !s.excluded_periods.contains(&period_id))
                .map(|(id, _)| id)
                .collect();
            let subject_id = if running.is_empty() {
                pick(rng, &pools.subject_ids)
            } else {
                pick(rng, &running)
            };
            Op::Assignment(AssignmentOp::Assign(
                period_id,
                *student_id,
                subject_id,
                true,
            ))
        }
        SemanticRecipe::TeacherDrop => {
            let slot_id = pick(rng, &pools.slot_ids);
            let slot = inner
                .params
                .slots
                .find_slot(slot_id)
                .expect("slot id from live pool");
            let (teacher_id, subject_id) = (slot.teacher_id, slot.subject_id);
            let mut teacher = inner
                .params
                .teachers
                .teacher_map
                .get(&teacher_id)
                .expect("a live slot's teacher is live")
                .clone();
            teacher.subjects.remove(&subject_id);
            Op::Teacher(TeacherOp::Update(teacher_id, teacher))
        }
        SemanticRecipe::SubjectExclude => {
            let rows: Vec<(PeriodId, SubjectId)> = inner
                .params
                .assignments
                .iter()
                .map(|(period, subject, _)| (period, subject))
                .collect();
            let (period_id, subject_id) = rows[rng.random_range(0..rows.len())];
            let mut subject = inner
                .params
                .subjects
                .find_subject(subject_id)
                .expect("a subject with an assignment row is live")
                .clone();
            subject.excluded_periods.insert(period_id);
            Op::Subject(SubjectOp::Update(subject_id, subject))
        }
        SemanticRecipe::WeekOff => {
            let weeks: Vec<WeekId> = inner
                .colloscope
                .iter()
                .map(|((_, week), _)| week)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let week_id = weeks[rng.random_range(0..weeks.len())];
            Op::Week(WeekOp::Update(week_id, WeekDesc::new(false)))
        }
    }
}

/// A ForceLogic recipe whose material is present in the current state. Each
/// lands a `LogicError` state (short-circuiting the new checker).
///
/// (The prefill count/duplicate flavors are gone: `GroupList::new` makes those
/// states unrepresentable — a mismatched or duplicate-student filling can no
/// longer be constructed, neither through the op surface nor a `GlobalUpdate`
/// clone — so there is no such `LogicError` left to forge. Likewise the
/// same-subject pairing and same-slot slot-pairing flavors are gone:
/// `PairingRule::new`/`SlotPairingRule::new` reject a rule with both parts on
/// one subject/slot, so `PairingRulePartsShareSubject` and
/// `SlotPairingRulePartsShareSlot` can no longer be reached and there is
/// nothing left to forge — `GlobalDup` is the only recipe remaining.)
#[derive(Clone, Copy)]
enum LogicRecipe {
    /// `GlobalUpdate` clone with a duplicated id (kept id max, so the issuer
    /// stays out of the dangling range) → `DuplicatedId`.
    GlobalDup,
}

fn available_logic_recipes(_inner: &InnerData, pools: &Pools) -> Vec<LogicRecipe> {
    let mut recipes = Vec::new();
    if !pools.subject_ids.is_empty() {
        recipes.push(LogicRecipe::GlobalDup);
    }
    recipes
}

fn gen_force_logic(
    rng: &mut ChaCha8Rng,
    inner: &InnerData,
    pools: &Pools,
    recipes: &[LogicRecipe],
) -> Op {
    match recipes[rng.random_range(0..recipes.len())] {
        LogicRecipe::GlobalDup => {
            let mut broken = inner.clone();
            let duplicated = unsafe { StudentId::new(pick(rng, &pools.subject_ids).inner()) };
            broken
                .params
                .students
                .student_map
                .insert(duplicated, Student::default());
            Op::GlobalUpdate(broken)
        }
    }
}
