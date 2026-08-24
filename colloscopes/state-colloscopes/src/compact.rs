//! Dense id renumbering ([InnerData::compact_ids])
//!
//! Ids tie the parts of a document together, and their values carry no meaning
//! of their own — so a document can always be renumbered, as long as every
//! occurrence of an id moves together. This module owns the complete walk over
//! every id occurrence of an [InnerData]: each domain module contributes a
//! `collect_ids`/`remap_ids` pair for its own container (they need access to
//! private fields), and this module drives them.
//!
//! The two methods of a pair must walk exactly the same fields. That is what
//! makes the map total: an id the collect pass misses is an id the remap pass
//! would not find in the map.

use crate::InnerData;
use crate::ids::Id;
use std::collections::{BTreeMap, BTreeSet};

/// The renumbering map, keyed by old raw id value
pub(crate) type IdMap = BTreeMap<u64, u64>;

/// Applies the map to one typed id
///
/// Panics if the id is not in the map — the map is always built by the very
/// walk that applies it, so a miss is a bug in this module.
pub(crate) fn remap<I: Id>(map: &IdMap, id: I) -> I {
    let new = *map
        .get(&id.inner())
        .expect("The map was built from this very walk, so it covers every id");
    // SAFETY: the map is injective (it is built from a `BTreeSet` zipped with
    // `0u64..`), so distinct ids stay distinct and no duplicate is created.
    unsafe { I::new(new) }
}

impl InnerData {
    /// Returns the same document with every id renumbered densely from 0, in
    /// ascending order of the old values
    ///
    /// The map is built over every id occurrence — defining and referencing
    /// alike, including the memory-only week ids — so the pass cannot fail. It
    /// is strictly monotone, so every relative order keyed by id is preserved,
    /// and injective, so it repairs nothing: a duplicated id stays duplicated
    /// and a dangling reference stays dangling. On a valid document the result
    /// is valid.
    ///
    /// This is how a document whose ids outgrew the file format's ceiling of
    /// 2^63 - 1 becomes writable again: densely renumbered, its ids fit with
    /// astronomical room to spare.
    pub fn compact_ids(self) -> InnerData {
        let mut ids = BTreeSet::new();
        self.collect_ids(&mut ids);
        let map: IdMap = ids.into_iter().zip(0u64..).collect();
        self.remap_ids(&map)
    }

    /// Inserts every id occurring anywhere in the document into `ids`
    fn collect_ids(&self, ids: &mut BTreeSet<u64>) {
        self.params.periods.collect_ids(ids);
        self.params.weeks.collect_ids(ids);
        self.params.subjects.collect_ids(ids);
        self.params.teachers.collect_ids(ids);
        self.params.students.collect_ids(ids);
        self.params.assignments.collect_ids(ids);
        self.params.week_patterns.collect_ids(ids);
        self.params.slots.collect_ids(ids);
        self.params.incompats.collect_ids(ids);
        self.params.group_lists.collect_ids(ids);
        self.params.settings.collect_ids(ids);
        self.params.pairings.collect_ids(ids);
        self.params.slot_pairings.collect_ids(ids);
        self.params.balancing.collect_ids(ids);
        self.colloscope.collect_ids(ids);
        // `export_config` holds no ids.
    }

    /// Rebuilds the document with every id occurrence sent through `map`
    fn remap_ids(self, map: &IdMap) -> InnerData {
        let InnerData {
            params,
            colloscope,
            export_config,
        } = self;
        InnerData {
            params: crate::colloscope_params::Parameters {
                periods: params.periods.remap_ids(map),
                weeks: params.weeks.remap_ids(map),
                subjects: params.subjects.remap_ids(map),
                teachers: params.teachers.remap_ids(map),
                students: params.students.remap_ids(map),
                assignments: params.assignments.remap_ids(map),
                week_patterns: params.week_patterns.remap_ids(map),
                slots: params.slots.remap_ids(map),
                incompats: params.incompats.remap_ids(map),
                group_lists: params.group_lists.remap_ids(map),
                settings: params.settings.remap_ids(map),
                pairings: params.pairings.remap_ids(map),
                slot_pairings: params.slot_pairings.remap_ids(map),
                balancing: params.balancing.remap_ids(map),
            },
            colloscope: colloscope.remap_ids(map),
            export_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Data;
    use crate::balancing::{Balancing, BalancingOptions};
    use crate::colloscope_params::Parameters;
    use crate::group_lists::{
        GroupList, GroupListFilling, GroupListParameters, GroupLists, PrefilledGroup,
    };
    use crate::ids::{
        GroupListId, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
        SubjectId, TeacherId, WeekId, WeekPatternId,
    };
    use crate::incompats::{Incompatibility, Incompats};
    use crate::non_empty_range::NonEmptyRangeInclusive;
    use crate::pairings::{PairingRule, Pairings, RulePart};
    use crate::periods::Periods;
    use crate::settings::{Limits, Settings};
    use crate::slot_pairings::{SlotPairingRule, SlotPairings, SlotRulePart};
    use crate::slots::{Slot, Slots};
    use crate::soft_param::SoftParam;
    use crate::students::{Student, Students};
    use crate::subjects::{
        Subject, SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity, Subjects,
    };
    use crate::teachers::{Teacher, Teachers};
    use crate::week_patterns::{WeekPattern, WeekPatterns};
    use crate::weeks::{WeekDesc, Weeks};
    use collomatique_time::{NonZeroMinutes, SlotStart, WholeMinuteTime};
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;

    /// How many distinct ids [`rich_document`] hands out
    const RICH_ID_COUNT: u64 = 19;

    /// Forges the `k`-th id of a [`rich_document`], under the given numbering
    fn forge<I: Id>(scale: &dyn Fn(u64) -> u64, k: u64) -> I {
        // SAFETY: `scale` is injective and each `k` is used for exactly one
        // entity, so no duplicate id is created.
        unsafe { I::new(scale(k)) }
    }

    fn slot_start(hour: u32) -> SlotStart {
        SlotStart {
            weekday: chrono::Weekday::Mon.into(),
            start_time: WholeMinuteTime::new(
                chrono::NaiveTime::from_hms_opt(hour, 0, 0).expect("valid time"),
            )
            .expect("whole-minute time"),
        }
    }

    fn interrogation_parameters() -> SubjectInterrogationParameters {
        SubjectInterrogationParameters {
            students_per_group: NonEmptyRangeInclusive::new(
                NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
            )
            .expect("statically non-empty"),
            groups_per_interrogation: NonEmptyRangeInclusive::new(
                NonZeroU32::new(1).unwrap()..=NonZeroU32::new(1).unwrap(),
            )
            .expect("statically non-empty"),
            duration: NonZeroMinutes::new(60).unwrap(),
            take_duration_into_account: true,
            periodicity: SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
            },
        }
    }

    /// A document that populates **every** row of the id inventory the walk is
    /// responsible for: both ordering mirrors, both composite-key mirrors, the
    /// association mirror's value, the two rule families, the two settings
    /// override tables and both colloscope tables.
    ///
    /// The `k`-th id it hands out is `scale(k)`, for `k` in
    /// `0..RICH_ID_COUNT`. Called with a strictly increasing `scale`, the
    /// documents it returns differ *only* by their ids — which is what makes
    /// the compaction test below an exact-value comparison rather than a
    /// property check.
    fn rich_document(scale: &dyn Fn(u64) -> u64) -> InnerData {
        let period_a: PeriodId = forge(scale, 0);
        let period_b: PeriodId = forge(scale, 1);
        let week_a: WeekId = forge(scale, 2);
        let week_b: WeekId = forge(scale, 3);
        let week_c: WeekId = forge(scale, 4);
        let subject_a: SubjectId = forge(scale, 5);
        let subject_b: SubjectId = forge(scale, 6);
        let teacher: TeacherId = forge(scale, 7);
        let student_a: StudentId = forge(scale, 8);
        let student_b: StudentId = forge(scale, 9);
        let student_c: StudentId = forge(scale, 10);
        let week_pattern: WeekPatternId = forge(scale, 11);
        let slot_a: SlotId = forge(scale, 12);
        let slot_b: SlotId = forge(scale, 13);
        let incompat: IncompatId = forge(scale, 14);
        let group_list_a: GroupListId = forge(scale, 15);
        let group_list_b: GroupListId = forge(scale, 16);
        let pairing_rule: PairingRuleId = forge(scale, 17);
        let slot_pairing_rule: SlotPairingRuleId = forge(scale, 18);

        let periods =
            Periods::from_ordered_ids(None, vec![period_a, period_b]).expect("distinct period ids");
        let weeks = Weeks::from_period_rows([
            (
                period_a,
                vec![(week_a, WeekDesc::new(true)), (week_b, WeekDesc::new(true))],
            ),
            (period_b, vec![(week_c, WeekDesc::new(false))]),
        ])
        .expect("distinct week ids");

        let subjects = Subjects {
            ordered_subject_list: vec![
                (
                    subject_a,
                    Subject {
                        parameters: SubjectParameters {
                            name: "Mathématiques".into(),
                            interrogation_parameters: Some(interrogation_parameters()),
                        },
                        excluded_periods: BTreeSet::new(),
                    },
                ),
                (
                    subject_b,
                    Subject {
                        parameters: SubjectParameters {
                            name: "Physique".into(),
                            interrogation_parameters: Some(interrogation_parameters()),
                        },
                        excluded_periods: BTreeSet::from([period_b]),
                    },
                ),
            ]
            .try_into()
            .expect("distinct subject ids"),
        };

        let teachers = Teachers {
            teacher_map: [(
                teacher,
                Teacher {
                    desc: crate::PersonWithContact {
                        surname: "Rogue".into(),
                        firstname: "Severus".into(),
                        tel: None,
                        email: None,
                    },
                    subjects: BTreeSet::from([subject_a, subject_b]),
                },
            )]
            .into_iter()
            .collect(),
        };

        let students = Students {
            student_map: [
                (
                    student_a,
                    Student {
                        desc: crate::PersonWithContact::default(),
                        excluded_periods: BTreeSet::new(),
                    },
                ),
                (
                    student_b,
                    Student {
                        desc: crate::PersonWithContact::default(),
                        excluded_periods: BTreeSet::new(),
                    },
                ),
                (
                    student_c,
                    Student {
                        desc: crate::PersonWithContact::default(),
                        excluded_periods: BTreeSet::from([period_b]),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let assignments = crate::assignments::Assignments {
            map: [(
                (period_a, subject_a),
                BTreeSet::from([student_a, student_b]),
            )]
            .into_iter()
            .collect(),
        };

        let week_patterns = WeekPatterns {
            week_pattern_map: [(
                week_pattern,
                WeekPattern {
                    name: "Semaines A".into(),
                    excluded_weeks: BTreeSet::from([week_b]),
                },
            )]
            .into_iter()
            .collect(),
        };

        let slots = Slots::from_subject_rows([(
            subject_a,
            vec![
                (
                    slot_a,
                    Slot {
                        subject_id: subject_a,
                        teacher_id: teacher,
                        start_time: slot_start(8),
                        extra_info: "Salle 101".into(),
                        week_pattern: Some(week_pattern),
                        cost: 0,
                    },
                ),
                (
                    slot_b,
                    Slot {
                        subject_id: subject_a,
                        teacher_id: teacher,
                        start_time: slot_start(10),
                        extra_info: String::new(),
                        week_pattern: None,
                        cost: 1,
                    },
                ),
            ],
        )])
        .expect("distinct slot ids");

        let incompats = Incompats {
            incompat_map: [(
                incompat,
                Incompatibility {
                    subject_id: subject_a,
                    name: "Sport".into(),
                    slots: vec![
                        collomatique_time::SlotWithDuration::new(
                            slot_start(14),
                            NonZeroMinutes::new(60).unwrap(),
                        )
                        .expect("does not cross midnight"),
                    ],
                    minimum_free_slots: NonZeroU32::new(1).unwrap(),
                    week_pattern_id: Some(week_pattern),
                },
            )]
            .into_iter()
            .collect(),
        };

        let group_lists = GroupLists {
            group_list_map: [
                (
                    group_list_a,
                    GroupList::new(
                        GroupListParameters {
                            name: "Groupes A".into(),
                            students_per_group: NonEmptyRangeInclusive::new(
                                NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
                            )
                            .expect("statically non-empty"),
                            group_names: vec![None, None],
                        },
                        GroupListFilling::Automatic {
                            excluded_students: BTreeSet::from([student_c]),
                        },
                    )
                    .expect("an automatic filling is always consistent"),
                ),
                (
                    group_list_b,
                    GroupList::new(
                        GroupListParameters {
                            name: "Groupes B".into(),
                            students_per_group: NonEmptyRangeInclusive::new(
                                NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
                            )
                            .expect("statically non-empty"),
                            group_names: vec![None, None],
                        },
                        GroupListFilling::Prefilled {
                            groups: vec![
                                PrefilledGroup {
                                    students: BTreeSet::from([student_a]),
                                },
                                PrefilledGroup {
                                    students: BTreeSet::from([student_b]),
                                },
                            ],
                        },
                    )
                    .expect("two groups for two names, no student twice"),
                ),
            ]
            .into_iter()
            .collect(),
            subjects_associations: [((period_a, subject_a), group_list_a)]
                .into_iter()
                .collect(),
        };

        let settings = Settings {
            global: Limits::default(),
            students: [(
                student_b,
                Limits {
                    interrogations_per_week_min: None,
                    interrogations_per_week_max: Some(SoftParam {
                        soft: false,
                        value: 3,
                    }),
                    max_interrogations_per_day: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        let pairings = Pairings {
            pairing_rule_map: [(
                pairing_rule,
                PairingRule::new(
                    RulePart {
                        subject_id: subject_a,
                        should_have: true,
                    },
                    RulePart {
                        subject_id: subject_b,
                        should_have: false,
                    },
                    BTreeSet::from([period_b]),
                    true,
                )
                .expect("distinct subjects"),
            )]
            .into_iter()
            .collect(),
        };

        let slot_pairings = SlotPairings {
            slot_pairing_rule_map: [(
                slot_pairing_rule,
                SlotPairingRule::new(
                    SlotRulePart {
                        slot_id: slot_a,
                        should_have: true,
                    },
                    SlotRulePart {
                        slot_id: slot_b,
                        should_have: true,
                    },
                    BTreeSet::from([period_a]),
                    false,
                )
                .expect("distinct slots"),
            )]
            .into_iter()
            .collect(),
        };

        let balancing = Balancing {
            global: BalancingOptions::default(),
            subjects: [(subject_a, BalancingOptions::default())]
                .into_iter()
                .collect(),
        };

        let mut colloscope = crate::colloscopes::Colloscope::default();
        colloscope.set_interrogation(slot_b, week_a, BTreeSet::from([0]));
        colloscope.set_group_list(
            group_list_a,
            BTreeMap::from([(student_a, 0), (student_b, 1)]),
        );

        InnerData {
            params: Parameters {
                periods,
                weeks,
                subjects,
                teachers,
                students,
                assignments,
                week_patterns,
                slots,
                incompats,
                group_lists,
                settings,
                pairings,
                slot_pairings,
                balancing,
            },
            colloscope,
            export_config: crate::export_config::ExportConfig::default(),
        }
    }

    /// The ids actually present in a document, as raw values
    fn ids_of(inner: &InnerData) -> BTreeSet<u64> {
        let mut ids = BTreeSet::new();
        inner.collect_ids(&mut ids);
        ids
    }

    #[test]
    fn the_rich_fixture_is_a_valid_document() {
        // The fixture is the yardstick of the tests below, so it has to be a
        // document the rest of the crate would accept — otherwise "compaction
        // preserves validity" would be vacuous.
        Data::from_inner_data(rich_document(&|k| k))
            .expect("the rich fixture should pass the invariant gate");
        assert_eq!(
            ids_of(&rich_document(&|k| k)),
            (0..RICH_ID_COUNT).collect::<BTreeSet<u64>>(),
            "the fixture hands out exactly RICH_ID_COUNT dense ids",
        );
    }

    #[test]
    fn sparse_ids_are_renumbered_densely_in_ascending_order() {
        // `3k + 5` is strictly increasing, so the k-th smallest id of the
        // spread document is the one the dense document numbers `k`: compacting
        // the spread document must produce the dense document *exactly*, field
        // for field. Any id occurrence the walk forgets keeps its spread value
        // and this equality fails — that is the coverage net.
        let spread = rich_document(&|k| 3 * k + 5);
        let dense = rich_document(&|k| k);

        assert_ne!(spread, dense, "the two numberings must really differ");
        assert_eq!(spread.compact_ids(), dense);
    }

    #[test]
    fn compaction_preserves_validity() {
        // Compaction moves every occurrence of an id together, so a valid
        // document stays valid: no reference is left behind, no mirror
        // desynchronizes and no two entities collide on one number. The gate
        // checks all three.
        let spread = rich_document(&|k| 3 * k + 5);
        Data::from_inner_data(spread.clone())
            .expect("the spread fixture should pass the invariant gate");
        Data::from_inner_data(spread.compact_ids()).expect("compaction should preserve validity");
    }

    #[test]
    fn an_already_dense_document_is_left_alone() {
        // A document numbered 0, 1, 2… is its own compaction: the map is the
        // identity. This is what makes the pass safe to run unconditionally.
        let dense = rich_document(&|k| k);
        assert_eq!(dense.clone().compact_ids(), dense);
    }

    #[test]
    fn week_ids_are_compacted_too() {
        // Week ids never reach a file — the file format stores weeks
        // positionally — but they share the in-memory id space, so they must be
        // renumbered along with everything else. Here the three weeks of the
        // spread document carry 11, 14 and 17; after compaction they sit at 2,
        // 3 and 4, inside the dense range like every other id.
        let compacted = rich_document(&|k| 3 * k + 5).compact_ids();

        let week_ids: Vec<u64> = compacted
            .params
            .week_ids()
            .map(|week_id| week_id.inner())
            .collect();
        assert_eq!(week_ids, vec![2, 3, 4]);
        assert_eq!(
            ids_of(&compacted),
            (0..RICH_ID_COUNT).collect::<BTreeSet<u64>>(),
        );
    }
}
