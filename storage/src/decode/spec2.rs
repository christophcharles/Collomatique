//! Spec-2 decode submodule
//!
//! This module builds a [Data] from the raw entries of a spec-2
//! document, in three layers:
//!
//! 1. [collect_blocks]: sort the entries into typed blocks. Unknown
//!    block names go through the forward-compatibility rules (spec §5);
//!    known block payloads are parsed with the format structs, which
//!    enforce every local validity rule.
//! 2. [reconstruct]: rebuild the in-memory
//!    [InnerData](collomatique_state_colloscopes::InnerData) from the
//!    blocks, completing everything the file deliberately omits (absent
//!    blocks, derived key sets, the colloscope interrogation skeleton).
//! 3. [Data::from_inner_data]: the invariant layer — the single trust
//!    boundary. The decoder inserts referentially-suspect rows anyway
//!    whenever this layer rejects them with a precise error.

use std::collections::{BTreeMap, BTreeSet};

use super::{Caveat, DecodeError};
use crate::format::{self, BlockName};
use crate::json::{CURRENT_SPEC_VERSION, RawEntry, Version};

use collomatique_state_colloscopes as mem;
use mem::ids::Id;
use mem::{
    Data, GroupListId, InnerData, InnerDataError, InvariantError, PairingRuleId, PeriodId, SlotId,
    SlotPairingRuleId, StudentId, SubjectId, TeacherId, WeekPatternId,
};

/// Every spec-2 block name declares these canonical envelope values
/// (spec §2); they are frozen with the block names themselves.
const CANONICAL_MINIMUM_SPEC_VERSION: u32 = 2;
const CANONICAL_NEEDED_ENTRY: bool = true;

pub fn decode(
    entries: &[RawEntry],
    version: &Version,
    caveats: &mut BTreeSet<Caveat>,
) -> Result<Data, DecodeError> {
    let blocks = collect_blocks(entries, version, caveats)?;
    let inner_data = reconstruct(blocks)?;
    Ok(Data::from_inner_data(inner_data)?)
}

/// The typed payloads of a document, at most one per block name
///
/// `None` means the block is absent: its default state.
#[derive(Default)]
struct Blocks {
    general_planning: Option<format::general_planning::GeneralPlanning>,
    subjects: Option<format::subjects::Subjects>,
    teachers: Option<format::teachers::Teachers>,
    students: Option<format::students::Students>,
    assignments: Option<format::assignments::Assignments>,
    week_patterns: Option<format::week_patterns::WeekPatterns>,
    slots: Option<format::slots::Slots>,
    incompatibilities: Option<format::incompatibilities::Incompatibilities>,
    group_lists: Option<format::group_lists::GroupLists>,
    group_list_associations: Option<format::group_list_associations::GroupListAssociations>,
    pairings: Option<format::pairings::Pairings>,
    slot_pairings: Option<format::slot_pairings::SlotPairings>,
    settings: Option<format::settings::Settings>,
    balancing: Option<format::balancing::Balancing>,
    colloscope: Option<format::colloscope::Colloscope>,
    export_config: Option<format::export_config::ExportConfig>,
}

fn store_block<T>(slot: &mut Option<T>, value: T, name: &'static str) -> Result<(), DecodeError> {
    if slot.is_some() {
        return Err(DecodeError::DuplicatedBlock(name));
    }
    *slot = Some(value);
    Ok(())
}

impl Blocks {
    fn store(&mut self, block: format::Block) -> Result<(), DecodeError> {
        let name = block.name().as_str();
        use format::Block;
        match block {
            Block::GeneralPlanning(b) => store_block(&mut self.general_planning, b, name),
            Block::Subjects(b) => store_block(&mut self.subjects, b, name),
            Block::Teachers(b) => store_block(&mut self.teachers, b, name),
            Block::Students(b) => store_block(&mut self.students, b, name),
            Block::Assignments(b) => store_block(&mut self.assignments, b, name),
            Block::WeekPatterns(b) => store_block(&mut self.week_patterns, b, name),
            Block::Slots(b) => store_block(&mut self.slots, b, name),
            Block::Incompatibilities(b) => store_block(&mut self.incompatibilities, b, name),
            Block::GroupLists(b) => store_block(&mut self.group_lists, b, name),
            Block::GroupListAssociations(b) => {
                store_block(&mut self.group_list_associations, b, name)
            }
            Block::Pairings(b) => store_block(&mut self.pairings, b, name),
            Block::SlotPairings(b) => store_block(&mut self.slot_pairings, b, name),
            Block::Settings(b) => store_block(&mut self.settings, b, name),
            Block::Balancing(b) => store_block(&mut self.balancing, b, name),
            Block::Colloscope(b) => store_block(&mut self.colloscope, b, name),
            Block::ExportConfig(b) => store_block(&mut self.export_config, b, name),
        }
    }
}

fn collect_blocks(
    entries: &[RawEntry],
    version: &Version,
    caveats: &mut BTreeSet<Caveat>,
) -> Result<Blocks, DecodeError> {
    let mut blocks = Blocks::default();

    for entry in entries {
        let content_map = serde_json::from_str::<BTreeMap<String, &serde_json::value::RawValue>>(
            entry.content.get(),
        )
        .map_err(|_| DecodeError::MalformedEntryContent)?;
        if content_map.len() != 1 {
            return Err(DecodeError::MalformedEntryContent);
        }
        let name = content_map
            .keys()
            .next()
            .expect("Content map has exactly one key");

        let Some(block_name) = BlockName::from_name(name) else {
            // Unknown block name: forward-compatibility rules (spec §5).
            // A block claiming a spec level we fully support cannot be
            // legitimately unknown — the name is probably a typo.
            if entry.minimum_spec_version <= CURRENT_SPEC_VERSION {
                return Err(DecodeError::ProbablyIllformedEntry);
            }
            if entry.needed_entry {
                return Err(DecodeError::UnknownNeededEntry(version.clone()));
            }
            caveats.insert(Caveat::UnknownEntries);
            continue;
        };

        if entry.minimum_spec_version != CANONICAL_MINIMUM_SPEC_VERSION
            || entry.needed_entry != CANONICAL_NEEDED_ENTRY
        {
            return Err(DecodeError::MismatchedSpecRequirementInEntry);
        }

        let block =
            serde_json::from_str::<format::Block>(entry.content.get()).map_err(|error| {
                DecodeError::IllformedBlock {
                    block: block_name.as_str(),
                    detail: error.to_string(),
                }
            })?;
        blocks.store(block)?;
    }

    Ok(blocks)
}

/// Builds an id from its file value
///
/// # Safety of the underlying call
///
/// Building unchecked ids is exactly the decoder's job: uniqueness and
/// referential validity of every id are checked by layer 3
/// ([Data::from_inner_data]) on the fully reconstructed data.
fn id<I: Id>(value: u64) -> I {
    unsafe { I::new(value) }
}

fn id_set<I: Id>(ids: crate::format::keyed::UniqueVec<u64>) -> BTreeSet<I> {
    ids.into_inner().into_iter().map(id).collect()
}

fn weekday(day: format::scalars::Weekday) -> collomatique_time::Weekday {
    use format::scalars::Weekday as FormatWeekday;
    collomatique_time::Weekday(match day {
        FormatWeekday::Monday => chrono::Weekday::Mon,
        FormatWeekday::Tuesday => chrono::Weekday::Tue,
        FormatWeekday::Wednesday => chrono::Weekday::Wed,
        FormatWeekday::Thursday => chrono::Weekday::Thu,
        FormatWeekday::Friday => chrono::Weekday::Fri,
        FormatWeekday::Saturday => chrono::Weekday::Sat,
        FormatWeekday::Sunday => chrono::Weekday::Sun,
    })
}

fn time_of_day(time: format::scalars::TimeOfDay) -> collomatique_time::WholeMinuteTime {
    collomatique_time::WholeMinuteTime::new(
        chrono::NaiveTime::from_hms_opt(time.hour().into(), time.minute().into(), 0)
            .expect("Format time of day is within range"),
    )
    .expect("Format time of day is on a whole minute")
}

fn range<T>(range: format::scalars::Range<T>) -> std::ops::RangeInclusive<T> {
    let (min, max) = range.into_min_max();
    min..=max
}

fn soft_param<T>(param: format::scalars::SoftParam<T>) -> mem::soft_param::SoftParam<T> {
    mem::soft_param::SoftParam {
        soft: param.soft,
        value: param.value,
    }
}

fn soft_flag(flag: format::scalars::SoftFlag) -> mem::soft_param::SoftParam<()> {
    mem::soft_param::SoftParam {
        soft: flag.soft,
        value: (),
    }
}

fn reconstruct(blocks: Blocks) -> Result<InnerData, DecodeError> {
    let periods = reconstruct_periods(blocks.general_planning.unwrap_or_default())?;
    let subjects = reconstruct_subjects(blocks.subjects.unwrap_or_default())?;
    let teachers = reconstruct_teachers(blocks.teachers.unwrap_or_default());
    let students = reconstruct_students(blocks.students.unwrap_or_default());
    let assignments = reconstruct_assignments(blocks.assignments.unwrap_or_default(), &periods)?;
    let week_patterns = reconstruct_week_patterns(blocks.week_patterns.unwrap_or_default());
    let slots = reconstruct_slots(blocks.slots.unwrap_or_default())?;
    let incompats = reconstruct_incompats(blocks.incompatibilities.unwrap_or_default())?;
    let group_lists = reconstruct_group_lists(
        blocks.group_lists.unwrap_or_default(),
        blocks.group_list_associations.unwrap_or_default(),
    );
    let settings = reconstruct_settings(blocks.settings.unwrap_or_default());
    let pairings = reconstruct_pairings(blocks.pairings.unwrap_or_default());
    let slot_pairings = reconstruct_slot_pairings(blocks.slot_pairings.unwrap_or_default());
    let balancing = reconstruct_balancing(blocks.balancing.unwrap_or_default());

    let params = mem::colloscope_params::Parameters {
        periods,
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
    };

    // The colloscope skeleton builder panics on parameters that violate
    // the invariants, so they must be checked first. This anticipates
    // (part of) layer 3 with the same errors.
    params.check_invariants().map_err(InnerDataError::from)?;

    let colloscope = reconstruct_colloscope(blocks.colloscope.unwrap_or_default(), &params)?;
    let export_config = reconstruct_export_config(blocks.export_config.unwrap_or_default());

    Ok(InnerData {
        params,
        colloscope,
        export_config,
    })
}

fn reconstruct_periods(
    block: format::general_planning::GeneralPlanning,
) -> Result<mem::periods::Periods, DecodeError> {
    let ordered_period_list = block
        .periods
        .into_iter()
        .map(|period| {
            (
                id(period.id),
                period
                    .weeks
                    .into_iter()
                    .map(|week| mem::periods::WeekDesc {
                        interrogations: week.interrogations,
                        annotation: week.annotation,
                    })
                    .collect(),
            )
        })
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| DecodeError::DuplicatedID)?;

    Ok(mem::periods::Periods {
        first_week: block.first_week.map(|date| {
            collomatique_time::WeekStart::new(date.date())
                .expect("Format week start date is a Monday")
        }),
        ordered_period_list,
    })
}

fn reconstruct_subjects(
    block: format::subjects::Subjects,
) -> Result<mem::subjects::Subjects, DecodeError> {
    let ordered_subject_list = block
        .into_iter()
        .map(|subject| {
            (
                id(subject.id),
                mem::subjects::Subject {
                    parameters: mem::subjects::SubjectParameters {
                        name: subject.name,
                        interrogation_parameters: subject
                            .interrogation_parameters
                            .map(interrogation_parameters),
                    },
                    excluded_periods: id_set(subject.excluded_periods),
                },
            )
        })
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| DecodeError::DuplicatedID)?;

    Ok(mem::subjects::Subjects {
        ordered_subject_list,
    })
}

fn interrogation_parameters(
    params: format::subjects::InterrogationParameters,
) -> mem::subjects::SubjectInterrogationParameters {
    mem::subjects::SubjectInterrogationParameters {
        students_per_group: range(params.students_per_group),
        groups_per_interrogation: range(params.groups_per_interrogation),
        duration: params.duration_minutes.get().into(),
        take_duration_into_account: params.take_duration_into_account,
        periodicity: periodicity(params.periodicity),
    }
}

fn periodicity(periodicity: format::subjects::Periodicity) -> mem::subjects::SubjectPeriodicity {
    use format::subjects::Periodicity as FormatPeriodicity;
    use mem::subjects::SubjectPeriodicity;
    match periodicity {
        FormatPeriodicity::OnceForEveryBlockOfWeeks(v) => {
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block: v.weeks_per_block,
                minimum_week_separation: v.minimum_week_separation,
            }
        }
        FormatPeriodicity::ExactlyPeriodic(v) => SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks: v.periodicity_in_weeks,
        },
        FormatPeriodicity::AmountInYear(v) => SubjectPeriodicity::AmountInYear {
            interrogation_count_in_year: range(v.interrogation_count_in_year),
            minimum_week_separation: v.minimum_week_separation,
        },
        FormatPeriodicity::AmountForEveryArbitraryBlock(v) => {
            SubjectPeriodicity::AmountForEveryArbitraryBlock {
                blocks: v
                    .blocks
                    .into_iter()
                    .map(|block| mem::subjects::WeekBlock {
                        delay_in_weeks: block.delay_in_weeks,
                        size_in_weeks: block.size_in_weeks,
                        interrogation_count_in_block: range(block.interrogation_count_in_block),
                    })
                    .collect(),
                minimum_week_separation: v.minimum_week_separation,
            }
        }
    }
}

fn reconstruct_teachers(block: format::teachers::Teachers) -> mem::teachers::Teachers {
    mem::teachers::Teachers {
        teacher_map: block
            .into_inner()
            .into_iter()
            .map(|teacher| {
                (
                    id::<TeacherId>(teacher.id),
                    mem::teachers::Teacher {
                        desc: mem::PersonWithContact {
                            surname: teacher.surname,
                            firstname: teacher.firstname,
                            tel: teacher.tel,
                            email: teacher.email,
                        },
                        subjects: id_set(teacher.subjects),
                    },
                )
            })
            .collect(),
    }
}

fn reconstruct_students(block: format::students::Students) -> mem::students::Students {
    mem::students::Students {
        student_map: block
            .into_inner()
            .into_iter()
            .map(|student| {
                (
                    id::<StudentId>(student.id),
                    mem::students::Student {
                        desc: mem::PersonWithContact {
                            surname: student.surname,
                            firstname: student.firstname,
                            tel: student.tel,
                            email: student.email,
                        },
                        excluded_periods: id_set(student.excluded_periods),
                    },
                )
            })
            .collect(),
    }
}

fn reconstruct_assignments(
    block: format::assignments::Assignments,
    periods: &mem::periods::Periods,
) -> Result<mem::assignments::Assignments, DecodeError> {
    // Sparse canonical form: a row is stored iff at least one student is
    // assigned (spec §4.5). An explicitly-empty row in the file decodes to an
    // absent row — the two are indistinguishable once loaded.
    let mut entries: Vec<((PeriodId, SubjectId), BTreeSet<StudentId>)> = Vec::new();
    for row in block.into_inner() {
        let period_id = id::<PeriodId>(row.period_id);
        if periods.find_period_position(period_id).is_none() {
            // Layer 3 rejects unknown period ids with this same error, but a
            // row on an unknown period could not otherwise be caught, so it is
            // rejected here.
            return Err(
                InnerDataError::Params(InvariantError::InvalidPeriodIdInAssignements).into(),
            );
        }
        let students = id_set(row.students);
        if students.is_empty() {
            // Neutral row: drop it to keep the canonical (absent) form.
            continue;
        }
        // A row on an unknown or excluded subject is inserted anyway:
        // layer 3 rejects it
        entries.push(((period_id, id(row.subject_id)), students));
    }

    Ok(mem::assignments::Assignments {
        map: entries.into_iter().collect(),
    })
}

fn reconstruct_week_patterns(
    block: format::week_patterns::WeekPatterns,
) -> mem::week_patterns::WeekPatterns {
    mem::week_patterns::WeekPatterns {
        week_pattern_map: block
            .into_inner()
            .into_iter()
            .map(|week_pattern| {
                (
                    id::<WeekPatternId>(week_pattern.id),
                    mem::week_patterns::WeekPattern {
                        name: week_pattern.name,
                        weeks: week_pattern.weeks,
                    },
                )
            })
            .collect(),
    }
}

fn reconstruct_slots(block: format::slots::Slots) -> Result<mem::slots::Slots, DecodeError> {
    // Sparse ordering: one row per subject that has slots. An explicitly-empty
    // row in the file (a redundant neutral entry of the derived key set, spec
    // §4.7) decodes to no row, matching the canonical absent form. Rows on
    // subjects without interrogations are inserted anyway: layer 3 rejects them.
    let mut rows: BTreeMap<SubjectId, Vec<(SlotId, mem::slots::Slot)>> = BTreeMap::new();

    for row in block.into_inner() {
        let subject_id = id::<SubjectId>(row.subject_id);
        let ordered_slots: Vec<(SlotId, mem::slots::Slot)> = row
            .slots
            .into_iter()
            .map(|slot| {
                (
                    id::<SlotId>(slot.id),
                    mem::slots::Slot {
                        subject_id,
                        teacher_id: id(slot.teacher_id),
                        start_time: collomatique_time::SlotStart {
                            weekday: weekday(slot.start.day),
                            start_time: time_of_day(slot.start.time),
                        },
                        extra_info: slot.extra_info,
                        week_pattern: slot.week_pattern_id.map(id),
                        cost: slot.cost,
                    },
                )
            })
            .collect();
        if ordered_slots.is_empty() {
            // Neutral row: drop it to keep the canonical (absent) form.
            continue;
        }
        rows.insert(subject_id, ordered_slots);
    }

    // A slot id duplicated across subjects would silently collapse the flat
    // slot table; detect it explicitly instead (it previously surfaced as a
    // duplicate-id invariant error).
    mem::slots::Slots::from_subject_rows(rows).map_err(|_| DecodeError::DuplicatedID)
}

fn reconstruct_incompats(
    block: format::incompatibilities::Incompatibilities,
) -> Result<mem::incompats::Incompats, DecodeError> {
    let mut incompat_map = BTreeMap::new();

    for row in block.into_inner() {
        let mut slots = Vec::new();
        for slot in row.slots {
            let start = collomatique_time::SlotStart {
                weekday: weekday(slot.day),
                start_time: time_of_day(slot.time),
            };
            // The only semantic check the decoder must do itself: the
            // in-memory type cannot represent a slot crossing midnight
            let Some(slot) =
                collomatique_time::SlotWithDuration::new(start, slot.duration_minutes.get().into())
            else {
                return Err(DecodeError::SlotCrossesMidnight);
            };
            slots.push(slot);
        }

        incompat_map.insert(
            id::<mem::IncompatId>(row.id),
            mem::incompats::Incompatibility {
                subject_id: id(row.subject_id),
                name: row.name,
                slots,
                minimum_free_slots: row.minimum_free_slots,
                week_pattern_id: row.week_pattern_id.map(id),
            },
        );
    }

    Ok(mem::incompats::Incompats {
        incompat_map: incompat_map.into(),
    })
}

fn reconstruct_group_lists(
    block: format::group_lists::GroupLists,
    associations: format::group_list_associations::GroupListAssociations,
) -> mem::group_lists::GroupLists {
    let group_list_map = block
        .into_inner()
        .into_iter()
        .map(|group_list| {
            let filling = match group_list.filling {
                format::group_lists::Filling::Prefilled(prefilled) => {
                    mem::group_lists::GroupListFilling::Prefilled {
                        groups: prefilled
                            .groups
                            .into_iter()
                            .map(|group| mem::group_lists::PrefilledGroup {
                                students: id_set(group.students),
                            })
                            .collect(),
                    }
                }
                format::group_lists::Filling::Automatic(automatic) => {
                    mem::group_lists::GroupListFilling::Automatic {
                        excluded_students: id_set(automatic.excluded_students),
                    }
                }
            };
            (
                id::<GroupListId>(group_list.id),
                mem::group_lists::GroupList {
                    params: mem::group_lists::GroupListParameters {
                        name: group_list.name,
                        students_per_group: range(group_list.students_per_group),
                        group_names: group_list.group_names,
                    },
                    filling,
                },
            )
        })
        .collect();

    // The associations table is sparse: one row per associated
    // `(period, subject)` (spec §4.10). Rows on an unknown period are kept
    // here and rejected by layer 3.
    let subjects_associations = associations
        .into_inner()
        .into_iter()
        .map(|row| {
            (
                (
                    id::<PeriodId>(row.period_id),
                    id::<SubjectId>(row.subject_id),
                ),
                id::<GroupListId>(row.group_list_id),
            )
        })
        .collect();

    mem::group_lists::GroupLists {
        group_list_map,
        subjects_associations,
    }
}

fn limits(limits: format::settings::Limits) -> mem::settings::Limits {
    mem::settings::Limits {
        interrogations_per_week_min: limits.interrogations_per_week_min.map(soft_param),
        interrogations_per_week_max: limits.interrogations_per_week_max.map(soft_param),
        max_interrogations_per_day: limits.max_interrogations_per_day.map(soft_param),
    }
}

fn reconstruct_settings(block: format::settings::Settings) -> mem::settings::Settings {
    mem::settings::Settings {
        global: limits(block.global),
        students: block
            .students
            .into_inner()
            .into_iter()
            .map(|row| (id::<StudentId>(row.student_id), limits(row.limits)))
            .collect(),
    }
}

fn reconstruct_pairings(block: format::pairings::Pairings) -> mem::pairings::Pairings {
    mem::pairings::Pairings {
        pairing_rule_map: block
            .into_inner()
            .into_iter()
            .map(|rule| {
                let part = |part: format::pairings::PairingPart| mem::pairings::RulePart {
                    subject_id: id(part.subject_id),
                    should_have: part.should_have,
                };
                (
                    id::<PairingRuleId>(rule.id),
                    mem::pairings::PairingRule {
                        antecedent: part(rule.antecedent),
                        consequent: part(rule.consequent),
                        excluded_periods: id_set(rule.excluded_periods),
                        soft: rule.soft,
                    },
                )
            })
            .collect(),
    }
}

fn reconstruct_slot_pairings(
    block: format::slot_pairings::SlotPairings,
) -> mem::slot_pairings::SlotPairings {
    mem::slot_pairings::SlotPairings {
        slot_pairing_rule_map: block
            .into_inner()
            .into_iter()
            .map(|rule| {
                let part = |part: format::slot_pairings::SlotPairingPart| {
                    mem::slot_pairings::SlotRulePart {
                        slot_id: id(part.slot_id),
                        should_have: part.should_have,
                    }
                };
                (
                    id::<SlotPairingRuleId>(rule.id),
                    mem::slot_pairings::SlotPairingRule {
                        antecedent: part(rule.antecedent),
                        consequent: part(rule.consequent),
                        excluded_periods: id_set(rule.excluded_periods),
                        soft: rule.soft,
                    },
                )
            })
            .collect(),
    }
}

fn balancing_options(options: format::balancing::Options) -> mem::balancing::BalancingOptions {
    mem::balancing::BalancingOptions {
        teacher_rotation: options.teacher_rotation.map(soft_flag),
        slot_rotation: options.slot_rotation.map(soft_flag),
        avoid_twice_in_a_row: options.avoid_twice_in_a_row,
        year_teacher_rotation: options.year_teacher_rotation,
        period_teacher_rotation: options.period_teacher_rotation,
    }
}

fn reconstruct_balancing(block: format::balancing::Balancing) -> mem::balancing::Balancing {
    mem::balancing::Balancing {
        global: balancing_options(block.global),
        subjects: block
            .subjects
            .into_inner()
            .into_iter()
            .map(|row| {
                (
                    id::<SubjectId>(row.subject_id),
                    balancing_options(row.options),
                )
            })
            .collect(),
    }
}

fn reconstruct_colloscope(
    block: format::colloscope::Colloscope,
    params: &mem::colloscope_params::Parameters,
) -> Result<mem::colloscopes::Colloscope, DecodeError> {
    // The whole key structure is derived (spec §4.15): the skeleton —
    // one entry per period, per slot running on it, with a Some/None
    // cell per week from the merged pattern, plus one group-list entry
    // per automatic group list — is rebuilt from the parameters, and
    // the sparse rows are placed onto it.
    let mut colloscope = mem::colloscopes::Colloscope::new_empty_from_params(params);

    // Global week index -> (period, week position within the period)
    let mut week_table = Vec::new();
    for (period_id, desc) in params.periods.ordered_period_list.iter() {
        for week_in_period in 0..desc.len() {
            week_table.push((period_id, week_in_period));
        }
    }

    for row in block.interrogations.into_inner() {
        let cell = usize::try_from(row.week)
            .ok()
            .and_then(|week| week_table.get(week).copied());
        let Some((period_id, week_in_period)) = cell else {
            // Week out of range
            return Err(DecodeError::InvalidInterrogationCell {
                slot_id: row.slot_id,
                week: row.week,
            });
        };

        let slot_id = id::<SlotId>(row.slot_id);
        if params.slots.find_slot(slot_id).is_none() {
            return Err(DecodeError::UnknownSlotInColloscope(row.slot_id));
        }

        let period = colloscope
            .period_map
            .get_mut(&period_id)
            .expect("Every period has an entry in the colloscope skeleton");
        let Some(slot) = period.slot_map.get_mut(&slot_id) else {
            // The slot exists but its subject does not run on the
            // period containing the week: the cell does not exist
            return Err(DecodeError::InvalidInterrogationCell {
                slot_id: row.slot_id,
                week: row.week,
            });
        };
        let Some(interrogation) = &mut slot.interrogations[week_in_period] else {
            // Inactive week: the week's interrogations flag or the
            // slot's week pattern is off
            return Err(DecodeError::InvalidInterrogationCell {
                slot_id: row.slot_id,
                week: row.week,
            });
        };
        interrogation.assigned_groups = row.assigned_groups.into_inner().into_iter().collect();
    }

    for row in block.group_lists.into_inner() {
        let Some(group_list) = colloscope
            .group_lists
            .get_mut(&id::<GroupListId>(row.group_list_id))
        else {
            // Unknown id, or a prefilled list (whose composition lives
            // in the GroupLists block): the skeleton has entries for
            // exactly the automatic group lists
            return Err(DecodeError::InvalidColloscopeGroupList(row.group_list_id));
        };
        group_list.groups_for_students = row
            .students
            .into_inner()
            .into_iter()
            .map(|placement| (id::<StudentId>(placement.student_id), placement.group))
            .collect();
    }

    Ok(colloscope)
}

fn color(color: format::scalars::Color) -> mem::export_config::Color {
    mem::export_config::Color {
        red: color.red,
        green: color.green,
        blue: color.blue,
    }
}

fn orientation(
    orientation: format::export_config::Orientation,
) -> mem::export_config::PageOrientation {
    match orientation {
        format::export_config::Orientation::Portrait => {
            mem::export_config::PageOrientation::Portrait
        }
        format::export_config::Orientation::Landscape => {
            mem::export_config::PageOrientation::Landscape
        }
    }
}

fn per_student_groups_config(
    config: format::export_config::PerStudentGroupsConfig,
) -> mem::export_config::PerStudentGroupsConfig {
    mem::export_config::PerStudentGroupsConfig {
        sheet_name: config.sheet_name,
        orientation: config.orientation.map(orientation),
        show_emails: config.show_emails,
        show_tel: config.show_tel,
    }
}

fn reconstruct_export_config(
    block: format::export_config::ExportConfig,
) -> mem::export_config::ExportConfig {
    mem::export_config::ExportConfig {
        global: mem::export_config::GlobalConfig {
            background_color: color(block.global.background_color),
            stripes_color_enabled: block.global.stripes_color_enabled,
            stripes_color: color(block.global.stripes_color),
        },
        colloscope_enabled: block.colloscope_enabled,
        all_groups_enabled: block.all_groups_enabled,
        automatic_groups_enabled: block.automatic_groups_enabled,
        prefilled_groups_enabled: block.prefilled_groups_enabled,
        per_group_list_enabled: block.per_group_list_enabled,
        colloscope_config: mem::export_config::ColloscopeConfig {
            sheet_name: block.colloscope_config.sheet_name,
            extra_info_column_enabled: block.colloscope_config.extra_info_column_enabled,
            extra_info_column_name: block.colloscope_config.extra_info_column_name,
            teacher_email_enabled: block.colloscope_config.teacher_email_enabled,
            teacher_email: block.colloscope_config.teacher_email,
            teacher_tel_enabled: block.colloscope_config.teacher_tel_enabled,
            teacher_tel: block.colloscope_config.teacher_tel,
            orientation: orientation(block.colloscope_config.orientation),
            display_week_dates: block.colloscope_config.display_week_dates,
            display_annotations: block.colloscope_config.display_annotations,
            no_interrogation_color: color(block.colloscope_config.no_interrogation_color),
            annotation_color_enabled: block.colloscope_config.annotation_color_enabled,
            annotation_color: color(block.colloscope_config.annotation_color),
            extra_colors: block
                .colloscope_config
                .extra_colors
                .into_inner()
                .into_iter()
                .map(|extra| (extra.name, color(extra.color)))
                .collect(),
        },
        all_groups_config: per_student_groups_config(block.all_groups_config),
        automatic_groups_config: per_student_groups_config(block.automatic_groups_config),
        prefilled_groups_config: per_student_groups_config(block.prefilled_groups_config),
        per_group_list_config: mem::export_config::PerGroupListConfig {
            orientation: orientation(block.per_group_list_config.orientation),
            show_emails: block.per_group_list_config.show_emails,
            show_tel: block.per_group_list_config.show_tel,
            center_vertically: block.per_group_list_config.center_vertically,
        },
    }
}
