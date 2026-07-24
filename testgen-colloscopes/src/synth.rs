//! Synthesis of domain entities for the property tests
//!
//! Every function is deterministic given the [ChaCha8Rng] state.
//! Valid entities mirror the constraints enforced by
//! `colloscope_params.rs`; the `*_invalid` variants deliberately
//! break exactly one of those constraints.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use collomatique_state_colloscopes::{
    NonEmptyRangeInclusive, PersonWithContact, Subject, SubjectInterrogationParameters,
    SubjectParameters, SubjectPeriodicity,
    balancing::{Balancing, BalancingOptions},
    export_config,
    group_lists::{GroupListFilling, GroupListParameters, PrefilledGroup},
    ids::{PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekId, WeekPatternId},
    incompats::Incompatibility,
    pairings::{PairingRule, RulePart},
    settings::{Limits, Settings},
    slot_pairings::{SlotPairingRule, SlotRulePart},
    slots::Slot,
    soft_param::SoftParam,
    students::Student,
    subjects::WeekBlock,
    teachers::Teacher,
    week_patterns::WeekPattern,
    weeks::WeekDesc,
};
use collomatique_time::{NonZeroMinutes, SlotStart, WeekStart, Weekday, WholeMinuteTime};

/// Picks a random element of a non-empty slice
pub fn pick<T: Copy>(rng: &mut ChaCha8Rng, items: &[T]) -> T {
    items[rng.random_range(0..items.len())]
}

/// Picks a random subset (possibly empty) of a slice
pub fn subset<T: Copy>(rng: &mut ChaCha8Rng, items: &[T], keep_probability: f64) -> Vec<T> {
    items
        .iter()
        .copied()
        .filter(|_| rng.random_bool(keep_probability))
        .collect()
}

fn non_empty(text: String) -> non_empty_string::NonEmptyString {
    non_empty_string::NonEmptyString::new(text).expect("Synthesized string should be non-empty")
}

pub fn person(rng: &mut ChaCha8Rng) -> PersonWithContact {
    let n: u32 = rng.random_range(0..100_000);
    PersonWithContact {
        surname: format!("Surname{n}"),
        firstname: format!("Firstname{n}"),
        tel: if rng.random_bool(0.3) {
            Some(non_empty(format!("06{n:08}")))
        } else {
            None
        },
        email: if rng.random_bool(0.3) {
            Some(non_empty(format!("person{n}@example.com")))
        } else {
            None
        },
    }
}

pub fn student(rng: &mut ChaCha8Rng, period_ids: &[PeriodId]) -> Student {
    let excluded_periods = if !period_ids.is_empty() && rng.random_bool(0.1) {
        BTreeSet::from([pick(rng, period_ids)])
    } else {
        BTreeSet::new()
    };
    Student {
        desc: person(rng),
        excluded_periods,
    }
}

pub fn week_desc(rng: &mut ChaCha8Rng) -> WeekDesc {
    let mut desc = WeekDesc::new(rng.random_bool(0.85));
    if rng.random_bool(0.1) {
        desc.annotation = Some(non_empty("week note".to_string()));
    }
    desc
}

pub fn week_desc_vec(rng: &mut ChaCha8Rng) -> Vec<WeekDesc> {
    let len = rng.random_range(3..=6);
    (0..len)
        .map(|i| {
            let mut desc = WeekDesc::new(rng.random_bool(0.85));
            if rng.random_bool(0.1) {
                desc.annotation = Some(non_empty(format!("week note {i}")));
            }
            desc
        })
        .collect()
}

/// Interrogation durations are kept short and start times are kept within
/// 8:00-18:00 so that a valid slot can never cross midnight, even after a
/// subject duration update.
pub const DURATION_CHOICES: [u32; 2] = [30, 60];

fn periodicity(rng: &mut ChaCha8Rng) -> SubjectPeriodicity {
    match rng.random_range(0..4) {
        0 => SubjectPeriodicity::OnceForEveryBlockOfWeeks {
            weeks_per_block: NonZeroU32::new(rng.random_range(1..=3)).unwrap(),
            minimum_week_separation: NonZeroU32::new(rng.random_range(1..=2)).unwrap(),
        },
        1 => SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks: NonZeroU32::new(rng.random_range(1..=3)).unwrap(),
        },
        2 => SubjectPeriodicity::AmountInYear {
            interrogation_count_in_year: {
                let min = rng.random_range(0..=3);
                NonEmptyRangeInclusive::new(min..=(min + rng.random_range(0..=3)))
                    .expect("statically non-empty")
            },
            minimum_week_separation: rng.random_range(0..=2),
        },
        _ => SubjectPeriodicity::AmountForEveryArbitraryBlock {
            blocks: (0..rng.random_range(0..=3))
                .map(|_| WeekBlock {
                    delay_in_weeks: rng.random_range(0..=2),
                    size_in_weeks: NonZeroU32::new(rng.random_range(1..=3)).unwrap(),
                    interrogation_count_in_block: {
                        let min = rng.random_range(0..=1);
                        NonEmptyRangeInclusive::new(min..=(min + rng.random_range(0..=2)))
                            .expect("statically non-empty")
                    },
                })
                .collect(),
            minimum_week_separation: rng.random_range(0..=2),
        },
    }
}

pub fn interrogation_parameters(rng: &mut ChaCha8Rng) -> SubjectInterrogationParameters {
    let students_min = rng.random_range(1..=3);
    let groups_min = rng.random_range(1..=2);
    SubjectInterrogationParameters {
        students_per_group: NonEmptyRangeInclusive::new(
            NonZeroU32::new(students_min).unwrap()
                ..=NonZeroU32::new(students_min + rng.random_range(0..=2)).unwrap(),
        )
        .expect("statically non-empty"),
        groups_per_interrogation: NonEmptyRangeInclusive::new(
            NonZeroU32::new(groups_min).unwrap()
                ..=NonZeroU32::new(groups_min + rng.random_range(0..=1)).unwrap(),
        )
        .expect("statically non-empty"),
        duration: NonZeroMinutes::new(pick(rng, &DURATION_CHOICES)).unwrap(),
        take_duration_into_account: rng.random_bool(0.8),
        periodicity: periodicity(rng),
    }
}

pub fn subject(rng: &mut ChaCha8Rng, period_ids: &[PeriodId], with_interrogation: bool) -> Subject {
    let excluded_periods = if period_ids.len() >= 2 && rng.random_bool(0.15) {
        BTreeSet::from([pick(rng, period_ids)])
    } else {
        BTreeSet::new()
    };
    Subject {
        parameters: SubjectParameters {
            name: format!("Subject{}", rng.random_range(0..100_000u32)),
            interrogation_parameters: if with_interrogation {
                Some(interrogation_parameters(rng))
            } else {
                None
            },
        },
        excluded_periods,
    }
}

pub fn teacher(rng: &mut ChaCha8Rng, interrogation_subject_ids: &[SubjectId]) -> Teacher {
    Teacher {
        desc: person(rng),
        subjects: subset(rng, interrogation_subject_ids, 0.5)
            .into_iter()
            .collect(),
    }
}

pub fn week_pattern(rng: &mut ChaCha8Rng, week_ids: &[WeekId]) -> WeekPattern {
    let mut excluded_weeks = BTreeSet::new();
    for &week_id in week_ids {
        if rng.random_bool(0.3) {
            excluded_weeks.insert(week_id);
        }
    }
    WeekPattern {
        name: format!("Pattern{}", rng.random_range(0..100_000u32)),
        excluded_weeks,
    }
}

/// A pattern that excludes a (presumably dangling) week — the invalid input
/// used to exercise the dangling-`WeekId` invariant.
pub fn week_pattern_excluding(rng: &mut ChaCha8Rng, week_id: WeekId) -> WeekPattern {
    WeekPattern {
        name: format!("Pattern{}", rng.random_range(0..100_000u32)),
        excluded_weeks: BTreeSet::from([week_id]),
    }
}

pub fn slot_start(rng: &mut ChaCha8Rng) -> SlotStart {
    let weekdays: Vec<Weekday> = Weekday::iter().collect();
    let minute = rng.random_range((8 * 60 / 5)..=(18 * 60 / 5)) * 5;
    SlotStart {
        weekday: pick(rng, &weekdays),
        start_time: WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(minute / 60, minute % 60, 0).unwrap(),
        )
        .unwrap(),
    }
}

/// A start time so late that any interrogation duration crosses midnight
pub fn slot_start_crossing_midnight(rng: &mut ChaCha8Rng) -> SlotStart {
    let weekdays: Vec<Weekday> = Weekday::iter().collect();
    SlotStart {
        weekday: pick(rng, &weekdays),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(23, 45, 0).unwrap())
            .unwrap(),
    }
}

pub fn slot(
    rng: &mut ChaCha8Rng,
    subject_id: SubjectId,
    teacher_id: TeacherId,
    week_pattern_ids: &[WeekPatternId],
) -> Slot {
    Slot {
        subject_id,
        teacher_id,
        start_time: slot_start(rng),
        extra_info: if rng.random_bool(0.3) {
            format!("Room {}", rng.random_range(100..400))
        } else {
            String::new()
        },
        week_pattern: if !week_pattern_ids.is_empty() && rng.random_bool(0.4) {
            Some(pick(rng, week_pattern_ids))
        } else {
            None
        },
        cost: rng.random_range(-2..=5),
    }
}

pub fn incompatibility(
    rng: &mut ChaCha8Rng,
    subject_id: SubjectId,
    week_pattern_ids: &[WeekPatternId],
) -> Incompatibility {
    let slot_count = rng.random_range(1..=3);
    let slots = (0..slot_count)
        .map(|_| {
            collomatique_time::SlotWithDuration::new(
                slot_start(rng),
                NonZeroMinutes::new(pick(rng, &DURATION_CHOICES)).unwrap(),
            )
            .expect("Synthesized slot should not cross midnight")
        })
        .collect();
    Incompatibility {
        subject_id,
        name: format!("Incompat{}", rng.random_range(0..100_000u32)),
        slots,
        minimum_free_slots: NonZeroU32::new(rng.random_range(1..=slot_count as u32)).unwrap(),
        week_pattern_id: if !week_pattern_ids.is_empty() && rng.random_bool(0.4) {
            Some(pick(rng, week_pattern_ids))
        } else {
            None
        },
    }
}

pub fn group_list_parameters(rng: &mut ChaCha8Rng, group_count: usize) -> GroupListParameters {
    let students_min = rng.random_range(1..=3);
    GroupListParameters {
        name: format!("List{}", rng.random_range(0..100_000u32)),
        students_per_group: NonEmptyRangeInclusive::new(
            NonZeroU32::new(students_min).unwrap()
                ..=NonZeroU32::new(students_min + rng.random_range(0..=2)).unwrap(),
        )
        .expect("statically non-empty"),
        group_names: (0..group_count)
            .map(|i| {
                if rng.random_bool(0.5) {
                    Some(non_empty(format!("Group {i}")))
                } else {
                    None
                }
            })
            .collect(),
    }
}

/// Builds a prefilled filling with exactly `group_count` groups over a
/// duplicate-free subset of `student_ids`
pub fn prefilled_filling(
    rng: &mut ChaCha8Rng,
    group_count: usize,
    student_ids: &[StudentId],
) -> GroupListFilling {
    let mut groups: Vec<PrefilledGroup> = (0..group_count)
        .map(|_| PrefilledGroup::default())
        .collect();
    if group_count > 0 {
        for student_id in subset(rng, student_ids, 0.5) {
            let group = rng.random_range(0..group_count);
            groups[group].students.insert(student_id);
        }
    }
    GroupListFilling::Prefilled { groups }
}

pub fn automatic_filling(rng: &mut ChaCha8Rng, student_ids: &[StudentId]) -> GroupListFilling {
    GroupListFilling::Automatic {
        excluded_students: subset(rng, student_ids, 0.2).into_iter().collect(),
    }
}

fn limits(rng: &mut ChaCha8Rng) -> Limits {
    Limits {
        interrogations_per_week_min: if rng.random_bool(0.4) {
            Some(SoftParam {
                soft: rng.random_bool(0.5),
                value: rng.random_range(0..=2),
            })
        } else {
            None
        },
        interrogations_per_week_max: if rng.random_bool(0.4) {
            Some(SoftParam {
                soft: rng.random_bool(0.5),
                value: rng.random_range(2..=6),
            })
        } else {
            None
        },
        max_interrogations_per_day: if rng.random_bool(0.4) {
            Some(SoftParam {
                soft: rng.random_bool(0.5),
                value: NonZeroU32::new(rng.random_range(1..=3)).unwrap(),
            })
        } else {
            None
        },
    }
}

pub fn settings(rng: &mut ChaCha8Rng, student_ids: &[StudentId]) -> Settings {
    Settings {
        global: limits(rng),
        students: subset(rng, student_ids, 0.2)
            .into_iter()
            .map(|id| (id, limits(rng)))
            .collect(),
    }
}

fn balancing_options(rng: &mut ChaCha8Rng) -> BalancingOptions {
    BalancingOptions {
        teacher_rotation: if rng.random_bool(0.5) {
            Some(SoftParam {
                soft: rng.random_bool(0.5),
                value: (),
            })
        } else {
            None
        },
        slot_rotation: if rng.random_bool(0.3) {
            Some(SoftParam {
                soft: rng.random_bool(0.5),
                value: (),
            })
        } else {
            None
        },
        avoid_twice_in_a_row: rng.random_bool(0.5),
        year_teacher_rotation: rng.random_bool(0.3),
        period_teacher_rotation: rng.random_bool(0.3),
    }
}

pub fn balancing(rng: &mut ChaCha8Rng, interrogation_subject_ids: &[SubjectId]) -> Balancing {
    Balancing {
        global: balancing_options(rng),
        subjects: subset(rng, interrogation_subject_ids, 0.3)
            .into_iter()
            .map(|id| (id, balancing_options(rng)))
            .collect(),
    }
}

pub fn pairing_rule(
    rng: &mut ChaCha8Rng,
    antecedent: SubjectId,
    consequent: SubjectId,
    period_ids: &[PeriodId],
) -> PairingRule {
    PairingRule {
        antecedent: RulePart {
            subject_id: antecedent,
            should_have: rng.random_bool(0.7),
        },
        consequent: RulePart {
            subject_id: consequent,
            should_have: rng.random_bool(0.7),
        },
        excluded_periods: subset(rng, period_ids, 0.15).into_iter().collect(),
        soft: rng.random_bool(0.5),
    }
}

pub fn slot_pairing_rule(
    rng: &mut ChaCha8Rng,
    antecedent: SlotId,
    consequent: SlotId,
    period_ids: &[PeriodId],
) -> SlotPairingRule {
    SlotPairingRule {
        antecedent: SlotRulePart {
            slot_id: antecedent,
            should_have: rng.random_bool(0.7),
        },
        consequent: SlotRulePart {
            slot_id: consequent,
            should_have: rng.random_bool(0.7),
        },
        excluded_periods: subset(rng, period_ids, 0.15).into_iter().collect(),
        soft: rng.random_bool(0.5),
    }
}

pub fn week_start(rng: &mut ChaCha8Rng) -> WeekStart {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 9, rng.random_range(1..=28)).unwrap();
    WeekStart::round_from(date)
}

fn color(rng: &mut ChaCha8Rng) -> export_config::Color {
    export_config::Color {
        red: rng.random_range(0..=255),
        green: rng.random_range(0..=255),
        blue: rng.random_range(0..=255),
    }
}

fn orientation(rng: &mut ChaCha8Rng) -> export_config::PageOrientation {
    if rng.random_bool(0.5) {
        export_config::PageOrientation::Portrait
    } else {
        export_config::PageOrientation::Landscape
    }
}

pub fn global_config(rng: &mut ChaCha8Rng) -> export_config::GlobalConfig {
    export_config::GlobalConfig {
        background_color: color(rng),
        stripes_color_enabled: rng.random_bool(0.5),
        stripes_color: color(rng),
    }
}

pub fn colloscope_config(rng: &mut ChaCha8Rng) -> export_config::ColloscopeConfig {
    export_config::ColloscopeConfig {
        sheet_name: format!("Sheet{}", rng.random_range(0..1000)),
        extra_info_column_enabled: rng.random_bool(0.5),
        extra_info_column_name: "Info".into(),
        teacher_email_enabled: rng.random_bool(0.5),
        teacher_email: "Contact".into(),
        teacher_tel_enabled: rng.random_bool(0.5),
        teacher_tel: String::new(),
        orientation: orientation(rng),
        display_week_dates: rng.random_bool(0.5),
        display_annotations: rng.random_bool(0.5),
        no_interrogation_color: color(rng),
        annotation_color_enabled: rng.random_bool(0.5),
        annotation_color: color(rng),
        extra_colors: BTreeMap::new(),
    }
}

pub fn per_student_groups_config(rng: &mut ChaCha8Rng) -> export_config::PerStudentGroupsConfig {
    export_config::PerStudentGroupsConfig {
        sheet_name: format!("Groups{}", rng.random_range(0..1000)),
        orientation: if rng.random_bool(0.5) {
            Some(orientation(rng))
        } else {
            None
        },
        show_emails: rng.random_bool(0.5),
        show_tel: rng.random_bool(0.5),
    }
}

pub fn per_group_list_config(rng: &mut ChaCha8Rng) -> export_config::PerGroupListConfig {
    export_config::PerGroupListConfig {
        orientation: orientation(rng),
        show_emails: rng.random_bool(0.5),
        show_tel: rng.random_bool(0.5),
        center_vertically: rng.random_bool(0.5),
    }
}
