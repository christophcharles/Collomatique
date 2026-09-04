//! Semantic pins for the document order.
//!
//! These tests check the *order*, not the machinery. `generic/state/` already owns the
//! machinery: `generic/state/src/partial_order.rs` pins the building blocks and
//! `generic/state/tests/derive_content_ord.rs` pins what the derive generates. What is
//! left — and what only this crate can say — is which ordering rule each
//! colloscope type actually carries.
//!
//! Every test has the same shape: build a value and a twin that differs in one
//! deliberate way, call `content_cmp`, and assert the exact `Option<Ordering>`.
//! Written that way the test survives any future change of *how* the impl is
//! produced (derive, blanket, macro or hand-written).
//!
//! Ids are forged with `unsafe { XxxId::new(n) }`: nothing here is applied
//! through the gate, so no id issuer is involved and a forged id cannot
//! collide with a live entity. The two tests that do need a real document
//! (the `Data` quotient and the `default` sanity pin) build it through the op
//! surface instead.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use collomatique_state::{ContentOrd, InMemoryData};

use crate::assignments::Assignments;
use crate::balancing::BalancingOptions;
use crate::colloscopes::Colloscope;
use crate::export_config::ExportConfig;
use crate::group_lists::{
    GroupList, GroupListFilling, GroupListParameters, GroupLists, PrefilledGroup,
};
use crate::ids::{
    GroupListId, Id, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekId, WeekPatternId,
};
use crate::incompats::Incompatibility;
use crate::ops::{
    AnnotatedOp, AnnotatedPeriodOp, AnnotatedStudentOp, AnnotatedSubjectOp, AssignmentOp, Op,
    PeriodOp, StudentOp, SubjectOp,
};
use crate::pairings::{PairingRule, RulePart};
use crate::settings::{Limits, Settings};
use crate::slots::Slot;
use crate::soft_param::SoftParam;
use crate::students::Student;
use crate::subjects::{Subject, SubjectPeriodicity, Subjects, WeekBlock};
use crate::teachers::{Teacher, Teachers};
use crate::week_patterns::WeekPattern;
use crate::weeks::{WeekDesc, Weeks};
use crate::{Data, InnerData, NonEmptyRangeInclusive, OrderedTable, PersonWithContact, Table};

// ---- Forged ids ----

fn period(n: u64) -> PeriodId {
    unsafe { PeriodId::new(n) }
}
fn week(n: u64) -> WeekId {
    unsafe { WeekId::new(n) }
}
fn subject(n: u64) -> SubjectId {
    unsafe { SubjectId::new(n) }
}
fn teacher(n: u64) -> TeacherId {
    unsafe { TeacherId::new(n) }
}
fn student(n: u64) -> StudentId {
    unsafe { StudentId::new(n) }
}
fn week_pattern(n: u64) -> WeekPatternId {
    unsafe { WeekPatternId::new(n) }
}
fn slot_id(n: u64) -> SlotId {
    unsafe { SlotId::new(n) }
}
fn group_list_id(n: u64) -> GroupListId {
    unsafe { GroupListId::new(n) }
}

// ---- Small value builders ----

fn non_empty(text: &str) -> non_empty_string::NonEmptyString {
    non_empty_string::NonEmptyString::new(text.into()).expect("non-empty literal")
}

fn person(surname: &str) -> PersonWithContact {
    PersonWithContact {
        surname: surname.into(),
        ..Default::default()
    }
}

fn one_teacher(surname: &str) -> Teacher {
    Teacher {
        desc: person(surname),
        subjects: BTreeSet::new(),
    }
}

fn teachers(entries: &[(u64, &str)]) -> Teachers {
    let mut teacher_map = Table::new();
    for (id, surname) in entries {
        teacher_map.insert(teacher(*id), one_teacher(surname));
    }
    Teachers { teacher_map }
}

fn named_subject(name: &str) -> Subject {
    let mut value = Subject::default();
    value.parameters.name = name.into();
    value
}

fn subjects(entries: &[(u64, &str)]) -> Subjects {
    let mut ordered_subject_list = OrderedTable::new();
    for (pos, (id, name)) in entries.iter().enumerate() {
        ordered_subject_list
            .insert_at(pos, subject(*id), named_subject(name))
            .expect("the forged ids are distinct");
    }
    Subjects {
        ordered_subject_list,
    }
}

fn slot_start(hour: u32) -> collomatique_time::SlotStart {
    collomatique_time::SlotStart {
        weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
        start_time: collomatique_time::WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(hour, 0, 0).expect("a valid hour"),
        )
        .expect("a whole minute"),
    }
}

fn time_window(hour: u32) -> collomatique_time::SlotWithDuration {
    collomatique_time::SlotWithDuration::new(
        slot_start(hour),
        collomatique_time::NonZeroMinutes::new(60).expect("non-zero"),
    )
    .expect("an hour-long morning slot fits in the day")
}

fn a_slot(pattern: Option<WeekPatternId>, hour: u32) -> Slot {
    Slot {
        subject_id: subject(1),
        teacher_id: teacher(1),
        start_time: slot_start(hour),
        extra_info: String::new(),
        week_pattern: pattern,
        cost: 0,
    }
}

fn incompatibility(windows: Vec<collomatique_time::SlotWithDuration>) -> Incompatibility {
    Incompatibility {
        subject_id: subject(1),
        name: "Sport".into(),
        slots: windows,
        minimum_free_slots: NonZeroU32::new(1).expect("non-zero"),
        week_pattern_id: None,
    }
}

/// A group list whose groups are prefilled, one `group_names` entry per group
/// (the count match is `GroupList::new`'s first value-internal invariant).
fn prefilled(names: Vec<Option<&str>>, groups: Vec<&[u64]>) -> GroupList {
    GroupList::new(
        GroupListParameters {
            name: "Liste".into(),
            group_names: names.into_iter().map(|n| n.map(non_empty)).collect(),
            ..Default::default()
        },
        GroupListFilling::Prefilled {
            groups: groups
                .into_iter()
                .map(|students| PrefilledGroup {
                    students: students.iter().map(|s| student(*s)).collect(),
                })
                .collect(),
        },
    )
    .expect("the group count matches and no student sits in two groups")
}

fn pairing_rule(excluded_periods: &[u64], antecedent_should_have: bool) -> PairingRule {
    PairingRule::new(
        RulePart {
            subject_id: subject(1),
            should_have: antecedent_should_have,
        },
        RulePart {
            subject_id: subject(2),
            should_have: true,
        },
        excluded_periods.iter().map(|p| period(*p)).collect(),
        false,
    )
    .expect("the two parts name distinct subjects")
}

/// Annotates an op and pushes it through the gate. A rejection is a bug in the
/// fixture, never something a test tolerates.
fn apply(data: &mut Data, op: Op) -> AnnotatedOp {
    let (annotated, _) = data.annotate(op);
    data.apply(&annotated).expect("the fixture op should apply");
    annotated
}

// ---- The two roots ----

#[test]
fn data_ignores_the_id_issuer() {
    // Two op sequences ending on the same document: the empty one, and one
    // that issues a period id and then gives it back.
    let untouched = Data::default();

    let mut round_trip = Data::default();
    let issued = match apply(&mut round_trip, Op::Period(PeriodOp::AddFront)) {
        AnnotatedOp::Period(AnnotatedPeriodOp::AddFront(p)) => p,
        other => panic!("unexpected annotated op {other:?}"),
    };
    apply(&mut round_trip, Op::Period(PeriodOp::Remove(issued)));

    assert_eq!(
        untouched.get_inner_data(),
        round_trip.get_inner_data(),
        "the two sequences end on the same document"
    );
    assert_ne!(
        untouched.id_issuer.get_internal_counter(),
        round_trip.id_issuer.get_internal_counter(),
        "the issuers really do differ, so the quotient is doing work"
    );

    // The declared quotient: `#[ord(ignore)]` on the issuer.
    assert_eq!(untouched.content_cmp(&round_trip), Some(Ordering::Equal));
    // For `Data` that equivalence coincides with its hand-written `==`.
    assert_eq!(untouched, round_trip);
}

#[test]
fn default_is_below_a_populated_document() {
    // A document built through the gate, touching no configuration op — so
    // every atom (settings, balancing, export config) stays at its default and
    // only entities were added. A sanity pin, deliberately *not* the universal
    // claim that `default()` is below everything: the order has many minimal
    // elements.
    let mut data = Data::default();
    let the_period = match apply(&mut data, Op::Period(PeriodOp::AddFront)) {
        AnnotatedOp::Period(AnnotatedPeriodOp::AddFront(p)) => p,
        other => panic!("unexpected annotated op {other:?}"),
    };
    let the_subject = match apply(
        &mut data,
        Op::Subject(SubjectOp::AddAfter(None, Subject::default())),
    ) {
        AnnotatedOp::Subject(AnnotatedSubjectOp::AddAfter(s, _, _)) => s,
        other => panic!("unexpected annotated op {other:?}"),
    };
    let the_student = match apply(&mut data, Op::Student(StudentOp::Add(Student::default()))) {
        AnnotatedOp::Student(AnnotatedStudentOp::Add(u, _)) => u,
        other => panic!("unexpected annotated op {other:?}"),
    };
    apply(
        &mut data,
        Op::Assignment(AssignmentOp::SetRow(
            the_period,
            the_subject,
            BTreeSet::from([the_student]),
        )),
    );

    assert!(InnerData::default().content_lt(data.get_inner_data()));
}

// ---- Rows: presence, identity and value ----

#[test]
fn row_removal_is_strictly_below() {
    let both = teachers(&[(1, "Dupont"), (2, "Martin")]);
    let one = teachers(&[(1, "Dupont")]);

    assert_eq!(one.content_cmp(&both), Some(Ordering::Less));
    assert_eq!(both.content_cmp(&one), Some(Ordering::Greater));
}

#[test]
fn same_content_under_a_different_id_is_incomparable() {
    // The defining example. The two documents hold byte-identical teacher
    // *content*, but under different ids — so neither row is present in the
    // other, and neither includes the other.
    let under_one = teachers(&[(1, "Dupont")]);
    let under_two = teachers(&[(2, "Dupont")]);

    assert_eq!(under_one.content_cmp(&under_two), None);
    assert_ne!(under_one, under_two);
}

#[test]
fn same_id_with_a_different_name_is_incomparable() {
    // The key matches, so the row is compared by value — and two different
    // names are two different atoms, neither below the other.
    let dupont = teachers(&[(1, "Dupont")]);
    let martin = teachers(&[(1, "Martin")]);

    assert_eq!(dupont.content_cmp(&martin), None);
}

#[test]
fn reorder_is_incomparable() {
    // An ordered list is compared by embedding, and a permutation embeds in
    // neither direction.
    let forward = subjects(&[(1, "Maths"), (2, "Physique")]);
    let backward = subjects(&[(2, "Physique"), (1, "Maths")]);
    assert_eq!(forward.content_cmp(&backward), None);

    // The same on the week ordering sidecar: the week entities are untouched,
    // only the per-period order vector is permuted.
    let rows = |order: [u64; 2]| {
        Weeks::from_period_rows([(
            period(1),
            order
                .iter()
                .map(|w| (week(*w), WeekDesc::new(true)))
                .collect::<Vec<_>>(),
        )])
        .expect("the forged week ids are distinct")
    };
    assert_eq!(rows([10, 11]).content_cmp(&rows([11, 10])), None);
}

#[test]
fn middle_removal_in_an_ordered_list_is_strictly_below() {
    // Value-borne identity: a subject is identified by its id, not by its
    // position, so dropping one from the middle is plain removal (subsequence).
    let three = subjects(&[(1, "Maths"), (2, "Physique"), (3, "Chimie")]);
    let two = subjects(&[(1, "Maths"), (3, "Chimie")]);

    assert_eq!(two.content_cmp(&three), Some(Ordering::Less));
}

// ---- Sets: the content-not-semantics pin ----

#[test]
fn excluded_period_drop_is_strictly_below() {
    // The pin in one assertion. Dropping an exclusion *widens* what the subject
    // denotes (it now runs on one more period), but the document holds one
    // element less — and the order reads the content, never the meaning.
    let mut wide = Subject::default();
    wide.excluded_periods = BTreeSet::from([period(1), period(2)]);
    let mut narrow = Subject::default();
    narrow.excluded_periods = BTreeSet::from([period(1)]);

    assert_eq!(narrow.content_cmp(&wide), Some(Ordering::Less));
}

#[test]
fn subject_week_pattern_drop_is_strictly_below() {
    // Same reading as the exclusion set above, through the `Option` blanket: a
    // subject that follows no pattern runs on more weeks, but the document holds
    // one reference less.
    let mut dressed = Subject::default();
    dressed.week_pattern = Some(week_pattern(1));
    let bare = Subject::default();

    assert_eq!(bare.content_cmp(&dressed), Some(Ordering::Less));
    // Another pattern is not a smaller one.
    let mut other = Subject::default();
    other.week_pattern = Some(week_pattern(2));
    assert_eq!(other.content_cmp(&dressed), None);
}

#[test]
fn week_pattern_exclusion_drop_is_strictly_below() {
    let pattern = |excluded: &[u64]| WeekPattern {
        name: "Semaines A".into(),
        excluded_weeks: excluded.iter().map(|w| week(*w)).collect(),
    };

    assert_eq!(
        pattern(&[10]).content_cmp(&pattern(&[10, 11])),
        Some(Ordering::Less)
    );
    // A *different* exclusion is not a smaller one.
    assert_eq!(pattern(&[10]).content_cmp(&pattern(&[11])), None);
}

// ---- Options: clearing is removal, changing is not ----

#[test]
fn optional_edge_clear_is_strictly_below() {
    let with_pattern = a_slot(Some(week_pattern(1)), 9);
    let without = a_slot(None, 9);
    assert_eq!(without.content_cmp(&with_pattern), Some(Ordering::Less));

    // Moving the slot changes an atom: incomparable, not smaller.
    let moved = a_slot(Some(week_pattern(1)), 10);
    assert_eq!(with_pattern.content_cmp(&moved), None);
}

#[test]
fn contact_clear_is_strictly_below() {
    // The uniform `Option` rule, on a field that is no foreign key at all.
    let mut with_tel = person("Dupont");
    with_tel.tel = Some(non_empty("0102030405"));
    let cleared = person("Dupont");

    assert_eq!(cleared.content_cmp(&with_tel), Some(Ordering::Less));

    let mut other_tel = person("Dupont");
    other_tel.tel = Some(non_empty("0605040302"));
    assert_eq!(with_tel.content_cmp(&other_tel), None);
}

// ---- Junction tables ----

#[test]
fn assignment_row_shrink_and_clear() {
    let assignments = |rows: &[((u64, u64), &[u64])]| {
        let mut map = Table::new();
        for ((p, s), students) in rows {
            map.insert(
                (period(*p), subject(*s)),
                students
                    .iter()
                    .map(|u| student(*u))
                    .collect::<BTreeSet<_>>(),
            );
        }
        Assignments { map }
    };

    let full = assignments(&[((1, 1), &[1, 2])]);
    let shrunk = assignments(&[((1, 1), &[1])]);
    let cleared = assignments(&[]);

    assert_eq!(shrunk.content_cmp(&full), Some(Ordering::Less));
    assert_eq!(cleared.content_cmp(&shrunk), Some(Ordering::Less));

    // A swapped student is a different set, comparable to neither.
    let swapped = assignments(&[((1, 1), &[3])]);
    assert_eq!(shrunk.content_cmp(&swapped), None);
}

#[test]
fn association_retarget_is_incomparable() {
    let associations = |targets: &[((u64, u64), u64)]| {
        let mut subjects_associations = Table::new();
        for ((p, s), list) in targets {
            subjects_associations.insert((period(*p), subject(*s)), group_list_id(*list));
        }
        GroupLists {
            group_list_map: Table::new(),
            subjects_associations,
        }
    };

    let to_first = associations(&[((1, 1), 1)]);
    let to_second = associations(&[((1, 1), 2)]);
    let unassociated = associations(&[]);

    assert_eq!(to_first.content_cmp(&to_second), None);
    assert_eq!(unassociated.content_cmp(&to_first), Some(Ordering::Less));
}

// ---- Value-borne identity inside a Vec ----

#[test]
fn incompat_slot_window_removal_is_strictly_below() {
    // A time window *is* its value: nothing points at it by position, so
    // dropping one anywhere in the list is removing content. This holds
    // because the order pre-exists the resolution map — no fix touches this
    // field today.
    let three = incompatibility(vec![time_window(8), time_window(10), time_window(14)]);
    let without_middle = incompatibility(vec![time_window(8), time_window(14)]);
    assert_eq!(
        without_middle.content_cmp(&three),
        Some(Ordering::Less),
        "a middle removal is a subsequence, not a shift"
    );

    // A *modified* window is a different value, present in neither list.
    let modified = incompatibility(vec![time_window(8), time_window(11), time_window(14)]);
    assert_eq!(three.content_cmp(&modified), None);
}

// ---- The relational chain ----

#[test]
fn periodicity_blocks_are_one_atom() {
    let block = |delay: u32| WeekBlock {
        delay_in_weeks: delay,
        size_in_weeks: NonZeroU32::new(2).expect("non-zero"),
        interrogation_count_in_block: NonEmptyRangeInclusive::new(1..=1).expect("non-empty"),
    };
    let periodicity = |blocks: Vec<WeekBlock>| SubjectPeriodicity::AmountForEveryArbitraryBlock {
        blocks,
        minimum_week_separation: 0,
    };

    // Each block's delay is measured from the previous one, so the list is a
    // chain, not a collection: a different inner block is a different value.
    assert_eq!(
        periodicity(vec![block(0), block(1)]).content_cmp(&periodicity(vec![block(0), block(2)])),
        None
    );

    // The pin that separates "atom" from "subsequence" or "prefix": even a
    // strict *truncation* is incomparable, because dropping the tail re-dates
    // nothing but still changes one composite value.
    assert_eq!(
        periodicity(vec![block(0)]).content_cmp(&periodicity(vec![block(0), block(1)])),
        None
    );

    // Two enum variants are never comparable.
    assert_eq!(
        periodicity(vec![block(0)]).content_cmp(&SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks: NonZeroU32::new(2).expect("non-zero"),
        }),
        None
    );
}

// ---- Position-borne identity inside a Vec ----

#[test]
fn group_list_prefilled_minus_student_is_strictly_below() {
    let full = prefilled(vec![None, None], vec![&[1, 2], &[3]]);
    let minus_one = prefilled(vec![None, None], vec![&[1], &[3]]);

    assert_eq!(minus_one.content_cmp(&full), Some(Ordering::Less));
}

#[test]
fn group_list_trailing_group_removal_is_strictly_below() {
    // Adding a group is adding content, and zero groups is the minimum — so
    // truncating the list (names *and* groups, which `GroupList::new` keeps in
    // step) is a step down.
    let two = prefilled(vec![None, None], vec![&[1], &[2]]);
    let one = prefilled(vec![None], vec![&[1]]);

    assert_eq!(one.content_cmp(&two), Some(Ordering::Less));
}

#[test]
fn group_list_middle_group_removal_is_incomparable() {
    // The identity shift: removing group 1 re-aims group 2 onto index 1, and
    // the colloscope's placement maps reference groups by that index.
    let three = prefilled(vec![None, None, None], vec![&[1], &[2], &[3]]);
    let without_middle = prefilled(vec![None, None], vec![&[1], &[3]]);

    assert_eq!(without_middle.content_cmp(&three), None);
}

#[test]
fn group_name_unset_is_strictly_below() {
    let named = prefilled(vec![Some("Rouge"), Some("Bleu")], vec![&[1], &[2]]);
    let half_named = prefilled(vec![None, Some("Bleu")], vec![&[1], &[2]]);
    assert_eq!(half_named.content_cmp(&named), Some(Ordering::Less));

    // A rename is a different atom at that index, not a smaller one.
    let renamed = prefilled(vec![Some("Vert"), Some("Bleu")], vec![&[1], &[2]]);
    assert_eq!(named.content_cmp(&renamed), None);
}

#[test]
fn group_list_variant_change_is_incomparable() {
    let manual = GroupList::new(
        GroupListParameters {
            name: "Liste".into(),
            group_names: Vec::new(),
            ..Default::default()
        },
        GroupListFilling::Prefilled { groups: Vec::new() },
    )
    .expect("zero groups matches zero names");
    let automatic = GroupList::new(
        GroupListParameters {
            name: "Liste".into(),
            group_names: Vec::new(),
            ..Default::default()
        },
        GroupListFilling::Automatic {
            excluded_students: BTreeSet::new(),
        },
    )
    .expect("the automatic branch has nothing to validate");

    assert_eq!(manual.content_cmp(&automatic), None);
}

// ---- The colloscope ----

#[test]
fn colloscope_cell_trim_is_strictly_below() {
    let cell = |groups: &[u32]| {
        let mut colloscope = Colloscope::default();
        colloscope.set_interrogation(slot_id(1), week(1), groups.iter().copied().collect());
        colloscope
    };

    assert_eq!(cell(&[0]).content_cmp(&cell(&[0, 1])), Some(Ordering::Less));
    assert_eq!(cell(&[]).content_cmp(&cell(&[0])), Some(Ordering::Less));

    let placement = |entries: &[(u64, u32)]| {
        let mut colloscope = Colloscope::default();
        colloscope.set_group_list(
            group_list_id(1),
            entries
                .iter()
                .map(|(s, g)| (student(*s), *g))
                .collect::<BTreeMap<_, _>>(),
        );
        colloscope
    };

    assert_eq!(
        placement(&[(1, 0)]).content_cmp(&placement(&[(1, 0), (2, 1)])),
        Some(Ordering::Less)
    );
    // A renumbered placement keeps the key and changes the value atom.
    assert_eq!(
        placement(&[(1, 0)]).content_cmp(&placement(&[(1, 1)])),
        None
    );
}

// ---- The configuration records ----

#[test]
fn config_values_are_atoms() {
    // A `None` field in a limits override record means "disabled" — an active
    // choice, not absent content. So there is no bottom here: the default is
    // incomparable to any modification, not below it. The same holds for the
    // boolean records, whose fields are all active choices too.
    let limits = Limits::default();
    let mut other_limits = Limits::default();
    other_limits.interrogations_per_week_max = Some(SoftParam {
        soft: false,
        value: 3,
    });
    assert_eq!(limits.content_cmp(&limits.clone()), Some(Ordering::Equal));
    assert_eq!(limits.content_cmp(&other_limits), None);

    let balancing = BalancingOptions::default();
    let mut other_balancing = BalancingOptions::default();
    other_balancing.teacher_rotation = Some(crate::soft_param::SoftParam {
        soft: false,
        value: (),
    });
    assert_eq!(
        balancing.content_cmp(&balancing.clone()),
        Some(Ordering::Equal)
    );
    assert_eq!(balancing.content_cmp(&other_balancing), None);

    let export = ExportConfig::default();
    let mut other_export = ExportConfig::default();
    other_export.colloscope_enabled = !export.colloscope_enabled;
    assert_eq!(export.content_cmp(&export.clone()), Some(Ordering::Equal));
    assert_eq!(export.content_cmp(&other_export), None);
}

#[test]
fn settings_override_row_removal_is_strictly_below() {
    // The record is an atom, but the *table of overrides* is still a map:
    // dropping a whole per-student row is removing content.
    let with_override = Settings {
        global: Limits::default(),
        students: {
            let mut table = Table::new();
            table.insert(student(1), Limits::default());
            table
        },
    };
    let without = Settings::default();

    assert_eq!(without.content_cmp(&with_override), Some(Ordering::Less));
}

// ---- Pairing rules ----

#[test]
fn pairing_rule_excluded_period_drop_is_strictly_below() {
    let two = pairing_rule(&[1, 2], true);
    let one = pairing_rule(&[1], true);

    assert_eq!(one.content_cmp(&two), Some(Ordering::Less));
}

#[test]
fn pairing_rule_part_change_is_incomparable() {
    let positive = pairing_rule(&[1], true);
    let negative = pairing_rule(&[1], false);

    assert_eq!(positive.content_cmp(&negative), None);
}

// ---- The product rule at the top ----

#[test]
fn mixed_directions_are_incomparable() {
    // One document gains a student and loses a teacher relative to the other:
    // two fields disagree in direction, so the product is incomparable even
    // though neither field on its own is.
    let build = |students: &[u64], teachers_in: &[(u64, &str)]| {
        let mut inner = InnerData::default();
        for id in students {
            inner
                .params
                .students
                .student_map
                .insert(student(*id), Student::default());
        }
        inner.params.teachers = teachers(teachers_in);
        inner
    };

    let more_students = build(&[1, 2], &[(1, "Dupont")]);
    let more_teachers = build(&[1], &[(1, "Dupont"), (2, "Martin")]);

    assert_eq!(more_students.content_cmp(&more_teachers), None);
    // Each half on its own *is* comparable — the incomparability really comes
    // from mixing directions.
    let baseline = build(&[1], &[(1, "Dupont")]);
    assert_eq!(baseline.content_cmp(&more_students), Some(Ordering::Less));
    assert_eq!(baseline.content_cmp(&more_teachers), Some(Ordering::Less));
}
