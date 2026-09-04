//! Spec-2 encode submodule
//!
//! The name is historical: the pipeline writes every block, each stamped
//! with the spec revision that introduced it, so the file demands the
//! spec level of its content rather than of the writer.
//!
//! This module builds a [Spec2Document] from an [InnerData], in the
//! spec's canonical form (§3): blocks in default
//! state and neutral entries of derived-key-set collections are
//! omitted, blocks appear in canonical order, and unordered collections
//! are sorted.

use crate::EncodeError;
use crate::format;
use crate::json::{Spec2Document, Spec2Entry};

use collomatique_state_colloscopes as mem;
use collomatique_state_colloscopes::InnerData;
use mem::ids::Id;

use std::collections::BTreeSet;

/// Builds the seventeen format blocks of the document — the exact values
/// [encode] writes, which is what makes a check on them faithful
fn build_blocks(inner: &InnerData) -> format::Blocks {
    let params = &inner.params;
    format::Blocks {
        general_planning: Some(build_general_planning(params)),
        subjects: Some(build_subjects(params)),
        teachers: Some(build_teachers(params)),
        students: Some(build_students(params)),
        assignments: Some(build_assignments(params)),
        week_patterns: Some(build_week_patterns(params)),
        slots: Some(build_slots(params)),
        incompatibilities: Some(build_incompatibilities(params)),
        group_lists: Some(build_group_lists(params)),
        group_list_associations: Some(build_group_list_associations(params)),
        pairings: Some(build_pairings(params)),
        slot_pairings: Some(build_slot_pairings(params)),
        settings: Some(build_settings(params)),
        balancing: Some(build_balancing(params)),
        colloscope: Some(build_colloscope(inner)),
        export_config: Some(build_export_config(&inner.export_config)),
        subject_week_patterns: Some(build_subject_week_patterns(params)),
    }
}

/// Runs the writer's id check without writing anything
///
/// It builds the very blocks [encode] would write and checks those, so
/// its verdict is exactly [encode]'s — see [crate::check_encodable].
pub(crate) fn check_encodable(inner: &InnerData) -> Result<(), EncodeError> {
    check_ids(&build_blocks(inner))
}

pub fn encode(inner: &InnerData) -> Result<Spec2Document, EncodeError> {
    // The whole document is built first, then checked, then written out:
    // the id ceiling is a rule about the document as a whole, so it is
    // checked on the format values (where all the ids that will actually
    // be written live) rather than on the in-memory data.
    let blocks = build_blocks(inner);

    check_ids(&blocks)?;

    let mut entries = Vec::new();
    use format::Block;
    push(
        &mut entries,
        blocks.general_planning,
        Block::GeneralPlanning,
    );
    push(&mut entries, blocks.subjects, Block::Subjects);
    push(&mut entries, blocks.teachers, Block::Teachers);
    push(&mut entries, blocks.students, Block::Students);
    push(&mut entries, blocks.assignments, Block::Assignments);
    push(&mut entries, blocks.week_patterns, Block::WeekPatterns);
    push(&mut entries, blocks.slots, Block::Slots);
    push(
        &mut entries,
        blocks.incompatibilities,
        Block::Incompatibilities,
    );
    push(&mut entries, blocks.group_lists, Block::GroupLists);
    push(
        &mut entries,
        blocks.group_list_associations,
        Block::GroupListAssociations,
    );
    push(&mut entries, blocks.pairings, Block::Pairings);
    push(&mut entries, blocks.slot_pairings, Block::SlotPairings);
    push(&mut entries, blocks.settings, Block::Settings);
    push(&mut entries, blocks.balancing, Block::Balancing);
    push(&mut entries, blocks.colloscope, Block::Colloscope);
    push(&mut entries, blocks.export_config, Block::ExportConfig);
    push(
        &mut entries,
        blocks.subject_week_patterns,
        Block::SubjectWeekPatterns,
    );

    Ok(Spec2Document {
        header: super::generate_header(),
        entries,
    })
}

/// Refuses to write a document holding an id above the spec's ceiling
///
/// Nothing in memory forbids such an id: the id issuer hands out numbers
/// without an upper bound, so a long enough editing history — or one
/// operation on a document whose largest id was already the ceiling —
/// produces one. It is the file format, not the model, that caps ids
/// (spec §3), so this is where the document stops being writable.
fn check_ids(blocks: &format::Blocks) -> Result<(), EncodeError> {
    let mut error = None;
    format::id_visit::visit_ids(blocks, &mut |id| {
        if error.is_none() && id > (u64::MAX >> 1) {
            error = Some(EncodeError::IdAboveCeiling { id });
        }
    });
    match error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Appends an entry for the block — unless the block is in its default
/// state, which the canonical form encodes by omission
///
/// The block comes as the `Option` field of [format::Blocks]; the writer
/// fills every field, so `None` cannot happen here.
fn push<B: Default + PartialEq>(
    entries: &mut Vec<Spec2Entry>,
    block: Option<B>,
    wrap: fn(B) -> format::Block,
) {
    let block = block.expect("The writer builds every block");
    if block == B::default() {
        return;
    }
    let content = wrap(block);
    entries.push(Spec2Entry {
        minimum_spec_version: content.name().canonical_spec_version(),
        needed_entry: true,
        content,
    });
}

fn id_set<I: Id>(ids: &BTreeSet<I>) -> format::keyed::UniqueVec<u64> {
    format::keyed::UniqueVec::new(ids.iter().map(|id| id.inner()).collect())
        .expect("Ids from a set are distinct")
}

fn keyed<R: format::keyed::KeyedRow>(rows: Vec<R>) -> format::keyed::KeyedVec<R> {
    format::keyed::KeyedVec::new(rows).expect("Keys from in-memory maps are distinct")
}

fn weekday(day: collomatique_time::Weekday) -> format::scalars::Weekday {
    use format::scalars::Weekday as FormatWeekday;
    match day.into_inner() {
        chrono::Weekday::Mon => FormatWeekday::Monday,
        chrono::Weekday::Tue => FormatWeekday::Tuesday,
        chrono::Weekday::Wed => FormatWeekday::Wednesday,
        chrono::Weekday::Thu => FormatWeekday::Thursday,
        chrono::Weekday::Fri => FormatWeekday::Friday,
        chrono::Weekday::Sat => FormatWeekday::Saturday,
        chrono::Weekday::Sun => FormatWeekday::Sunday,
    }
}

fn time_of_day(time: collomatique_time::WholeMinuteTime) -> format::scalars::TimeOfDay {
    use chrono::Timelike;
    format::scalars::TimeOfDay::new(
        u8::try_from(time.hour()).expect("Hours fit in u8"),
        u8::try_from(time.minute()).expect("Minutes fit in u8"),
    )
    .expect("A whole-minute time is a valid time of day")
}

fn day_time(start: &collomatique_time::SlotStart) -> format::scalars::DayTime {
    format::scalars::DayTime {
        day: weekday(start.weekday),
        time: time_of_day(start.start_time),
    }
}

fn range<T: Copy + Ord + std::fmt::Debug>(
    range: &std::ops::RangeInclusive<T>,
) -> format::scalars::Range<T> {
    format::scalars::Range::new(*range.start(), *range.end())
        .expect("In-memory ranges have min <= max")
}

fn soft_param<T>(param: &mem::soft_param::SoftParam<T>) -> format::scalars::SoftParam<T>
where
    T: Copy,
{
    format::scalars::SoftParam {
        soft: param.soft,
        value: param.value,
    }
}

fn soft_flag(param: &mem::soft_param::SoftParam<()>) -> format::scalars::SoftFlag {
    format::scalars::SoftFlag { soft: param.soft }
}

fn build_general_planning(
    params: &mem::colloscope_params::Parameters,
) -> format::general_planning::GeneralPlanning {
    format::general_planning::GeneralPlanning {
        first_week: params.periods.first_week.as_ref().map(|week_start| {
            format::scalars::WeekStartDate::new(*week_start.monday())
                .expect("A week start is a Monday")
        }),
        periods: params
            .periods
            .period_ids()
            .map(|period_id| format::general_planning::Period {
                id: period_id.inner(),
                weeks: params
                    .weeks
                    .weeks_for_period(period_id)
                    .into_iter()
                    .flatten()
                    .map(|(week_id, week)| format::general_planning::Week {
                        id: week_id.inner(),
                        interrogations: week.interrogations,
                        annotation: week.annotation.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn build_subjects(params: &mem::colloscope_params::Parameters) -> format::subjects::Subjects {
    params
        .subjects
        .ordered_subject_list
        .iter()
        .map(|(subject_id, subject)| format::subjects::Subject {
            id: subject_id.inner(),
            name: subject.parameters.name.clone(),
            interrogation_parameters: subject
                .parameters
                .interrogation_parameters
                .as_ref()
                .map(interrogation_parameters),
            excluded_periods: id_set(&subject.excluded_periods),
        })
        .collect()
}

fn interrogation_parameters(
    params: &mem::subjects::SubjectInterrogationParameters,
) -> format::subjects::InterrogationParameters {
    format::subjects::InterrogationParameters {
        students_per_group: range(&params.students_per_group),
        groups_per_interrogation: range(&params.groups_per_interrogation),
        duration_minutes: format::scalars::DurationMinutes::new(params.duration.get()),
        take_duration_into_account: params.take_duration_into_account,
        periodicity: periodicity(&params.periodicity),
    }
}

fn periodicity(periodicity: &mem::subjects::SubjectPeriodicity) -> format::subjects::Periodicity {
    use format::subjects::Periodicity as FormatPeriodicity;
    use mem::subjects::SubjectPeriodicity;
    match periodicity {
        SubjectPeriodicity::OnceForEveryBlockOfWeeks {
            weeks_per_block,
            minimum_week_separation,
        } => FormatPeriodicity::OnceForEveryBlockOfWeeks(
            format::subjects::OnceForEveryBlockOfWeeks {
                weeks_per_block: *weeks_per_block,
                minimum_week_separation: *minimum_week_separation,
            },
        ),
        SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks,
        } => FormatPeriodicity::ExactlyPeriodic(format::subjects::ExactlyPeriodic {
            periodicity_in_weeks: *periodicity_in_weeks,
        }),
        SubjectPeriodicity::AmountInYear {
            interrogation_count_in_year,
            minimum_week_separation,
        } => FormatPeriodicity::AmountInYear(format::subjects::AmountInYear {
            interrogation_count_in_year: range(interrogation_count_in_year),
            minimum_week_separation: *minimum_week_separation,
        }),
        SubjectPeriodicity::AmountForEveryArbitraryBlock {
            blocks,
            minimum_week_separation,
        } => FormatPeriodicity::AmountForEveryArbitraryBlock(
            format::subjects::AmountForEveryArbitraryBlock {
                blocks: blocks
                    .iter()
                    .map(|block| format::subjects::PeriodicityBlock {
                        delay_in_weeks: block.delay_in_weeks,
                        size_in_weeks: block.size_in_weeks,
                        interrogation_count_in_block: range(&block.interrogation_count_in_block),
                    })
                    .collect(),
                minimum_week_separation: *minimum_week_separation,
            },
        ),
    }
}

fn build_teachers(params: &mem::colloscope_params::Parameters) -> format::teachers::Teachers {
    keyed(
        params
            .teachers
            .teacher_map
            .iter()
            .map(|(teacher_id, teacher)| format::teachers::Teacher {
                id: teacher_id.inner(),
                surname: teacher.desc.surname.clone(),
                firstname: teacher.desc.firstname.clone(),
                tel: teacher.desc.tel.clone(),
                email: teacher.desc.email.clone(),
                subjects: id_set(&teacher.subjects),
            })
            .collect(),
    )
}

fn build_students(params: &mem::colloscope_params::Parameters) -> format::students::Students {
    keyed(
        params
            .students
            .student_map
            .iter()
            .map(|(student_id, student)| format::students::Student {
                id: student_id.inner(),
                surname: student.desc.surname.clone(),
                firstname: student.desc.firstname.clone(),
                tel: student.desc.tel.clone(),
                email: student.desc.email.clone(),
                excluded_periods: id_set(&student.excluded_periods),
            })
            .collect(),
    )
}

fn build_assignments(
    params: &mem::colloscope_params::Parameters,
) -> format::assignments::Assignments {
    let mut rows = Vec::new();
    for ((period_id, subject_id), students) in params.assignments.map.iter() {
        if students.is_empty() {
            // Canonical sparse form never stores an empty row; guard the
            // invariant so a stray empty row is omitted rather than emitted.
            continue;
        }
        rows.push(format::assignments::Assignment {
            period_id: period_id.inner(),
            subject_id: subject_id.inner(),
            students: id_set(students),
        });
    }
    keyed(rows)
}

fn build_week_patterns(
    params: &mem::colloscope_params::Parameters,
) -> format::week_patterns::WeekPatterns {
    // The exclusion set is written as it is held: a `BTreeSet<WeekId>`
    // iterates ascending, which is the canonical order.
    keyed(
        params
            .week_patterns
            .week_pattern_map
            .iter()
            .map(
                |(week_pattern_id, week_pattern)| format::week_patterns::WeekPattern {
                    id: week_pattern_id.inner(),
                    name: week_pattern.name.clone(),
                    excluded_weeks: format::keyed::UniqueVec::new(
                        week_pattern
                            .excluded_weeks
                            .iter()
                            .map(|week_id| week_id.inner())
                            .collect(),
                    )
                    .expect("Week ids from a set are distinct"),
                },
            )
            .collect(),
    )
}

fn build_slots(params: &mem::colloscope_params::Parameters) -> format::slots::Slots {
    let mut rows = Vec::new();
    for subject_id in params.slots.subjects_with_slots() {
        let slots: Vec<_> = params
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
            .map(|(slot_id, slot)| format::slots::Slot {
                id: slot_id.inner(),
                teacher_id: slot.teacher_id.inner(),
                start: day_time(&slot.start_time),
                extra_info: slot.extra_info.clone(),
                week_pattern_id: slot.week_pattern.map(|id| id.inner()),
                cost: slot.cost,
            })
            .collect();
        if slots.is_empty() {
            // Neutral entry of a derived key set: omitted in canonical
            // form
            continue;
        }
        rows.push(format::slots::SubjectSlots {
            subject_id: subject_id.inner(),
            slots,
        });
    }
    keyed(rows)
}

fn build_incompatibilities(
    params: &mem::colloscope_params::Parameters,
) -> format::incompatibilities::Incompatibilities {
    keyed(
        params
            .incompats
            .incompat_map
            .iter()
            .map(
                |(incompat_id, incompat)| format::incompatibilities::Incompatibility {
                    id: incompat_id.inner(),
                    subject_id: incompat.subject_id.inner(),
                    name: incompat.name.clone(),
                    slots: incompat
                        .slots
                        .iter()
                        .map(|slot| format::incompatibilities::IncompatibilitySlot {
                            day: weekday(slot.start().weekday),
                            time: time_of_day(slot.start().start_time),
                            duration_minutes: format::scalars::DurationMinutes::new(
                                slot.duration().get(),
                            ),
                        })
                        .collect(),
                    minimum_free_slots: incompat.minimum_free_slots,
                    week_pattern_id: incompat.week_pattern_id.map(|id| id.inner()),
                },
            )
            .collect(),
    )
}

fn build_group_lists(
    params: &mem::colloscope_params::Parameters,
) -> format::group_lists::GroupLists {
    keyed(
        params
            .group_lists
            .group_list_map
            .iter()
            .map(|(group_list_id, group_list)| {
                let filling = match group_list.filling() {
                    mem::group_lists::GroupListFilling::Prefilled { groups } => {
                        format::group_lists::Filling::Prefilled(format::group_lists::Prefilled {
                            groups: groups
                                .iter()
                                .map(|group| format::group_lists::Group {
                                    students: id_set(&group.students),
                                })
                                .collect(),
                        })
                    }
                    mem::group_lists::GroupListFilling::Automatic { excluded_students } => {
                        format::group_lists::Filling::Automatic(format::group_lists::Automatic {
                            excluded_students: id_set(excluded_students),
                        })
                    }
                };
                format::group_lists::GroupList {
                    id: group_list_id.inner(),
                    name: group_list.params().name.clone(),
                    students_per_group: range(&group_list.params().students_per_group),
                    group_names: group_list.params().group_names.clone(),
                    filling,
                }
            })
            .collect(),
    )
}

fn build_group_list_associations(
    params: &mem::colloscope_params::Parameters,
) -> format::group_list_associations::GroupListAssociations {
    let mut rows = Vec::new();
    for ((period_id, subject_id), group_list_id) in params.group_lists.subjects_associations.iter()
    {
        rows.push(format::group_list_associations::GroupListAssociation {
            period_id: period_id.inner(),
            subject_id: subject_id.inner(),
            group_list_id: group_list_id.inner(),
        });
    }
    keyed(rows)
}

fn build_subject_week_patterns(
    params: &mem::colloscope_params::Parameters,
) -> format::subject_week_patterns::SubjectWeekPatterns {
    // Sparse: a subject without a pattern has no row. Sorted by subject id,
    // like every unordered collection in canonical form — the subject list
    // is in user order, which is not it.
    let mut rows: Vec<_> = params
        .subjects
        .ordered_subject_list
        .iter()
        .filter_map(|(subject_id, subject)| {
            subject.week_pattern.map(|week_pattern_id| {
                format::subject_week_patterns::SubjectWeekPattern {
                    subject_id: subject_id.inner(),
                    week_pattern_id: week_pattern_id.inner(),
                }
            })
        })
        .collect();
    rows.sort_by_key(|row| row.subject_id);
    keyed(rows)
}

fn build_pairings(params: &mem::colloscope_params::Parameters) -> format::pairings::Pairings {
    keyed(
        params
            .pairings
            .pairing_rule_map
            .iter()
            .map(|(rule_id, rule)| {
                let part = |part: &mem::pairings::RulePart| format::pairings::PairingPart {
                    subject_id: part.subject_id.inner(),
                    should_have: part.should_have,
                };
                format::pairings::Pairing {
                    id: rule_id.inner(),
                    antecedent: part(rule.antecedent()),
                    consequent: part(rule.consequent()),
                    excluded_periods: id_set(rule.excluded_periods()),
                    soft: rule.soft(),
                }
            })
            .collect(),
    )
}

fn build_slot_pairings(
    params: &mem::colloscope_params::Parameters,
) -> format::slot_pairings::SlotPairings {
    keyed(
        params
            .slot_pairings
            .slot_pairing_rule_map
            .iter()
            .map(|(rule_id, rule)| {
                let part = |part: &mem::slot_pairings::SlotRulePart| {
                    format::slot_pairings::SlotPairingPart {
                        slot_id: part.slot_id.inner(),
                        should_have: part.should_have,
                    }
                };
                format::slot_pairings::SlotPairing {
                    id: rule_id.inner(),
                    antecedent: part(rule.antecedent()),
                    consequent: part(rule.consequent()),
                    excluded_periods: id_set(rule.excluded_periods()),
                    soft: rule.soft(),
                }
            })
            .collect(),
    )
}

fn limits(limits: &mem::settings::Limits) -> format::settings::Limits {
    format::settings::Limits {
        interrogations_per_week_min: limits.interrogations_per_week_min.as_ref().map(soft_param),
        interrogations_per_week_max: limits.interrogations_per_week_max.as_ref().map(soft_param),
        max_interrogations_per_day: limits.max_interrogations_per_day.as_ref().map(soft_param),
    }
}

fn build_settings(params: &mem::colloscope_params::Parameters) -> format::settings::Settings {
    format::settings::Settings {
        global: limits(&params.settings.global),
        // Free key set: a row existing is information, so rows are
        // written exactly as they exist
        students: keyed(
            params
                .settings
                .students
                .iter()
                .map(
                    |(student_id, student_limits)| format::settings::StudentOverride {
                        student_id: student_id.inner(),
                        limits: limits(student_limits),
                    },
                )
                .collect(),
        ),
    }
}

fn balancing_options(options: &mem::balancing::BalancingOptions) -> format::balancing::Options {
    format::balancing::Options {
        teacher_rotation: options.teacher_rotation.as_ref().map(soft_flag),
        slot_rotation: options.slot_rotation.as_ref().map(soft_flag),
        avoid_twice_in_a_row: options.avoid_twice_in_a_row.as_ref().map(soft_flag),
        year_teacher_rotation: options.year_teacher_rotation,
        period_teacher_rotation: options.period_teacher_rotation,
    }
}

fn build_balancing(params: &mem::colloscope_params::Parameters) -> format::balancing::Balancing {
    format::balancing::Balancing {
        global: balancing_options(&params.balancing.global),
        subjects: keyed(
            params
                .balancing
                .subjects
                .iter()
                .map(|(subject_id, options)| format::balancing::SubjectOverride {
                    subject_id: subject_id.inner(),
                    options: balancing_options(options),
                })
                .collect(),
        ),
    }
}

fn build_colloscope(inner: &mem::InnerData) -> format::colloscope::Colloscope {
    // The colloscope key structure is derived, so only non-neutral
    // cells are written: interrogations with assigned groups, group
    // lists with placed students. The sparse surface yields exactly the
    // non-empty rows, already ascending on the `(slot_id, week_id)` key
    // it is stored under — which is the canonical order of the block.
    let interrogation_rows: Vec<_> = inner
        .colloscope
        .iter()
        .map(
            |((slot_id, week_id), assigned_groups)| format::colloscope::Interrogation {
                slot_id: slot_id.inner(),
                week_id: week_id.inner(),
                assigned_groups: format::keyed::UniqueVec::new(
                    assigned_groups.iter().copied().collect(),
                )
                .expect("Group numbers from a set are distinct"),
            },
        )
        .collect();

    let group_list_rows = inner
        .colloscope
        .group_lists_iter()
        .map(
            |(group_list_id, groups_for_students)| format::colloscope::FilledGroupList {
                group_list_id: group_list_id.inner(),
                students: keyed(
                    groups_for_students
                        .iter()
                        .map(|(student_id, group)| format::colloscope::StudentPlacement {
                            student_id: student_id.inner(),
                            group: *group,
                        })
                        .collect(),
                ),
            },
        )
        .collect();

    format::colloscope::Colloscope {
        interrogations: keyed(interrogation_rows),
        group_lists: keyed(group_list_rows),
    }
}

fn color(color: &mem::export_config::Color) -> format::scalars::Color {
    format::scalars::Color {
        red: color.red,
        green: color.green,
        blue: color.blue,
    }
}

fn orientation(
    orientation: &mem::export_config::PageOrientation,
) -> format::export_config::Orientation {
    match orientation {
        mem::export_config::PageOrientation::Portrait => {
            format::export_config::Orientation::Portrait
        }
        mem::export_config::PageOrientation::Landscape => {
            format::export_config::Orientation::Landscape
        }
    }
}

fn per_student_groups_config(
    config: &mem::export_config::PerStudentGroupsConfig,
) -> format::export_config::PerStudentGroupsConfig {
    format::export_config::PerStudentGroupsConfig {
        sheet_name: config.sheet_name.clone(),
        orientation: config.orientation.as_ref().map(orientation),
        show_emails: config.show_emails,
        show_tel: config.show_tel,
    }
}

fn build_export_config(
    config: &mem::export_config::ExportConfig,
) -> format::export_config::ExportConfig {
    format::export_config::ExportConfig {
        global: format::export_config::GlobalConfig {
            background_color: color(&config.global.background_color),
            stripes_color_enabled: config.global.stripes_color_enabled,
            stripes_color: color(&config.global.stripes_color),
        },
        colloscope_enabled: config.colloscope_enabled,
        all_groups_enabled: config.all_groups_enabled,
        automatic_groups_enabled: config.automatic_groups_enabled,
        prefilled_groups_enabled: config.prefilled_groups_enabled,
        per_group_list_enabled: config.per_group_list_enabled,
        colloscope_config: format::export_config::ColloscopeConfig {
            sheet_name: config.colloscope_config.sheet_name.clone(),
            extra_info_column_enabled: config.colloscope_config.extra_info_column_enabled,
            extra_info_column_name: config.colloscope_config.extra_info_column_name.clone(),
            teacher_email_enabled: config.colloscope_config.teacher_email_enabled,
            teacher_email: config.colloscope_config.teacher_email.clone(),
            teacher_tel_enabled: config.colloscope_config.teacher_tel_enabled,
            teacher_tel: config.colloscope_config.teacher_tel.clone(),
            orientation: orientation(&config.colloscope_config.orientation),
            display_week_dates: config.colloscope_config.display_week_dates,
            display_annotations: config.colloscope_config.display_annotations,
            no_interrogation_color: color(&config.colloscope_config.no_interrogation_color),
            annotation_color_enabled: config.colloscope_config.annotation_color_enabled,
            annotation_color: color(&config.colloscope_config.annotation_color),
            // BTreeMap iteration is ascending by name, which is the
            // canonical order for extra_colors
            extra_colors: keyed(
                config
                    .colloscope_config
                    .extra_colors
                    .iter()
                    .map(|(name, extra_color)| format::export_config::ExtraColor {
                        name: name.clone(),
                        color: color(extra_color),
                    })
                    .collect(),
            ),
        },
        all_groups_config: per_student_groups_config(&config.all_groups_config),
        automatic_groups_config: per_student_groups_config(&config.automatic_groups_config),
        prefilled_groups_config: per_student_groups_config(&config.prefilled_groups_config),
        per_group_list_config: format::export_config::PerGroupListConfig {
            orientation: orientation(&config.per_group_list_config.orientation),
            show_emails: config.per_group_list_config.show_emails,
            show_tel: config.per_group_list_config.show_tel,
            center_vertically: config.per_group_list_config.center_vertically,
        },
    }
}
