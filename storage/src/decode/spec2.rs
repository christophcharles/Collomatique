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
//!    blocks, derived key sets).
//! 3. [Data::from_inner_data]: the invariant layer — the last line of
//!    defence, which makes the result safe whatever layer 2 let through.
//!    Layer 2 should still diagnose every constraint it can name (the
//!    "honest decode" checks below); where it does not yet, the user
//!    gets a poor message and that is a bug to close — see
//!    `docs/todos/fixme_spec2_storage.md`. The target is a layer 2 that
//!    lets nothing broken through, so that a rejection here means this
//!    crate built an `InnerData` it should not have.

use std::collections::{BTreeMap, BTreeSet};

use super::{Caveat, DecodeError, IdKind, RowKey};
use crate::format::{self, BlockName};
use crate::json::{CURRENT_SPEC_VERSION, RawEntry, Version};

use collomatique_state_colloscopes as mem;
use mem::ids::Id;
use mem::{
    Data, GroupListId, InnerData, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekId, WeekPatternId,
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
            return Err(DecodeError::MismatchedSpecRequirementInEntry(
                block_name.as_str(),
            ));
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

fn range<T: Ord + Clone>(range: format::scalars::Range<T>) -> mem::NonEmptyRangeInclusive<T> {
    let (min, max) = range.into_min_max();
    mem::NonEmptyRangeInclusive::new(min..=max)
        .expect("format::scalars::Range guarantees min <= max")
}

fn soft_param<T>(param: format::scalars::SoftParam<T>) -> mem::soft_param::SoftParam<T> {
    mem::soft_param::SoftParam {
        soft: param.soft,
        value: param.value,
    }
}

/// Walks every *defining* id of the document — the ids that create an
/// entity, not the ids that reference one — with the name of the block
/// that defines it, in the canonical §2 block order. The ten defining
/// kinds are: periods, subjects, teachers, students, week patterns,
/// slots, incompatibilities, group lists, pairing rules, slot pairing
/// rules.
///
/// Week ids are never serialized (the file stores weeks positionally), so
/// decode synthesizes them; they must be assigned above every id the file
/// *does* define. In a valid file every reference id is also a defining id, so
/// the defining ids of each block bound the whole id space; a reference id
/// outside that set is dangling and rejected as such regardless.
fn for_each_defining_id(blocks: &Blocks, f: &mut impl FnMut(&'static str, u64)) {
    if let Some(b) = &blocks.general_planning {
        for period in &b.periods {
            f(BlockName::GeneralPlanning.as_str(), period.id);
        }
    }
    if let Some(b) = &blocks.subjects {
        for subject in b.iter() {
            f(BlockName::Subjects.as_str(), subject.id);
        }
    }
    if let Some(b) = &blocks.teachers {
        for teacher in b.iter() {
            f(BlockName::Teachers.as_str(), teacher.id);
        }
    }
    if let Some(b) = &blocks.students {
        for student in b.iter() {
            f(BlockName::Students.as_str(), student.id);
        }
    }
    if let Some(b) = &blocks.week_patterns {
        for week_pattern in b.iter() {
            f(BlockName::WeekPatterns.as_str(), week_pattern.id);
        }
    }
    if let Some(b) = &blocks.slots {
        for subject_slots in b.iter() {
            for slot in &subject_slots.slots {
                f(BlockName::Slots.as_str(), slot.id);
            }
        }
    }
    if let Some(b) = &blocks.incompatibilities {
        for incompat in b.iter() {
            f(BlockName::Incompatibilities.as_str(), incompat.id);
        }
    }
    if let Some(b) = &blocks.group_lists {
        for group_list in b.iter() {
            f(BlockName::GroupLists.as_str(), group_list.id);
        }
    }
    if let Some(b) = &blocks.pairings {
        for pairing in b.iter() {
            f(BlockName::Pairings.as_str(), pairing.id);
        }
    }
    if let Some(b) = &blocks.slot_pairings {
        for slot_pairing in b.iter() {
            f(BlockName::SlotPairings.as_str(), slot_pairing.id);
        }
    }
}

fn reconstruct(blocks: Blocks) -> Result<InnerData, DecodeError> {
    // The two id-space rules of spec §3 in one walk over the defining ids:
    // every id is at most 2^63 - 1, and an id value is defined at most once
    // across the whole file. Same-block duplicates keep the established
    // per-block diagnostic.
    let mut seen: BTreeMap<u64, &'static str> = BTreeMap::new();
    let mut first_error = None;
    for_each_defining_id(&blocks, &mut |block, id| {
        if first_error.is_some() {
            return;
        }
        if id > (u64::MAX >> 1) {
            first_error = Some(DecodeError::IdAboveCeiling { block, id });
            return;
        }
        if let Some(&first) = seen.get(&id) {
            first_error = Some(if first == block {
                DecodeError::DuplicatedIdInBlock { block, id }
            } else {
                DecodeError::DuplicatedIdAcrossBlocks {
                    first,
                    second: block,
                    id,
                }
            });
            return;
        }
        seen.insert(id, block);
    });
    if let Some(error) = first_error {
        return Err(error);
    }
    // Week ids are synthesized above every id the file defines (S11). All
    // defining ids are at most 2^63 - 1 here, so the synthesized ids stay
    // well inside u64; `saturating_add` is kept as a cost-free safety.
    let mut next_week_id = seen
        .keys()
        .next_back()
        .copied()
        .unwrap_or(0)
        .saturating_add(1);
    let (periods, weeks) = reconstruct_periods(
        blocks.general_planning.unwrap_or_default(),
        &mut next_week_id,
    )?;
    let subjects = reconstruct_subjects(blocks.subjects.unwrap_or_default(), &periods)?;
    let teachers = reconstruct_teachers(blocks.teachers.unwrap_or_default(), &subjects)?;
    let students = reconstruct_students(blocks.students.unwrap_or_default(), &periods)?;
    let assignments = reconstruct_assignments(
        blocks.assignments.unwrap_or_default(),
        &periods,
        &subjects,
        &students,
    )?;
    let week_patterns =
        reconstruct_week_patterns(blocks.week_patterns.unwrap_or_default(), &weeks, &periods)?;
    let slots = reconstruct_slots(
        blocks.slots.unwrap_or_default(),
        &subjects,
        &teachers,
        &week_patterns,
    )?;
    let incompats = reconstruct_incompats(
        blocks.incompatibilities.unwrap_or_default(),
        &subjects,
        &week_patterns,
    )?;
    let group_lists = reconstruct_group_lists(
        blocks.group_lists.unwrap_or_default(),
        blocks.group_list_associations.unwrap_or_default(),
        &periods,
        &subjects,
        &students,
    )?;
    let settings = reconstruct_settings(blocks.settings.unwrap_or_default(), &students)?;
    let pairings = reconstruct_pairings(blocks.pairings.unwrap_or_default(), &subjects, &periods)?;
    let slot_pairings =
        reconstruct_slot_pairings(blocks.slot_pairings.unwrap_or_default(), &slots, &periods)?;
    let balancing = reconstruct_balancing(blocks.balancing.unwrap_or_default(), &subjects)?;

    let params = mem::colloscope_params::Parameters {
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
    };

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
    next_week_id: &mut u64,
) -> Result<(mem::periods::Periods, mem::weeks::Weeks), DecodeError> {
    let first_week = block.first_week.map(|date| {
        collomatique_time::WeekStart::new(date.date()).expect("Format week start date is a Monday")
    });
    let rows = block
        .periods
        .into_iter()
        .map(|period| {
            (
                id(period.id),
                period
                    .weeks
                    .into_iter()
                    .map(|week| {
                        // Synthesize the week's id in walk order (S11).
                        let week_id = id::<WeekId>(*next_week_id);
                        *next_week_id = next_week_id.saturating_add(1);
                        (
                            week_id,
                            mem::weeks::WeekDesc {
                                interrogations: week.interrogations,
                                annotation: week.annotation,
                            },
                        )
                    })
                    .collect(),
            )
        })
        .collect::<Vec<_>>();

    // The period set is exactly the week-row keys by construction, so the two
    // containers are built from the same rows: periods carry only the ordered
    // ids, weeks carry the per-period ordering and the week table.
    let period_ids = rows.iter().map(|(id, _)| *id).collect();
    let periods = mem::periods::Periods::from_ordered_ids(first_week, period_ids).map_err(|e| {
        DecodeError::DuplicatedIdInBlock {
            block: BlockName::GeneralPlanning.as_str(),
            id: e.0.inner(),
        }
    })?;
    // Defensive: the week ids were synthesized just above by a monotonically
    // increasing counter, so no file can make them collide. The reported id
    // is that synthesized value, not one the file carried.
    let weeks = mem::weeks::Weeks::from_period_rows(rows).map_err(|e| {
        DecodeError::DuplicatedIdInBlock {
            block: BlockName::GeneralPlanning.as_str(),
            id: e.0.inner(),
        }
    })?;
    Ok((periods, weeks))
}

fn reconstruct_subjects(
    block: format::subjects::Subjects,
    periods: &mem::periods::Periods,
) -> Result<mem::subjects::Subjects, DecodeError> {
    let mut rows = Vec::new();
    for subject in block {
        for &period_raw in subject.excluded_periods.iter() {
            if periods
                .find_period_position(id::<PeriodId>(period_raw))
                .is_none()
            {
                return Err(DecodeError::DanglingReference {
                    block: BlockName::Subjects.as_str(),
                    row: RowKey::Id(subject.id),
                    referenced: IdKind::Period,
                    id: period_raw,
                });
            }
        }
        rows.push((
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
        ));
    }
    let ordered_subject_list = rows.try_into().map_err(
        |e: collomatique_state::tables::DuplicatedIdError<SubjectId>| {
            DecodeError::DuplicatedIdInBlock {
                block: BlockName::Subjects.as_str(),
                id: e.0.inner(),
            }
        },
    )?;

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

fn reconstruct_teachers(
    block: format::teachers::Teachers,
    subjects: &mem::subjects::Subjects,
) -> Result<mem::teachers::Teachers, DecodeError> {
    let mut teacher_map = BTreeMap::new();
    for teacher in block.into_inner() {
        for &subject_raw in teacher.subjects.iter() {
            if subjects
                .find_subject(id::<SubjectId>(subject_raw))
                .is_none()
            {
                return Err(DecodeError::DanglingReference {
                    block: BlockName::Teachers.as_str(),
                    row: RowKey::Id(teacher.id),
                    referenced: IdKind::Subject,
                    id: subject_raw,
                });
            }
        }
        teacher_map.insert(
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
        );
    }
    Ok(mem::teachers::Teachers {
        teacher_map: teacher_map.into(),
    })
}

fn reconstruct_students(
    block: format::students::Students,
    periods: &mem::periods::Periods,
) -> Result<mem::students::Students, DecodeError> {
    let mut student_map = BTreeMap::new();
    for student in block.into_inner() {
        for &period_raw in student.excluded_periods.iter() {
            if periods
                .find_period_position(id::<PeriodId>(period_raw))
                .is_none()
            {
                return Err(DecodeError::DanglingReference {
                    block: BlockName::Students.as_str(),
                    row: RowKey::Id(student.id),
                    referenced: IdKind::Period,
                    id: period_raw,
                });
            }
        }
        student_map.insert(
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
        );
    }
    Ok(mem::students::Students {
        student_map: student_map.into(),
    })
}

fn reconstruct_assignments(
    block: format::assignments::Assignments,
    periods: &mem::periods::Periods,
    subjects: &mem::subjects::Subjects,
    students: &mem::students::Students,
) -> Result<mem::assignments::Assignments, DecodeError> {
    // Sparse canonical form: a row is stored iff at least one student is
    // assigned (spec §4.5). An explicitly-empty row in the file decodes to an
    // absent row — the two are indistinguishable once loaded.
    let mut entries: Vec<((PeriodId, SubjectId), BTreeSet<StudentId>)> = Vec::new();
    for row in block.into_inner() {
        let period_id = id::<PeriodId>(row.period_id);
        if periods.find_period_position(period_id).is_none() {
            // An empty assignments row keyed by an unknown period decodes to an
            // absent row (canonical-absent rule) and would otherwise vanish
            // silently before the final gate can see it, so it is rejected here.
            return Err(DecodeError::UnknownPeriodInAssignments(row.period_id));
        }
        // Same reasoning for the subject half of the key: an empty row keyed
        // outside the derived set (unknown subject, or subject excluded from
        // this period) would vanish before the final gate can see it (§4.5).
        let subject_id = id::<SubjectId>(row.subject_id);
        let Some(subject) = subjects.find_subject(subject_id) else {
            return Err(DecodeError::UnknownSubjectInAssignments(row.subject_id));
        };
        if subject.excluded_periods.contains(&period_id) {
            return Err(DecodeError::AssignmentOnExcludedPeriod {
                period_id: row.period_id,
                subject_id: row.subject_id,
            });
        }
        for &student_raw in row.students.iter() {
            if students
                .student_map
                .get(&id::<StudentId>(student_raw))
                .is_none()
            {
                return Err(DecodeError::DanglingReference {
                    block: BlockName::Assignments.as_str(),
                    row: RowKey::PeriodSubject {
                        period_id: row.period_id,
                        subject_id: row.subject_id,
                    },
                    referenced: IdKind::Student,
                    id: student_raw,
                });
            }
        }
        let assigned_students = id_set(row.students);
        if assigned_students.is_empty() {
            // Neutral row: drop it to keep the canonical (absent) form.
            continue;
        }
        entries.push(((period_id, subject_id), assigned_students));
    }

    Ok(mem::assignments::Assignments {
        map: entries.into_iter().collect(),
    })
}

fn reconstruct_week_patterns(
    block: format::week_patterns::WeekPatterns,
    weeks: &mem::weeks::Weeks,
    periods: &mem::periods::Periods,
) -> Result<mem::week_patterns::WeekPatterns, DecodeError> {
    // The frozen positional bitmask carries one bit per week in global walk
    // order; a `false` bit excludes that week. The spec (§4.6) requires
    // exactly one element per week of the schedule — no shorter, no longer —
    // and the in-memory type has no length to re-check later, so the length
    // is enforced here.
    let week_ids: Vec<WeekId> = weeks
        .walk(periods)
        .map(|(_period_id, week_id, _week)| week_id)
        .collect();
    let week_pattern_map = block
        .into_inner()
        .into_iter()
        .map(|week_pattern| {
            if week_pattern.weeks.len() != week_ids.len() {
                return Err(DecodeError::WrongWeekCountInWeekPattern {
                    week_pattern_id: week_pattern.id,
                    expected: week_ids.len(),
                    found: week_pattern.weeks.len(),
                });
            }
            let excluded_weeks = week_ids
                .iter()
                .zip(week_pattern.weeks)
                .filter_map(|(&week_id, active)| (!active).then_some(week_id))
                .collect();
            Ok((
                id::<WeekPatternId>(week_pattern.id),
                mem::week_patterns::WeekPattern {
                    name: week_pattern.name,
                    excluded_weeks,
                },
            ))
        })
        .collect::<Result<_, _>>()?;

    Ok(mem::week_patterns::WeekPatterns { week_pattern_map })
}

fn reconstruct_slots(
    block: format::slots::Slots,
    subjects: &mem::subjects::Subjects,
    teachers: &mem::teachers::Teachers,
    week_patterns: &mem::week_patterns::WeekPatterns,
) -> Result<mem::slots::Slots, DecodeError> {
    // Sparse ordering: one row per subject that has slots. An explicitly-empty
    // row in the file (a redundant neutral entry of the derived key set, spec
    // §4.7) decodes to no row, matching the canonical absent form — which is
    // why the row key is validated first, below.
    let mut rows: BTreeMap<SubjectId, Vec<(SlotId, mem::slots::Slot)>> = BTreeMap::new();

    for row in block.into_inner() {
        let subject_id = id::<SubjectId>(row.subject_id);
        // The derived key set is "subjects with interrogations" (§4.7). An
        // empty row keyed outside it would decode to absence and vanish
        // before the final gate can see it, so it is rejected here.
        let Some(subject) = subjects.find_subject(subject_id) else {
            return Err(DecodeError::UnknownSubjectInSlots(row.subject_id));
        };
        if subject.parameters.interrogation_parameters.is_none() {
            return Err(DecodeError::SlotsForSubjectWithoutInterrogations(
                row.subject_id,
            ));
        }
        let mut ordered_slots: Vec<(SlotId, mem::slots::Slot)> = Vec::new();
        for slot in row.slots {
            if teachers
                .teacher_map
                .get(&id::<TeacherId>(slot.teacher_id))
                .is_none()
            {
                return Err(DecodeError::DanglingReference {
                    block: BlockName::Slots.as_str(),
                    row: RowKey::Id(slot.id),
                    referenced: IdKind::Teacher,
                    id: slot.teacher_id,
                });
            }
            if let Some(week_pattern_raw) = slot.week_pattern_id {
                if week_patterns
                    .week_pattern_map
                    .get(&id::<WeekPatternId>(week_pattern_raw))
                    .is_none()
                {
                    return Err(DecodeError::DanglingReference {
                        block: BlockName::Slots.as_str(),
                        row: RowKey::Id(slot.id),
                        referenced: IdKind::WeekPattern,
                        id: week_pattern_raw,
                    });
                }
            }
            ordered_slots.push((
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
            ));
        }
        if ordered_slots.is_empty() {
            // Neutral row: drop it to keep the canonical (absent) form.
            continue;
        }
        rows.insert(subject_id, ordered_slots);
    }

    // A slot id duplicated across subjects would silently collapse the flat
    // slot table; detect it explicitly instead (it previously surfaced as a
    // duplicate-id invariant error).
    mem::slots::Slots::from_subject_rows(rows).map_err(|e| DecodeError::DuplicatedIdInBlock {
        block: BlockName::Slots.as_str(),
        id: e.0.inner(),
    })
}

fn reconstruct_incompats(
    block: format::incompatibilities::Incompatibilities,
    subjects: &mem::subjects::Subjects,
    week_patterns: &mem::week_patterns::WeekPatterns,
) -> Result<mem::incompats::Incompats, DecodeError> {
    let mut incompat_map = BTreeMap::new();

    for row in block.into_inner() {
        if subjects
            .find_subject(id::<SubjectId>(row.subject_id))
            .is_none()
        {
            return Err(DecodeError::DanglingReference {
                block: BlockName::Incompatibilities.as_str(),
                row: RowKey::Id(row.id),
                referenced: IdKind::Subject,
                id: row.subject_id,
            });
        }
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

        if let Some(week_pattern_raw) = row.week_pattern_id {
            if week_patterns
                .week_pattern_map
                .get(&id::<WeekPatternId>(week_pattern_raw))
                .is_none()
            {
                return Err(DecodeError::DanglingReference {
                    block: BlockName::Incompatibilities.as_str(),
                    row: RowKey::Id(row.id),
                    referenced: IdKind::WeekPattern,
                    id: week_pattern_raw,
                });
            }
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
    periods: &mem::periods::Periods,
    subjects: &mem::subjects::Subjects,
    students: &mem::students::Students,
) -> Result<mem::group_lists::GroupLists, DecodeError> {
    let mut group_list_map = BTreeMap::new();
    for group_list in block.into_inner() {
        let raw_id = group_list.id;
        // The student ids of the filling, kept raw so they can be checked
        // after the internal seal below (same order as the pairings block:
        // seal first, then references).
        let raw_students: Vec<u64> = match &group_list.filling {
            format::group_lists::Filling::Prefilled(prefilled) => prefilled
                .groups
                .iter()
                .flat_map(|group| group.students.iter().copied())
                .collect(),
            format::group_lists::Filling::Automatic(automatic) => {
                automatic.excluded_students.iter().copied().collect()
            }
        };
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
        let params = mem::group_lists::GroupListParameters {
            name: group_list.name,
            students_per_group: range(group_list.students_per_group),
            group_names: group_list.group_names,
        };
        // Honest decode: an inconsistent (params, filling) pair is a hard
        // error here rather than being caught later by layer 3.
        let value = mem::group_lists::GroupList::new(params, filling)
            .map_err(|_| DecodeError::InconsistentGroupList(raw_id))?;
        for student_raw in raw_students {
            if students
                .student_map
                .get(&id::<StudentId>(student_raw))
                .is_none()
            {
                return Err(DecodeError::DanglingReference {
                    block: BlockName::GroupLists.as_str(),
                    row: RowKey::Id(raw_id),
                    referenced: IdKind::Student,
                    id: student_raw,
                });
            }
        }
        group_list_map.insert(id::<GroupListId>(raw_id), value);
    }

    // The associations table is sparse: one row per associated
    // `(period, subject)` (spec §4.10).
    let mut subjects_associations = BTreeMap::new();
    for row in associations.into_inner() {
        let row_key = RowKey::PeriodSubject {
            period_id: row.period_id,
            subject_id: row.subject_id,
        };
        if periods
            .find_period_position(id::<PeriodId>(row.period_id))
            .is_none()
        {
            return Err(DecodeError::DanglingReference {
                block: BlockName::GroupListAssociations.as_str(),
                row: row_key,
                referenced: IdKind::Period,
                id: row.period_id,
            });
        }
        if subjects
            .find_subject(id::<SubjectId>(row.subject_id))
            .is_none()
        {
            return Err(DecodeError::DanglingReference {
                block: BlockName::GroupListAssociations.as_str(),
                row: row_key,
                referenced: IdKind::Subject,
                id: row.subject_id,
            });
        }
        if !group_list_map.contains_key(&id::<GroupListId>(row.group_list_id)) {
            return Err(DecodeError::DanglingReference {
                block: BlockName::GroupListAssociations.as_str(),
                row: row_key,
                referenced: IdKind::GroupList,
                id: row.group_list_id,
            });
        }
        subjects_associations.insert(
            (
                id::<PeriodId>(row.period_id),
                id::<SubjectId>(row.subject_id),
            ),
            id::<GroupListId>(row.group_list_id),
        );
    }

    Ok(mem::group_lists::GroupLists {
        group_list_map: group_list_map.into(),
        subjects_associations: subjects_associations.into(),
    })
}

fn limits(limits: format::settings::Limits) -> mem::settings::Limits {
    mem::settings::Limits {
        interrogations_per_week_min: limits.interrogations_per_week_min.map(soft_param),
        interrogations_per_week_max: limits.interrogations_per_week_max.map(soft_param),
        max_interrogations_per_day: limits.max_interrogations_per_day.map(soft_param),
    }
}

fn reconstruct_settings(
    block: format::settings::Settings,
    students: &mem::students::Students,
) -> Result<mem::settings::Settings, DecodeError> {
    let mut per_student = BTreeMap::new();
    for row in block.students.into_inner() {
        if students
            .student_map
            .get(&id::<StudentId>(row.student_id))
            .is_none()
        {
            return Err(DecodeError::DanglingReference {
                block: BlockName::Settings.as_str(),
                row: RowKey::Id(row.student_id),
                referenced: IdKind::Student,
                id: row.student_id,
            });
        }
        per_student.insert(id::<StudentId>(row.student_id), limits(row.limits));
    }
    Ok(mem::settings::Settings {
        global: limits(block.global),
        students: per_student.into(),
    })
}

fn reconstruct_pairings(
    block: format::pairings::Pairings,
    subjects: &mem::subjects::Subjects,
    periods: &mem::periods::Periods,
) -> Result<mem::pairings::Pairings, DecodeError> {
    let pairing_rule_map = block
        .into_inner()
        .into_iter()
        .map(|rule| {
            let raw_id = rule.id;
            let raw_subjects = [rule.antecedent.subject_id, rule.consequent.subject_id];
            let raw_excluded_periods: Vec<u64> = rule.excluded_periods.iter().copied().collect();
            let part = |part: format::pairings::PairingPart| mem::pairings::RulePart {
                subject_id: id(part.subject_id),
                should_have: part.should_have,
            };
            // Honest decode: a rule with both parts on one subject is a hard
            // error here rather than being caught later by layer 3.
            let value = mem::pairings::PairingRule::new(
                part(rule.antecedent),
                part(rule.consequent),
                id_set(rule.excluded_periods),
                rule.soft,
            )
            .map_err(|_| DecodeError::InconsistentPairingRule(raw_id))?;
            // Same honesty for §4.11's two subject constraints. Antecedent
            // first, then consequent, matching the scan order the `ops`
            // pairing errors publish, so both layers blame the same part of a
            // rule that is wrong on both sides.
            for raw_subject in raw_subjects {
                let Some(subject) = subjects.find_subject(id::<SubjectId>(raw_subject)) else {
                    return Err(DecodeError::DanglingReference {
                        block: BlockName::Pairings.as_str(),
                        row: RowKey::Id(raw_id),
                        referenced: IdKind::Subject,
                        id: raw_subject,
                    });
                };
                if subject.parameters.interrogation_parameters.is_none() {
                    return Err(DecodeError::PairingRuleForSubjectWithoutInterrogations {
                        rule_id: raw_id,
                        subject_id: raw_subject,
                    });
                }
            }
            for &period_raw in &raw_excluded_periods {
                if periods
                    .find_period_position(id::<PeriodId>(period_raw))
                    .is_none()
                {
                    return Err(DecodeError::DanglingReference {
                        block: BlockName::Pairings.as_str(),
                        row: RowKey::Id(raw_id),
                        referenced: IdKind::Period,
                        id: period_raw,
                    });
                }
            }
            Ok((id::<PairingRuleId>(raw_id), value))
        })
        .collect::<Result<_, DecodeError>>()?;
    Ok(mem::pairings::Pairings { pairing_rule_map })
}

fn reconstruct_slot_pairings(
    block: format::slot_pairings::SlotPairings,
    slots: &mem::slots::Slots,
    periods: &mem::periods::Periods,
) -> Result<mem::slot_pairings::SlotPairings, DecodeError> {
    let slot_pairing_rule_map = block
        .into_inner()
        .into_iter()
        .map(|rule| {
            let raw_id = rule.id;
            let raw_slots = [rule.antecedent.slot_id, rule.consequent.slot_id];
            let raw_excluded_periods: Vec<u64> = rule.excluded_periods.iter().copied().collect();
            let part =
                |part: format::slot_pairings::SlotPairingPart| mem::slot_pairings::SlotRulePart {
                    slot_id: id(part.slot_id),
                    should_have: part.should_have,
                };
            // Honest decode: a rule with both parts on one slot is a hard
            // error here rather than being caught later by layer 3.
            let value = mem::slot_pairings::SlotPairingRule::new(
                part(rule.antecedent),
                part(rule.consequent),
                id_set(rule.excluded_periods),
                rule.soft,
            )
            .map_err(|_| DecodeError::InconsistentSlotPairingRule(raw_id))?;
            // §4.12's reference checks, after the internal seal like the
            // pairings block: antecedent first, then consequent, then the
            // excluded periods (spec field order).
            for raw_slot in raw_slots {
                if slots.find_slot(id::<SlotId>(raw_slot)).is_none() {
                    return Err(DecodeError::DanglingReference {
                        block: BlockName::SlotPairings.as_str(),
                        row: RowKey::Id(raw_id),
                        referenced: IdKind::Slot,
                        id: raw_slot,
                    });
                }
            }
            for &period_raw in &raw_excluded_periods {
                if periods
                    .find_period_position(id::<PeriodId>(period_raw))
                    .is_none()
                {
                    return Err(DecodeError::DanglingReference {
                        block: BlockName::SlotPairings.as_str(),
                        row: RowKey::Id(raw_id),
                        referenced: IdKind::Period,
                        id: period_raw,
                    });
                }
            }
            Ok((id::<SlotPairingRuleId>(raw_id), value))
        })
        .collect::<Result<_, DecodeError>>()?;
    Ok(mem::slot_pairings::SlotPairings {
        slot_pairing_rule_map,
    })
}

fn balancing_options(options: format::balancing::Options) -> mem::balancing::BalancingOptions {
    mem::balancing::BalancingOptions {
        teacher_rotation: options.teacher_rotation,
        slot_rotation: options.slot_rotation,
        avoid_twice_in_a_row: options.avoid_twice_in_a_row,
        year_teacher_rotation: options.year_teacher_rotation,
        period_teacher_rotation: options.period_teacher_rotation,
    }
}

fn reconstruct_balancing(
    block: format::balancing::Balancing,
    subjects: &mem::subjects::Subjects,
) -> Result<mem::balancing::Balancing, DecodeError> {
    let mut per_subject = BTreeMap::new();
    for row in block.subjects.into_inner() {
        if subjects
            .find_subject(id::<SubjectId>(row.subject_id))
            .is_none()
        {
            return Err(DecodeError::DanglingReference {
                block: BlockName::Balancing.as_str(),
                row: RowKey::Id(row.subject_id),
                referenced: IdKind::Subject,
                id: row.subject_id,
            });
        }
        per_subject.insert(
            id::<SubjectId>(row.subject_id),
            balancing_options(row.options),
        );
    }
    Ok(mem::balancing::Balancing {
        global: balancing_options(block.global),
        subjects: per_subject.into(),
    })
}

fn reconstruct_colloscope(
    block: format::colloscope::Colloscope,
    params: &mem::colloscope_params::Parameters,
) -> Result<mem::colloscopes::Colloscope, DecodeError> {
    // The whole key structure is derived (spec §4.15): rows are placed
    // through the sparse colloscope surface, whose canonical form drops
    // empty cells. The trust-boundary checks that the dense skeleton used
    // to embody are re-expressed against the parameters — `find_slot` and
    // `is_interrogation_possible`, which by contract reproduce the dense
    // "cell is `Some`" rule exactly — and against the group-list params.
    let mut colloscope = mem::colloscopes::Colloscope::default();

    // Global week index -> week id, in walk order (S11).
    let week_table: Vec<WeekId> = params
        .walk_weeks()
        .map(|(_period_id, week_id, _week)| week_id)
        .collect();

    for row in block.interrogations.into_inner() {
        let week_id = usize::try_from(row.week)
            .ok()
            .and_then(|week| week_table.get(week).copied());
        let Some(week_id) = week_id else {
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

        if !params.is_interrogation_possible(slot_id, week_id) {
            // The slot's subject does not run on the week's period, or the
            // week is inactive for the slot's pattern: the cell does not
            // exist (mirrors the old dense "slot absent from the period" and
            // "cell is `None`" rejections, folded into one predicate).
            return Err(DecodeError::InvalidInterrogationCell {
                slot_id: row.slot_id,
                week: row.week,
            });
        }

        let assigned_groups: BTreeSet<u32> = row.assigned_groups.into_inner().into_iter().collect();
        if !assigned_groups.is_empty() {
            colloscope.set_interrogation(slot_id, week_id, assigned_groups);
        }
    }

    for row in block.group_lists.into_inner() {
        let group_list_id = id::<GroupListId>(row.group_list_id);
        let known_non_prefilled = params
            .group_lists
            .group_list_map
            .get(&group_list_id)
            .is_some_and(|group_list| !group_list.is_prefilled());
        if !known_non_prefilled {
            // Unknown id, or a prefilled list (whose composition lives in the
            // GroupLists block): only automatic group lists carry placements.
            return Err(DecodeError::InvalidColloscopeGroupList(row.group_list_id));
        }
        let mut placements: BTreeMap<StudentId, u32> = BTreeMap::new();
        for placement in row.students.into_inner() {
            if params
                .students
                .student_map
                .get(&id::<StudentId>(placement.student_id))
                .is_none()
            {
                return Err(DecodeError::DanglingReference {
                    block: BlockName::Colloscope.as_str(),
                    row: RowKey::Id(row.group_list_id),
                    referenced: IdKind::Student,
                    id: placement.student_id,
                });
            }
            placements.insert(id::<StudentId>(placement.student_id), placement.group);
        }
        if !placements.is_empty() {
            colloscope.set_group_list(group_list_id, placements);
        }
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
