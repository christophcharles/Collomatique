//! SQLite state persistence for Collomatique
//!
//! This crate provides SQLite-based storage for colloscope data.
//! It supports bidirectional conversion between `InnerData` and SQLite.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use collomatique_state_colloscopes::{
    assignments, colloscope_params, colloscopes, group_lists, ids, incompats, periods, settings,
    slots, students, subjects, teachers, week_patterns, InnerData, PersonWithContact,
};
use collomatique_time::{
    NonZeroMinutes, SlotStart, SlotWithDuration, WeekStart, Weekday, WholeMinuteTime,
};
use ids::{
    GroupListId, Id, IncompatId, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekPatternId,
};
use sqlx::SqlitePool;
use thiserror::Error;

mod schema;

pub use schema::SCHEMA_SQL;

/// Errors that can occur during SQLite operations
#[derive(Debug, Error)]
pub enum Error {
    #[error("SQLite error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Invalid data in database: {0}")]
    InvalidData(String),

    #[error("Missing required data: {0}")]
    MissingData(String),

    #[error("Invalid periodicity type: {0}")]
    InvalidPeriodicityType(String),

    #[error("Invalid filling type: {0}")]
    InvalidFillingType(String),

    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),

    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),

    #[error("Invalid weekday: {0}")]
    InvalidWeekday(i64),
}

/// Validation errors for database invariants
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Teacher {teacher_id} does not teach subject {subject_id} but has slot {slot_id}")]
    TeacherSubjectMismatch {
        teacher_id: i64,
        subject_id: i64,
        slot_id: i64,
    },

    #[error("Student {student_id} is assigned to subject {subject_id} in period {period_id} but is excluded from that period")]
    StudentExcludedFromPeriod {
        student_id: i64,
        subject_id: i64,
        period_id: i64,
    },

    #[error(
        "Subject {subject_id} is assigned in period {period_id} but is excluded from that period"
    )]
    SubjectExcludedFromPeriod { subject_id: i64, period_id: i64 },

    #[error("Colloscope slot entry for period {period_id}, slot {slot_id}, week {week_index} is invalid")]
    InvalidColloscopeSlot {
        period_id: i64,
        slot_id: i64,
        week_index: i64,
    },

    #[error("Prefilled group data exists for automatic group list {group_list_id}")]
    PrefilledDataForAutomaticGroupList { group_list_id: i64 },

    #[error("Automatic group exclusion exists for prefilled group list {group_list_id}")]
    AutomaticExclusionForPrefilledGroupList { group_list_id: i64 },

    #[error("SQLite error during validation: {0}")]
    Sqlx(#[from] sqlx::Error),
}

/// Create the database schema (all tables, triggers, indexes)
pub async fn create_schema(pool: &SqlitePool) -> Result<(), Error> {
    sqlx::raw_sql(SCHEMA_SQL).execute(pool).await?;
    Ok(())
}

/// Populate an SQLite database from InnerData
pub async fn inner_data_to_sqlite(pool: &SqlitePool, data: &InnerData) -> Result<(), Error> {
    let mut tx = pool.begin().await?;

    // Insert metadata
    insert_metadata(&mut tx, &data.params).await?;

    // Insert periods
    insert_periods(&mut tx, &data.params.periods).await?;

    // Insert subjects (must come before teachers due to foreign key constraints)
    insert_subjects(&mut tx, &data.params.subjects).await?;

    // Insert students
    insert_students(&mut tx, &data.params.students).await?;

    // Insert teachers
    insert_teachers(&mut tx, &data.params.teachers).await?;

    // Insert week patterns
    insert_week_patterns(&mut tx, &data.params.week_patterns, &data.params.periods).await?;

    // Insert slots
    insert_slots(&mut tx, &data.params.slots).await?;

    // Insert incompatibilities
    insert_incompats(&mut tx, &data.params.incompats).await?;

    // Insert group lists
    insert_group_lists(&mut tx, &data.params.group_lists).await?;

    // Insert assignments
    insert_assignments(&mut tx, &data.params.assignments).await?;

    // Insert settings
    insert_settings(&mut tx, &data.params.settings).await?;

    // Insert colloscope data
    insert_colloscope(&mut tx, &data.colloscope, &data.params).await?;

    tx.commit().await?;
    Ok(())
}

/// Reconstruct InnerData from an SQLite database
pub async fn sqlite_to_inner_data(pool: &SqlitePool) -> Result<InnerData, Error> {
    let params = read_parameters(pool).await?;
    let colloscope = read_colloscope(pool, &params).await?;

    Ok(InnerData { params, colloscope })
}

/// Export the database to a file
///
/// Uses SQLite's `VACUUM INTO` to create a clean, compacted copy of the database.
/// Note: `VACUUM INTO` doesn't support parameter bindings, so the path is quoted manually.
pub async fn export_to_file(pool: &SqlitePool, path: &std::path::Path) -> Result<(), Error> {
    // VACUUM INTO requires a string literal, not a bound parameter.
    // We escape single quotes by doubling them (SQL standard escaping).
    let path_str = path.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{path_str}'"))
        .execute(pool)
        .await?;
    Ok(())
}

/// Validate all invariants on the SQLite database
pub async fn validate_database(pool: &SqlitePool) -> Result<(), ValidationError> {
    // 1. Teacher-subject consistency: slots.teacher_id must teach slots.subject_id
    validate_teacher_subject_consistency(pool).await?;

    // 2. Period exclusion consistency for assignments
    validate_assignment_exclusions(pool).await?;

    // 3. Group list filling type consistency
    validate_group_list_filling_consistency(pool).await?;

    Ok(())
}

// ============================================================================
// Helper functions for inserting data
// ============================================================================

async fn insert_metadata(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    params: &colloscope_params::Parameters,
) -> Result<(), Error> {
    let first_week = params
        .periods
        .first_week
        .as_ref()
        .map(|w| w.monday().format("%Y-%m-%d").to_string());

    sqlx::query("INSERT INTO metadata (id, first_week, main_script) VALUES (1, ?, ?)")
        .bind(&first_week)
        .bind(&params.main_script)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

async fn insert_periods(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    periods: &periods::Periods,
) -> Result<(), Error> {
    for (position, (period_id, weeks)) in periods.ordered_period_list.iter().enumerate() {
        let id = period_id.inner() as i64;

        sqlx::query("INSERT INTO periods (id, position) VALUES (?, ?)")
            .bind(id)
            .bind(position as i64)
            .execute(&mut **tx)
            .await?;

        for (week_index, week_desc) in weeks.iter().enumerate() {
            sqlx::query(
                "INSERT INTO period_weeks (period_id, week_index, has_interrogations, annotation) VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind(week_index as i64)
            .bind(week_desc.interrogations as i64)
            .bind(week_desc.annotation.as_ref().map(|s| s.as_str()).unwrap_or(""))
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

async fn insert_subjects(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    subjects: &subjects::Subjects,
) -> Result<(), Error> {
    for (position, (subject_id, subject)) in subjects.ordered_subject_list.iter().enumerate() {
        let id = subject_id.inner() as i64;

        sqlx::query("INSERT INTO subjects (id, position, name) VALUES (?, ?, ?)")
            .bind(id)
            .bind(position as i64)
            .bind(&subject.parameters.name)
            .execute(&mut **tx)
            .await?;

        // Insert excluded periods
        for period_id in &subject.excluded_periods {
            sqlx::query(
                "INSERT INTO subject_excluded_periods (subject_id, period_id) VALUES (?, ?)",
            )
            .bind(id)
            .bind(period_id.inner() as i64)
            .execute(&mut **tx)
            .await?;
        }

        // Insert interrogation parameters if present
        if let Some(interrogation_params) = &subject.parameters.interrogation_parameters {
            // Prepare periodicity column values based on periodicity variant
            let (ep, ofeb_wpb, ofeb_mws, aiy_min, aiy_max, aiy_mws, afab_mws): (
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<i64>,
            ) = match &interrogation_params.periodicity {
                subjects::SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks,
                } => (
                    Some(periodicity_in_weeks.get() as i64),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
                subjects::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                    weeks_per_block,
                    minimum_week_separation,
                } => (
                    None,
                    Some(weeks_per_block.get() as i64),
                    Some(minimum_week_separation.get() as i64),
                    None,
                    None,
                    None,
                    None,
                ),
                subjects::SubjectPeriodicity::AmountInYear {
                    interrogation_count_in_year,
                    minimum_week_separation,
                } => (
                    None,
                    None,
                    None,
                    Some(*interrogation_count_in_year.start() as i64),
                    Some(*interrogation_count_in_year.end() as i64),
                    Some(*minimum_week_separation as i64),
                    None,
                ),
                subjects::SubjectPeriodicity::AmountForEveryArbitraryBlock {
                    minimum_week_separation,
                    ..
                } => (
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(*minimum_week_separation as i64),
                ),
            };

            sqlx::query(
                "INSERT INTO subject_interrogation_params (
                    subject_id, students_per_group_min, students_per_group_max,
                    groups_per_interrogation_min, groups_per_interrogation_max,
                    duration_minutes, take_duration_into_account,
                    ep_periodicity_in_weeks,
                    ofeb_weeks_per_block, ofeb_minimum_week_separation,
                    aiy_count_min, aiy_count_max, aiy_minimum_week_separation,
                    afab_minimum_week_separation
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(interrogation_params.students_per_group.start().get() as i64)
            .bind(interrogation_params.students_per_group.end().get() as i64)
            .bind(interrogation_params.groups_per_interrogation.start().get() as i64)
            .bind(interrogation_params.groups_per_interrogation.end().get() as i64)
            .bind(interrogation_params.duration.get().get() as i64)
            .bind(interrogation_params.take_duration_into_account as i64)
            .bind(ep)
            .bind(ofeb_wpb)
            .bind(ofeb_mws)
            .bind(aiy_min)
            .bind(aiy_max)
            .bind(aiy_mws)
            .bind(afab_mws)
            .execute(&mut **tx)
            .await?;

            // Insert week blocks for AmountForEveryArbitraryBlock periodicity
            if let subjects::SubjectPeriodicity::AmountForEveryArbitraryBlock { blocks, .. } =
                &interrogation_params.periodicity
            {
                for (block_index, block) in blocks.iter().enumerate() {
                    sqlx::query(
                        "INSERT INTO periodicity_week_blocks
                         (subject_id, block_index, delay_in_weeks, size_in_weeks,
                          interrogation_count_min, interrogation_count_max)
                         VALUES (?, ?, ?, ?, ?, ?)",
                    )
                    .bind(id)
                    .bind(block_index as i64)
                    .bind(block.delay_in_weeks as i64)
                    .bind(block.size_in_weeks.get() as i64)
                    .bind(*block.interrogation_count_in_block.start() as i64)
                    .bind(*block.interrogation_count_in_block.end() as i64)
                    .execute(&mut **tx)
                    .await?;
                }
            }
        }
    }

    Ok(())
}

async fn insert_students(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    students: &students::Students,
) -> Result<(), Error> {
    for (student_id, student) in &students.student_map {
        let id = student_id.inner() as i64;

        sqlx::query(
            "INSERT INTO students (id, surname, firstname, tel, email) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&student.desc.surname)
        .bind(&student.desc.firstname)
        .bind(student.desc.tel.as_ref().map(|s| s.as_str()).unwrap_or(""))
        .bind(
            student
                .desc
                .email
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or(""),
        )
        .execute(&mut **tx)
        .await?;

        for period_id in &student.excluded_periods {
            sqlx::query(
                "INSERT INTO student_excluded_periods (student_id, period_id) VALUES (?, ?)",
            )
            .bind(id)
            .bind(period_id.inner() as i64)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

async fn insert_teachers(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    teachers: &teachers::Teachers,
) -> Result<(), Error> {
    for (teacher_id, teacher) in &teachers.teacher_map {
        let id = teacher_id.inner() as i64;

        sqlx::query(
            "INSERT INTO teachers (id, surname, firstname, tel, email) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&teacher.desc.surname)
        .bind(&teacher.desc.firstname)
        .bind(teacher.desc.tel.as_ref().map(|s| s.as_str()).unwrap_or(""))
        .bind(
            teacher
                .desc
                .email
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or(""),
        )
        .execute(&mut **tx)
        .await?;

        for subject_id in &teacher.subjects {
            sqlx::query("INSERT INTO teacher_subjects (teacher_id, subject_id) VALUES (?, ?)")
                .bind(id)
                .bind(subject_id.inner() as i64)
                .execute(&mut **tx)
                .await?;
        }
    }

    Ok(())
}

async fn insert_week_patterns(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    week_patterns: &week_patterns::WeekPatterns,
    periods: &periods::Periods,
) -> Result<(), Error> {
    for (week_pattern_id, week_pattern) in &week_patterns.week_pattern_map {
        let id = week_pattern_id.inner() as i64;

        sqlx::query("INSERT INTO week_patterns (id, name) VALUES (?, ?)")
            .bind(id)
            .bind(&week_pattern.name)
            .execute(&mut **tx)
            .await?;

        // Convert global index to period-relative and insert only disabled weeks
        let mut global_index = 0usize;
        for (period_id, period_weeks) in &periods.ordered_period_list {
            for week_index in 0..period_weeks.len() {
                let is_active = week_pattern
                    .weeks
                    .get(global_index)
                    .copied()
                    .unwrap_or(true);
                if !is_active {
                    sqlx::query(
                        "INSERT INTO week_pattern_disabled_weeks
                         (week_pattern_id, period_id, week_index) VALUES (?, ?, ?)",
                    )
                    .bind(id)
                    .bind(period_id.inner() as i64)
                    .bind(week_index as i64)
                    .execute(&mut **tx)
                    .await?;
                }
                global_index += 1;
            }
        }
    }

    Ok(())
}

async fn insert_slots(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    all_slots: &slots::Slots,
) -> Result<(), Error> {
    for (subject_id, subject_slots) in &all_slots.subject_map {
        for (position, (slot_id, slot)) in subject_slots.ordered_slots.iter().enumerate() {
            let id = slot_id.inner() as i64;
            let weekday = slot.start_time.weekday.num_days_from_monday() as i64;
            let start_time = slot.start_time.start_time.format("%H:%M").to_string();

            sqlx::query(
                "INSERT INTO slots (id, subject_id, position, teacher_id, day, start_time, extra_info, week_pattern_id, cost)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(subject_id.inner() as i64)
            .bind(position as i64)
            .bind(slot.teacher_id.inner() as i64)
            .bind(weekday)
            .bind(&start_time)
            .bind(&slot.extra_info)
            .bind(slot.week_pattern.map(|wp| wp.inner() as i64))
            .bind(slot.cost as i64)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

async fn insert_incompats(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    incompats: &incompats::Incompats,
) -> Result<(), Error> {
    for (incompat_id, incompat) in &incompats.incompat_map {
        let id = incompat_id.inner() as i64;

        sqlx::query(
            "INSERT INTO incompats (id, subject_id, name, minimum_free_slots, week_pattern_id)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(incompat.subject_id.inner() as i64)
        .bind(&incompat.name)
        .bind(incompat.minimum_free_slots.get() as i64)
        .bind(incompat.week_pattern_id.map(|wp| wp.inner() as i64))
        .execute(&mut **tx)
        .await?;

        for (slot_index, slot) in incompat.slots.iter().enumerate() {
            let weekday = slot.start().weekday.num_days_from_monday() as i64;
            let start_time = slot.start().start_time.format("%H:%M").to_string();

            sqlx::query(
                "INSERT INTO incompat_slots (incompat_id, slot_index, day, start_time, duration_minutes)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(slot_index as i64)
            .bind(weekday)
            .bind(&start_time)
            .bind(slot.duration().get().get() as i64)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

async fn insert_group_lists(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    group_lists: &group_lists::GroupLists,
) -> Result<(), Error> {
    for (group_list_id, group_list) in &group_lists.group_list_map {
        let id = group_list_id.inner() as i64;
        let filling_type = match &group_list.filling {
            group_lists::GroupListFilling::Prefilled { .. } => "prefilled",
            group_lists::GroupListFilling::Automatic { .. } => "automatic",
        };

        sqlx::query(
            "INSERT INTO group_lists (id, name, students_per_group_min, students_per_group_max, filling_type)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&group_list.params.name)
        .bind(group_list.params.students_per_group.start().get() as i64)
        .bind(group_list.params.students_per_group.end().get() as i64)
        .bind(filling_type)
        .execute(&mut **tx)
        .await?;

        // Insert group names
        for (group_index, group_name) in group_list.params.group_names.iter().enumerate() {
            sqlx::query(
                "INSERT INTO group_list_group_names (group_list_id, group_index, name) VALUES (?, ?, ?)",
            )
            .bind(id)
            .bind(group_index as i64)
            .bind(group_name.as_ref().map(|s| s.as_str()).unwrap_or(""))
            .execute(&mut **tx)
            .await?;
        }

        // Insert filling-specific data
        match &group_list.filling {
            group_lists::GroupListFilling::Prefilled { groups } => {
                for (group_index, group) in groups.iter().enumerate() {
                    for student_id in &group.students {
                        sqlx::query(
                            "INSERT INTO prefilled_group_students (group_list_id, group_index, student_id)
                             VALUES (?, ?, ?)",
                        )
                        .bind(id)
                        .bind(group_index as i64)
                        .bind(student_id.inner() as i64)
                        .execute(&mut **tx)
                        .await?;
                    }
                }
            }
            group_lists::GroupListFilling::Automatic { excluded_students } => {
                for student_id in excluded_students {
                    sqlx::query(
                        "INSERT INTO automatic_group_excluded_students (group_list_id, student_id)
                         VALUES (?, ?)",
                    )
                    .bind(id)
                    .bind(student_id.inner() as i64)
                    .execute(&mut **tx)
                    .await?;
                }
            }
        }
    }

    // Insert subject associations
    for (period_id, subject_map) in &group_lists.subjects_associations {
        for (subject_id, group_list_id) in subject_map {
            sqlx::query(
                "INSERT INTO group_list_subject_associations (period_id, subject_id, group_list_id)
                 VALUES (?, ?, ?)",
            )
            .bind(period_id.inner() as i64)
            .bind(subject_id.inner() as i64)
            .bind(group_list_id.inner() as i64)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

async fn insert_assignments(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    assignments: &assignments::Assignments,
) -> Result<(), Error> {
    for (period_id, period_assignments) in &assignments.period_map {
        for (subject_id, student_ids) in &period_assignments.subject_map {
            for student_id in student_ids {
                sqlx::query(
                    "INSERT INTO assignments (period_id, subject_id, student_id) VALUES (?, ?, ?)",
                )
                .bind(period_id.inner() as i64)
                .bind(subject_id.inner() as i64)
                .bind(student_id.inner() as i64)
                .execute(&mut **tx)
                .await?;
            }
        }
    }

    Ok(())
}

async fn insert_settings(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    settings_data: &settings::Settings,
) -> Result<(), Error> {
    // Insert global settings
    sqlx::query(
        "INSERT INTO settings_global (id,
            interrogations_per_week_min_value, interrogations_per_week_min_soft,
            interrogations_per_week_max_value, interrogations_per_week_max_soft,
            max_interrogations_per_day_value, max_interrogations_per_day_soft)
         VALUES (1, ?, ?, ?, ?, ?, ?)",
    )
    .bind(
        settings_data
            .global
            .interrogations_per_week_min
            .as_ref()
            .map(|p| p.value as i64),
    )
    .bind(
        settings_data
            .global
            .interrogations_per_week_min
            .as_ref()
            .map(|p| p.soft as i64),
    )
    .bind(
        settings_data
            .global
            .interrogations_per_week_max
            .as_ref()
            .map(|p| p.value as i64),
    )
    .bind(
        settings_data
            .global
            .interrogations_per_week_max
            .as_ref()
            .map(|p| p.soft as i64),
    )
    .bind(
        settings_data
            .global
            .max_interrogations_per_day
            .as_ref()
            .map(|p| p.value.get() as i64),
    )
    .bind(
        settings_data
            .global
            .max_interrogations_per_day
            .as_ref()
            .map(|p| p.soft as i64),
    )
    .execute(&mut **tx)
    .await?;

    // Insert per-student settings
    for (student_id, limits) in &settings_data.students {
        sqlx::query(
            "INSERT INTO settings_students (student_id,
                interrogations_per_week_min_value, interrogations_per_week_min_soft,
                interrogations_per_week_max_value, interrogations_per_week_max_soft,
                max_interrogations_per_day_value, max_interrogations_per_day_soft)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(student_id.inner() as i64)
        .bind(
            limits
                .interrogations_per_week_min
                .as_ref()
                .map(|p| p.value as i64),
        )
        .bind(
            limits
                .interrogations_per_week_min
                .as_ref()
                .map(|p| p.soft as i64),
        )
        .bind(
            limits
                .interrogations_per_week_max
                .as_ref()
                .map(|p| p.value as i64),
        )
        .bind(
            limits
                .interrogations_per_week_max
                .as_ref()
                .map(|p| p.soft as i64),
        )
        .bind(
            limits
                .max_interrogations_per_day
                .as_ref()
                .map(|p| p.value.get() as i64),
        )
        .bind(
            limits
                .max_interrogations_per_day
                .as_ref()
                .map(|p| p.soft as i64),
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

async fn insert_colloscope(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    colloscope: &colloscopes::Colloscope,
    _params: &colloscope_params::Parameters,
) -> Result<(), Error> {
    // Insert colloscope slots
    for (period_id, period) in &colloscope.period_map {
        for (slot_id, slot) in &period.slot_map {
            for (week_index, interrogation_opt) in slot.interrogations.iter().enumerate() {
                let has_interrogation = interrogation_opt.is_some();

                sqlx::query(
                    "INSERT INTO colloscope_slots (period_id, slot_id, week_index, has_interrogation)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(period_id.inner() as i64)
                .bind(slot_id.inner() as i64)
                .bind(week_index as i64)
                .bind(has_interrogation as i64)
                .execute(&mut **tx)
                .await?;

                if let Some(interrogation) = interrogation_opt {
                    for group_number in &interrogation.assigned_groups {
                        sqlx::query(
                            "INSERT INTO colloscope_interrogation_groups
                             (period_id, slot_id, week_index, group_number) VALUES (?, ?, ?, ?)",
                        )
                        .bind(period_id.inner() as i64)
                        .bind(slot_id.inner() as i64)
                        .bind(week_index as i64)
                        .bind(*group_number as i64)
                        .execute(&mut **tx)
                        .await?;
                    }
                }
            }
        }
    }

    // Insert colloscope group list student assignments
    for (group_list_id, group_list) in &colloscope.group_lists {
        for (student_id, group_number) in &group_list.groups_for_students {
            sqlx::query(
                "INSERT INTO colloscope_group_list_students (group_list_id, student_id, group_number)
                 VALUES (?, ?, ?)",
            )
            .bind(group_list_id.inner() as i64)
            .bind(student_id.inner() as i64)
            .bind(*group_number as i64)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

// ============================================================================
// Helper functions for reading data
// ============================================================================

async fn read_parameters(pool: &SqlitePool) -> Result<colloscope_params::Parameters, Error> {
    let periods = read_periods(pool).await?;
    let subjects = read_subjects(pool).await?;
    let students = read_students(pool).await?;
    let teachers = read_teachers(pool).await?;
    let week_patterns = read_week_patterns(pool, &periods).await?;
    let slots = read_slots(pool).await?;
    let incompats = read_incompats(pool).await?;
    let group_lists = read_group_lists(pool).await?;
    let assignments = read_assignments(pool).await?;
    let settings = read_settings(pool).await?;
    let main_script = read_main_script(pool).await?;

    Ok(colloscope_params::Parameters {
        periods,
        subjects,
        students,
        teachers,
        assignments,
        week_patterns,
        slots,
        incompats,
        group_lists,
        settings,
        main_script,
    })
}

async fn read_periods(pool: &SqlitePool) -> Result<periods::Periods, Error> {
    // Read first_week from metadata
    let first_week: Option<String> =
        sqlx::query_scalar("SELECT first_week FROM metadata WHERE id = 1")
            .fetch_optional(pool)
            .await?
            .flatten();

    let first_week =
        match first_week {
            Some(date_str) => {
                let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .map_err(|_| Error::InvalidDateFormat(date_str.clone()))?;
                Some(WeekStart::new(date).ok_or_else(|| {
                    Error::InvalidDateFormat(format!("{} is not a Monday", date_str))
                })?)
            }
            None => None,
        };

    // Read periods ordered by position
    let period_rows: Vec<(i64, i64)> =
        sqlx::query_as("SELECT id, position FROM periods ORDER BY position")
            .fetch_all(pool)
            .await?;

    let mut ordered_period_list = Vec::new();

    for (period_id, _position) in period_rows {
        let week_rows: Vec<(i64, i64, String)> = sqlx::query_as(
            "SELECT week_index, has_interrogations, annotation FROM period_weeks
             WHERE period_id = ? ORDER BY week_index",
        )
        .bind(period_id)
        .fetch_all(pool)
        .await?;

        let weeks: Vec<periods::WeekDesc> = week_rows
            .into_iter()
            .map(|(_idx, has_interr, annot)| periods::WeekDesc {
                interrogations: has_interr != 0,
                annotation: non_empty_string::NonEmptyString::new(annot).ok(),
            })
            .collect();

        let id = unsafe { PeriodId::new(period_id as u64) };
        ordered_period_list.push((id, weeks));
    }

    Ok(periods::Periods {
        first_week,
        ordered_period_list,
    })
}

async fn read_subjects(pool: &SqlitePool) -> Result<subjects::Subjects, Error> {
    let subject_rows: Vec<(i64, i64, String)> =
        sqlx::query_as("SELECT id, position, name FROM subjects ORDER BY position")
            .fetch_all(pool)
            .await?;

    let mut ordered_subject_list = Vec::new();

    for (subject_id, _position, name) in subject_rows {
        // Read excluded periods
        let excluded_period_rows: Vec<(i64,)> =
            sqlx::query_as("SELECT period_id FROM subject_excluded_periods WHERE subject_id = ?")
                .bind(subject_id)
                .fetch_all(pool)
                .await?;

        let excluded_periods: BTreeSet<PeriodId> = excluded_period_rows
            .into_iter()
            .map(|(pid,)| unsafe { PeriodId::new(pid as u64) })
            .collect();

        // Read interrogation parameters with inline periodicity columns
        let interr_params: Option<(
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
        )> = sqlx::query_as(
            "SELECT students_per_group_min, students_per_group_max,
                    groups_per_interrogation_min, groups_per_interrogation_max,
                    duration_minutes, take_duration_into_account,
                    ep_periodicity_in_weeks,
                    ofeb_weeks_per_block, ofeb_minimum_week_separation,
                    aiy_count_min, aiy_count_max, aiy_minimum_week_separation,
                    afab_minimum_week_separation
             FROM subject_interrogation_params WHERE subject_id = ?",
        )
        .bind(subject_id)
        .fetch_optional(pool)
        .await?;

        let interrogation_parameters = match interr_params {
            None => None,
            Some((
                spg_min,
                spg_max,
                gpi_min,
                gpi_max,
                duration,
                take_duration,
                ep,
                ofeb_wpb,
                ofeb_mws,
                aiy_min,
                aiy_max,
                aiy_mws,
                afab_mws,
            )) => {
                let periodicity = read_periodicity(
                    pool, subject_id, ep, ofeb_wpb, ofeb_mws, aiy_min, aiy_max, aiy_mws, afab_mws,
                )
                .await?;

                Some(subjects::SubjectInterrogationParameters {
                    students_per_group: NonZeroU32::new(spg_min as u32).unwrap()
                        ..=NonZeroU32::new(spg_max as u32).unwrap(),
                    groups_per_interrogation: NonZeroU32::new(gpi_min as u32).unwrap()
                        ..=NonZeroU32::new(gpi_max as u32).unwrap(),
                    duration: NonZeroMinutes::new(duration as u32).unwrap(),
                    take_duration_into_account: take_duration != 0,
                    periodicity,
                })
            }
        };

        let subject = subjects::Subject {
            parameters: subjects::SubjectParameters {
                name,
                interrogation_parameters,
            },
            excluded_periods,
        };

        let id = unsafe { SubjectId::new(subject_id as u64) };
        ordered_subject_list.push((id, subject));
    }

    Ok(subjects::Subjects {
        ordered_subject_list,
    })
}

async fn read_periodicity(
    pool: &SqlitePool,
    subject_id: i64,
    ep_periodicity_in_weeks: Option<i64>,
    ofeb_weeks_per_block: Option<i64>,
    ofeb_minimum_week_separation: Option<i64>,
    aiy_count_min: Option<i64>,
    aiy_count_max: Option<i64>,
    aiy_minimum_week_separation: Option<i64>,
    afab_minimum_week_separation: Option<i64>,
) -> Result<subjects::SubjectPeriodicity, Error> {
    // Determine periodicity type from which column is set
    if let Some(weeks) = ep_periodicity_in_weeks {
        return Ok(subjects::SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks: NonZeroU32::new(weeks as u32).unwrap(),
        });
    }

    if let (Some(wpb), Some(mws)) = (ofeb_weeks_per_block, ofeb_minimum_week_separation) {
        return Ok(subjects::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
            weeks_per_block: NonZeroU32::new(wpb as u32).unwrap(),
            minimum_week_separation: NonZeroU32::new(mws as u32).unwrap(),
        });
    }

    if let (Some(count_min), Some(count_max), Some(mws)) =
        (aiy_count_min, aiy_count_max, aiy_minimum_week_separation)
    {
        return Ok(subjects::SubjectPeriodicity::AmountInYear {
            interrogation_count_in_year: (count_min as u32)..=(count_max as u32),
            minimum_week_separation: mws as u32,
        });
    }

    if let Some(mws) = afab_minimum_week_separation {
        let block_rows: Vec<(i64, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT block_index, delay_in_weeks, size_in_weeks,
                    interrogation_count_min, interrogation_count_max
             FROM periodicity_week_blocks WHERE subject_id = ? ORDER BY block_index",
        )
        .bind(subject_id)
        .fetch_all(pool)
        .await?;

        let blocks: Vec<subjects::WeekBlock> = block_rows
            .into_iter()
            .map(
                |(_idx, delay, size, count_min, count_max)| subjects::WeekBlock {
                    delay_in_weeks: delay as u32,
                    size_in_weeks: NonZeroU32::new(size as u32).unwrap(),
                    interrogation_count_in_block: (count_min as u32)..=(count_max as u32),
                },
            )
            .collect();

        return Ok(subjects::SubjectPeriodicity::AmountForEveryArbitraryBlock {
            blocks,
            minimum_week_separation: mws as u32,
        });
    }

    // No periodicity found - this shouldn't happen if CHECK constraint is enforced
    Err(Error::MissingData(format!(
        "No periodicity found for subject {}",
        subject_id
    )))
}

async fn read_students(pool: &SqlitePool) -> Result<students::Students, Error> {
    let student_rows: Vec<(i64, String, String, String, String)> =
        sqlx::query_as("SELECT id, surname, firstname, tel, email FROM students")
            .fetch_all(pool)
            .await?;

    let mut student_map = BTreeMap::new();

    for (student_id, surname, firstname, tel, email) in student_rows {
        let excluded_period_rows: Vec<(i64,)> =
            sqlx::query_as("SELECT period_id FROM student_excluded_periods WHERE student_id = ?")
                .bind(student_id)
                .fetch_all(pool)
                .await?;

        let excluded_periods: BTreeSet<PeriodId> = excluded_period_rows
            .into_iter()
            .map(|(pid,)| unsafe { PeriodId::new(pid as u64) })
            .collect();

        let student = students::Student {
            desc: PersonWithContact {
                surname,
                firstname,
                tel: non_empty_string::NonEmptyString::new(tel).ok(),
                email: non_empty_string::NonEmptyString::new(email).ok(),
            },
            excluded_periods,
        };

        let id = unsafe { StudentId::new(student_id as u64) };
        student_map.insert(id, student);
    }

    Ok(students::Students { student_map })
}

async fn read_teachers(pool: &SqlitePool) -> Result<teachers::Teachers, Error> {
    let teacher_rows: Vec<(i64, String, String, String, String)> =
        sqlx::query_as("SELECT id, surname, firstname, tel, email FROM teachers")
            .fetch_all(pool)
            .await?;

    let mut teacher_map = BTreeMap::new();

    for (teacher_id, surname, firstname, tel, email) in teacher_rows {
        let subject_rows: Vec<(i64,)> =
            sqlx::query_as("SELECT subject_id FROM teacher_subjects WHERE teacher_id = ?")
                .bind(teacher_id)
                .fetch_all(pool)
                .await?;

        let subjects: BTreeSet<SubjectId> = subject_rows
            .into_iter()
            .map(|(sid,)| unsafe { SubjectId::new(sid as u64) })
            .collect();

        let teacher = teachers::Teacher {
            desc: PersonWithContact {
                surname,
                firstname,
                tel: non_empty_string::NonEmptyString::new(tel).ok(),
                email: non_empty_string::NonEmptyString::new(email).ok(),
            },
            subjects,
        };

        let id = unsafe { TeacherId::new(teacher_id as u64) };
        teacher_map.insert(id, teacher);
    }

    Ok(teachers::Teachers { teacher_map })
}

async fn read_week_patterns(
    pool: &SqlitePool,
    periods: &periods::Periods,
) -> Result<week_patterns::WeekPatterns, Error> {
    let pattern_rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM week_patterns")
        .fetch_all(pool)
        .await?;

    let mut week_pattern_map = BTreeMap::new();

    for (pattern_id, name) in pattern_rows {
        // Get all disabled weeks for this pattern
        let disabled_rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT period_id, week_index FROM week_pattern_disabled_weeks
             WHERE week_pattern_id = ?",
        )
        .bind(pattern_id)
        .fetch_all(pool)
        .await?;

        let disabled_set: BTreeSet<(i64, i64)> = disabled_rows.into_iter().collect();

        // Build flat Vec<bool> from periods, checking disabled set
        let mut weeks = Vec::new();
        for (period_id, period_weeks) in &periods.ordered_period_list {
            for week_index in 0..period_weeks.len() {
                let key = (period_id.inner() as i64, week_index as i64);
                weeks.push(!disabled_set.contains(&key));
            }
        }

        let pattern = week_patterns::WeekPattern { name, weeks };

        let id = unsafe { WeekPatternId::new(pattern_id as u64) };
        week_pattern_map.insert(id, pattern);
    }

    Ok(week_patterns::WeekPatterns { week_pattern_map })
}

async fn read_slots(pool: &SqlitePool) -> Result<slots::Slots, Error> {
    let slot_rows: Vec<(i64, i64, i64, i64, i64, String, String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT id, subject_id, position, teacher_id, day, start_time, extra_info, week_pattern_id, cost
         FROM slots ORDER BY subject_id, position",
    )
    .fetch_all(pool)
    .await?;

    let mut subject_map: BTreeMap<SubjectId, slots::SubjectSlots> = BTreeMap::new();

    for (
        slot_id,
        subject_id,
        _position,
        teacher_id,
        day,
        start_time_str,
        extra_info,
        week_pattern_id,
        cost,
    ) in slot_rows
    {
        let weekday = weekday_from_i64(day)?;
        let start_time = parse_time(&start_time_str)?;

        let slot = slots::Slot {
            teacher_id: unsafe { TeacherId::new(teacher_id as u64) },
            start_time: SlotStart {
                weekday,
                start_time,
            },
            extra_info,
            week_pattern: week_pattern_id.map(|id| unsafe { WeekPatternId::new(id as u64) }),
            cost: cost as i32,
        };

        let sid = unsafe { SubjectId::new(subject_id as u64) };
        let slot_id = unsafe { SlotId::new(slot_id as u64) };

        subject_map
            .entry(sid)
            .or_insert_with(|| slots::SubjectSlots {
                ordered_slots: Vec::new(),
            })
            .ordered_slots
            .push((slot_id, slot));
    }

    Ok(slots::Slots { subject_map })
}

async fn read_incompats(pool: &SqlitePool) -> Result<incompats::Incompats, Error> {
    let incompat_rows: Vec<(i64, i64, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, subject_id, name, minimum_free_slots, week_pattern_id FROM incompats",
    )
    .fetch_all(pool)
    .await?;

    let mut incompat_map = BTreeMap::new();

    for (incompat_id, subject_id, name, min_free, week_pattern_id) in incompat_rows {
        let slot_rows: Vec<(i64, i64, String, i64)> = sqlx::query_as(
            "SELECT slot_index, day, start_time, duration_minutes
             FROM incompat_slots WHERE incompat_id = ? ORDER BY slot_index",
        )
        .bind(incompat_id)
        .fetch_all(pool)
        .await?;

        let mut incompat_slots = Vec::new();
        for (_idx, day, start_time_str, duration) in slot_rows {
            let weekday = weekday_from_i64(day)?;
            let start_time = parse_time(&start_time_str)?;
            let duration = NonZeroMinutes::new(duration as u32).unwrap();

            let slot = SlotWithDuration::new(
                SlotStart {
                    weekday,
                    start_time,
                },
                duration,
            )
            .ok_or_else(|| Error::InvalidData("Slot crosses midnight".to_string()))?;

            incompat_slots.push(slot);
        }

        let incompat = incompats::Incompatibility {
            subject_id: unsafe { SubjectId::new(subject_id as u64) },
            name,
            slots: incompat_slots,
            minimum_free_slots: NonZeroU32::new(min_free as u32).unwrap(),
            week_pattern_id: week_pattern_id.map(|id| unsafe { WeekPatternId::new(id as u64) }),
        };

        let id = unsafe { IncompatId::new(incompat_id as u64) };
        incompat_map.insert(id, incompat);
    }

    Ok(incompats::Incompats { incompat_map })
}

async fn read_group_lists(pool: &SqlitePool) -> Result<group_lists::GroupLists, Error> {
    let list_rows: Vec<(i64, String, i64, i64, String)> = sqlx::query_as(
        "SELECT id, name, students_per_group_min, students_per_group_max, filling_type FROM group_lists",
    )
    .fetch_all(pool)
    .await?;

    let mut group_list_map = BTreeMap::new();

    for (list_id, name, spg_min, spg_max, filling_type) in list_rows {
        // Read group names
        let name_rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT group_index, name FROM group_list_group_names
             WHERE group_list_id = ? ORDER BY group_index",
        )
        .bind(list_id)
        .fetch_all(pool)
        .await?;

        let group_names: Vec<Option<non_empty_string::NonEmptyString>> = name_rows
            .into_iter()
            .map(|(_idx, name)| non_empty_string::NonEmptyString::new(name).ok())
            .collect();

        let filling = match filling_type.as_str() {
            "prefilled" => {
                let student_rows: Vec<(i64, i64)> = sqlx::query_as(
                    "SELECT group_index, student_id FROM prefilled_group_students
                     WHERE group_list_id = ? ORDER BY group_index",
                )
                .bind(list_id)
                .fetch_all(pool)
                .await?;

                let mut groups: Vec<group_lists::PrefilledGroup> = (0..group_names.len())
                    .map(|_| group_lists::PrefilledGroup::default())
                    .collect();

                for (group_index, student_id) in student_rows {
                    let student_id = unsafe { StudentId::new(student_id as u64) };
                    groups[group_index as usize].students.insert(student_id);
                }

                group_lists::GroupListFilling::Prefilled { groups }
            }
            "automatic" => {
                let excluded_rows: Vec<(i64,)> = sqlx::query_as(
                    "SELECT student_id FROM automatic_group_excluded_students WHERE group_list_id = ?",
                )
                .bind(list_id)
                .fetch_all(pool)
                .await?;

                let excluded_students: BTreeSet<StudentId> = excluded_rows
                    .into_iter()
                    .map(|(sid,)| unsafe { StudentId::new(sid as u64) })
                    .collect();

                group_lists::GroupListFilling::Automatic { excluded_students }
            }
            other => return Err(Error::InvalidFillingType(other.to_string())),
        };

        let group_list = group_lists::GroupList {
            params: group_lists::GroupListParameters {
                name,
                students_per_group: NonZeroU32::new(spg_min as u32).unwrap()
                    ..=NonZeroU32::new(spg_max as u32).unwrap(),
                group_names,
            },
            filling,
        };

        let id = unsafe { GroupListId::new(list_id as u64) };
        group_list_map.insert(id, group_list);
    }

    // Read subject associations
    let assoc_rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT period_id, subject_id, group_list_id FROM group_list_subject_associations",
    )
    .fetch_all(pool)
    .await?;

    let mut subjects_associations: BTreeMap<PeriodId, BTreeMap<SubjectId, GroupListId>> =
        BTreeMap::new();

    for (period_id, subject_id, group_list_id) in assoc_rows {
        let period_id = unsafe { PeriodId::new(period_id as u64) };
        let subject_id = unsafe { SubjectId::new(subject_id as u64) };
        let group_list_id = unsafe { GroupListId::new(group_list_id as u64) };

        subjects_associations
            .entry(period_id)
            .or_default()
            .insert(subject_id, group_list_id);
    }

    Ok(group_lists::GroupLists {
        group_list_map,
        subjects_associations,
    })
}

async fn read_assignments(pool: &SqlitePool) -> Result<assignments::Assignments, Error> {
    let rows: Vec<(i64, i64, i64)> =
        sqlx::query_as("SELECT period_id, subject_id, student_id FROM assignments")
            .fetch_all(pool)
            .await?;

    let mut period_map: BTreeMap<PeriodId, assignments::PeriodAssignments> = BTreeMap::new();

    for (period_id, subject_id, student_id) in rows {
        let period_id = unsafe { PeriodId::new(period_id as u64) };
        let subject_id = unsafe { SubjectId::new(subject_id as u64) };
        let student_id = unsafe { StudentId::new(student_id as u64) };

        period_map
            .entry(period_id)
            .or_insert_with(|| assignments::PeriodAssignments {
                subject_map: BTreeMap::new(),
            })
            .subject_map
            .entry(subject_id)
            .or_default()
            .insert(student_id);
    }

    Ok(assignments::Assignments { period_map })
}

async fn read_settings(pool: &SqlitePool) -> Result<settings::Settings, Error> {
    let global_row: Option<(
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    )> = sqlx::query_as(
        "SELECT interrogations_per_week_min_value, interrogations_per_week_min_soft,
                interrogations_per_week_max_value, interrogations_per_week_max_soft,
                max_interrogations_per_day_value, max_interrogations_per_day_soft
         FROM settings_global WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    let global = match global_row {
        Some((min_val, min_soft, max_val, max_soft, day_val, day_soft)) => settings::Limits {
            interrogations_per_week_min: match (min_val, min_soft) {
                (Some(v), Some(s)) => Some(settings::SoftParam {
                    value: v as u32,
                    soft: s != 0,
                }),
                _ => None,
            },
            interrogations_per_week_max: match (max_val, max_soft) {
                (Some(v), Some(s)) => Some(settings::SoftParam {
                    value: v as u32,
                    soft: s != 0,
                }),
                _ => None,
            },
            max_interrogations_per_day: match (day_val, day_soft) {
                (Some(v), Some(s)) => Some(settings::SoftParam {
                    value: NonZeroU32::new(v as u32).unwrap(),
                    soft: s != 0,
                }),
                _ => None,
            },
        },
        None => settings::Limits::default(),
    };

    let student_rows: Vec<(
        i64,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    )> = sqlx::query_as(
        "SELECT student_id,
                interrogations_per_week_min_value, interrogations_per_week_min_soft,
                interrogations_per_week_max_value, interrogations_per_week_max_soft,
                max_interrogations_per_day_value, max_interrogations_per_day_soft
         FROM settings_students",
    )
    .fetch_all(pool)
    .await?;

    let mut students_settings = BTreeMap::new();

    for (student_id, min_val, min_soft, max_val, max_soft, day_val, day_soft) in student_rows {
        let limits = settings::Limits {
            interrogations_per_week_min: match (min_val, min_soft) {
                (Some(v), Some(s)) => Some(settings::SoftParam {
                    value: v as u32,
                    soft: s != 0,
                }),
                _ => None,
            },
            interrogations_per_week_max: match (max_val, max_soft) {
                (Some(v), Some(s)) => Some(settings::SoftParam {
                    value: v as u32,
                    soft: s != 0,
                }),
                _ => None,
            },
            max_interrogations_per_day: match (day_val, day_soft) {
                (Some(v), Some(s)) => Some(settings::SoftParam {
                    value: NonZeroU32::new(v as u32).unwrap(),
                    soft: s != 0,
                }),
                _ => None,
            },
        };

        let id = unsafe { StudentId::new(student_id as u64) };
        students_settings.insert(id, limits);
    }

    Ok(settings::Settings {
        global,
        students: students_settings,
    })
}

async fn read_main_script(pool: &SqlitePool) -> Result<Option<String>, Error> {
    let result: Option<Option<String>> =
        sqlx::query_scalar("SELECT main_script FROM metadata WHERE id = 1")
            .fetch_optional(pool)
            .await?;

    Ok(result.flatten())
}

async fn read_colloscope(
    pool: &SqlitePool,
    params: &colloscope_params::Parameters,
) -> Result<colloscopes::Colloscope, Error> {
    // Read colloscope slots
    let slot_rows: Vec<(i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT period_id, slot_id, week_index, has_interrogation FROM colloscope_slots
         ORDER BY period_id, slot_id, week_index",
    )
    .fetch_all(pool)
    .await?;

    let mut period_map: BTreeMap<PeriodId, colloscopes::ColloscopePeriod> = BTreeMap::new();

    // Initialize periods from params
    for (period_id, _) in &params.periods.ordered_period_list {
        period_map.insert(
            *period_id,
            colloscopes::ColloscopePeriod {
                slot_map: BTreeMap::new(),
            },
        );
    }

    // Group rows by period and slot
    let mut current_period: Option<PeriodId> = None;
    let mut current_slot: Option<SlotId> = None;
    let mut current_interrogations: Vec<Option<colloscopes::ColloscopeInterrogation>> = Vec::new();

    for (period_id, slot_id, week_index, has_interr) in &slot_rows {
        let period_id = unsafe { PeriodId::new(*period_id as u64) };
        let slot_id = unsafe { SlotId::new(*slot_id as u64) };

        // Check if we need to save the previous slot
        if current_period != Some(period_id) || current_slot != Some(slot_id) {
            if let (Some(p), Some(s)) = (current_period, current_slot) {
                if let Some(period) = period_map.get_mut(&p) {
                    period.slot_map.insert(
                        s,
                        colloscopes::ColloscopeSlot {
                            interrogations: std::mem::take(&mut current_interrogations),
                        },
                    );
                }
            }
            current_period = Some(period_id);
            current_slot = Some(slot_id);
            current_interrogations = Vec::new();
        }

        // Ensure we have enough slots
        while current_interrogations.len() <= *week_index as usize {
            current_interrogations.push(None);
        }

        if *has_interr != 0 {
            current_interrogations[*week_index as usize] =
                Some(colloscopes::ColloscopeInterrogation {
                    assigned_groups: BTreeSet::new(),
                });
        }
    }

    // Save the last slot
    if let (Some(p), Some(s)) = (current_period, current_slot) {
        if let Some(period) = period_map.get_mut(&p) {
            period.slot_map.insert(
                s,
                colloscopes::ColloscopeSlot {
                    interrogations: current_interrogations,
                },
            );
        }
    }

    // Read interrogation groups
    let group_rows: Vec<(i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT period_id, slot_id, week_index, group_number FROM colloscope_interrogation_groups",
    )
    .fetch_all(pool)
    .await?;

    for (period_id, slot_id, week_index, group_number) in group_rows {
        let period_id = unsafe { PeriodId::new(period_id as u64) };
        let slot_id = unsafe { SlotId::new(slot_id as u64) };

        if let Some(period) = period_map.get_mut(&period_id) {
            if let Some(slot) = period.slot_map.get_mut(&slot_id) {
                if let Some(Some(interr)) = slot.interrogations.get_mut(week_index as usize) {
                    interr.assigned_groups.insert(group_number as u32);
                }
            }
        }
    }

    // Read group list student assignments
    let gl_rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT group_list_id, student_id, group_number FROM colloscope_group_list_students",
    )
    .fetch_all(pool)
    .await?;

    let mut group_lists: BTreeMap<GroupListId, colloscopes::ColloscopeGroupList> = BTreeMap::new();

    // Initialize non-prefilled group lists from params
    for (group_list_id, group_list) in &params.group_lists.group_list_map {
        if !group_list.is_prefilled() {
            group_lists.insert(
                *group_list_id,
                colloscopes::ColloscopeGroupList {
                    groups_for_students: BTreeMap::new(),
                },
            );
        }
    }

    for (group_list_id, student_id, group_number) in gl_rows {
        let group_list_id = unsafe { GroupListId::new(group_list_id as u64) };
        let student_id = unsafe { StudentId::new(student_id as u64) };

        group_lists
            .entry(group_list_id)
            .or_insert_with(|| colloscopes::ColloscopeGroupList {
                groups_for_students: BTreeMap::new(),
            })
            .groups_for_students
            .insert(student_id, group_number as u32);
    }

    Ok(colloscopes::Colloscope {
        period_map,
        group_lists,
    })
}

// ============================================================================
// Validation helpers
// ============================================================================

async fn validate_teacher_subject_consistency(pool: &SqlitePool) -> Result<(), ValidationError> {
    let invalid_slots: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT s.id, s.teacher_id, s.subject_id FROM slots s
         WHERE NOT EXISTS (
             SELECT 1 FROM teacher_subjects ts
             WHERE ts.teacher_id = s.teacher_id AND ts.subject_id = s.subject_id
         )",
    )
    .fetch_all(pool)
    .await?;

    if let Some((slot_id, teacher_id, subject_id)) = invalid_slots.first() {
        return Err(ValidationError::TeacherSubjectMismatch {
            teacher_id: *teacher_id,
            subject_id: *subject_id,
            slot_id: *slot_id,
        });
    }

    Ok(())
}

async fn validate_assignment_exclusions(pool: &SqlitePool) -> Result<(), ValidationError> {
    // Check student exclusions
    let invalid_student_assignments: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT a.student_id, a.subject_id, a.period_id FROM assignments a
         INNER JOIN student_excluded_periods sep
         ON a.student_id = sep.student_id AND a.period_id = sep.period_id",
    )
    .fetch_all(pool)
    .await?;

    if let Some((student_id, subject_id, period_id)) = invalid_student_assignments.first() {
        return Err(ValidationError::StudentExcludedFromPeriod {
            student_id: *student_id,
            subject_id: *subject_id,
            period_id: *period_id,
        });
    }

    // Check subject exclusions
    let invalid_subject_assignments: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT DISTINCT a.subject_id, a.period_id FROM assignments a
         INNER JOIN subject_excluded_periods sep
         ON a.subject_id = sep.subject_id AND a.period_id = sep.period_id",
    )
    .fetch_all(pool)
    .await?;

    if let Some((subject_id, period_id)) = invalid_subject_assignments.first() {
        return Err(ValidationError::SubjectExcludedFromPeriod {
            subject_id: *subject_id,
            period_id: *period_id,
        });
    }

    Ok(())
}

async fn validate_group_list_filling_consistency(pool: &SqlitePool) -> Result<(), ValidationError> {
    // Check for prefilled data in automatic lists
    let invalid_prefilled: Vec<(i64,)> = sqlx::query_as(
        "SELECT DISTINCT pgs.group_list_id FROM prefilled_group_students pgs
         INNER JOIN group_lists gl ON pgs.group_list_id = gl.id
         WHERE gl.filling_type = 'automatic'",
    )
    .fetch_all(pool)
    .await?;

    if let Some((group_list_id,)) = invalid_prefilled.first() {
        return Err(ValidationError::PrefilledDataForAutomaticGroupList {
            group_list_id: *group_list_id,
        });
    }

    // Check for automatic exclusions in prefilled lists
    let invalid_automatic: Vec<(i64,)> = sqlx::query_as(
        "SELECT DISTINCT ages.group_list_id FROM automatic_group_excluded_students ages
         INNER JOIN group_lists gl ON ages.group_list_id = gl.id
         WHERE gl.filling_type = 'prefilled'",
    )
    .fetch_all(pool)
    .await?;

    if let Some((group_list_id,)) = invalid_automatic.first() {
        return Err(ValidationError::AutomaticExclusionForPrefilledGroupList {
            group_list_id: *group_list_id,
        });
    }

    Ok(())
}

// ============================================================================
// Utility functions
// ============================================================================

fn weekday_from_i64(day: i64) -> Result<Weekday, Error> {
    let chrono_weekday = match day {
        0 => chrono::Weekday::Mon,
        1 => chrono::Weekday::Tue,
        2 => chrono::Weekday::Wed,
        3 => chrono::Weekday::Thu,
        4 => chrono::Weekday::Fri,
        5 => chrono::Weekday::Sat,
        6 => chrono::Weekday::Sun,
        _ => return Err(Error::InvalidWeekday(day)),
    };
    Ok(Weekday(chrono_weekday))
}

fn parse_time(time_str: &str) -> Result<WholeMinuteTime, Error> {
    let naive_time = chrono::NaiveTime::parse_from_str(time_str, "%H:%M")
        .map_err(|_| Error::InvalidTimeFormat(time_str.to_string()))?;
    WholeMinuteTime::new(naive_time).ok_or_else(|| Error::InvalidTimeFormat(time_str.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_schema() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        create_schema(&pool).await.unwrap();

        // Verify some tables exist
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='students'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(result.0, 1);
    }

    #[tokio::test]
    async fn test_empty_data_round_trip() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        create_schema(&pool).await.unwrap();

        let original = InnerData::default();
        inner_data_to_sqlite(&pool, &original).await.unwrap();

        let restored = sqlite_to_inner_data(&pool).await.unwrap();

        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn test_complex_data_round_trip() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        create_schema(&pool).await.unwrap();

        // Create complex test data
        let mut original = InnerData::default();

        // Add a period with 4 weeks
        let period_id = unsafe { PeriodId::new(1) };
        original.params.periods.ordered_period_list.push((
            period_id,
            vec![
                periods::WeekDesc::default(),
                periods::WeekDesc::new(false), // No interrogations this week
                periods::WeekDesc::default(),
                periods::WeekDesc::default(),
            ],
        ));

        // Add a subject with interrogations
        let subject_id = unsafe { SubjectId::new(2) };
        original.params.subjects.ordered_subject_list.push((
            subject_id,
            subjects::Subject {
                parameters: subjects::SubjectParameters {
                    name: "Math".to_string(),
                    interrogation_parameters: Some(subjects::SubjectInterrogationParameters {
                        students_per_group: NonZeroU32::new(2).unwrap()
                            ..=NonZeroU32::new(3).unwrap(),
                        groups_per_interrogation: NonZeroU32::new(1).unwrap()
                            ..=NonZeroU32::new(1).unwrap(),
                        duration: NonZeroMinutes::new(60).unwrap(),
                        take_duration_into_account: true,
                        periodicity: subjects::SubjectPeriodicity::ExactlyPeriodic {
                            periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
                        },
                    }),
                },
                excluded_periods: BTreeSet::new(),
            },
        ));

        // Add a student
        let student_id = unsafe { StudentId::new(3) };
        original.params.students.student_map.insert(
            student_id,
            students::Student {
                desc: PersonWithContact {
                    surname: "Doe".to_string(),
                    firstname: "John".to_string(),
                    tel: Some(
                        non_empty_string::NonEmptyString::new("0123456789".to_string()).unwrap(),
                    ),
                    email: None,
                },
                excluded_periods: BTreeSet::new(),
            },
        );

        // Add a teacher that teaches the subject
        let teacher_id = unsafe { TeacherId::new(4) };
        let mut teacher_subjects = BTreeSet::new();
        teacher_subjects.insert(subject_id);
        original.params.teachers.teacher_map.insert(
            teacher_id,
            teachers::Teacher {
                desc: PersonWithContact {
                    surname: "Smith".to_string(),
                    firstname: "Jane".to_string(),
                    tel: None,
                    email: Some(
                        non_empty_string::NonEmptyString::new("jane@example.com".to_string())
                            .unwrap(),
                    ),
                },
                subjects: teacher_subjects,
            },
        );

        // Add a slot
        let slot_id = unsafe { SlotId::new(5) };
        let slot = slots::Slot {
            teacher_id,
            start_time: SlotStart {
                weekday: Weekday(chrono::Weekday::Mon),
                start_time: WholeMinuteTime::new(
                    chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                )
                .unwrap(),
            },
            extra_info: "Room 101".to_string(),
            week_pattern: None,
            cost: 0,
        };
        original.params.slots.subject_map.insert(
            subject_id,
            slots::SubjectSlots {
                ordered_slots: vec![(slot_id, slot)],
            },
        );

        // Add a group list
        let group_list_id = unsafe { GroupListId::new(6) };
        original.params.group_lists.group_list_map.insert(
            group_list_id,
            group_lists::GroupList {
                params: group_lists::GroupListParameters {
                    name: "Main groups".to_string(),
                    students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    group_names: vec![None, None, None, None],
                },
                filling: group_lists::GroupListFilling::Automatic {
                    excluded_students: BTreeSet::new(),
                },
            },
        );

        // Associate group list with period/subject
        original
            .params
            .group_lists
            .subjects_associations
            .insert(period_id, {
                let mut map = BTreeMap::new();
                map.insert(subject_id, group_list_id);
                map
            });

        // Add assignment
        original.params.assignments.period_map.insert(
            period_id,
            assignments::PeriodAssignments {
                subject_map: {
                    let mut map = BTreeMap::new();
                    let mut students = BTreeSet::new();
                    students.insert(student_id);
                    map.insert(subject_id, students);
                    map
                },
            },
        );

        // Add colloscope data
        let mut period_slot_map = BTreeMap::new();
        period_slot_map.insert(
            slot_id,
            colloscopes::ColloscopeSlot {
                interrogations: vec![
                    Some(colloscopes::ColloscopeInterrogation::default()),
                    None, // No interrogation on week with has_interrogations=false
                    Some(colloscopes::ColloscopeInterrogation::default()),
                    Some(colloscopes::ColloscopeInterrogation::default()),
                ],
            },
        );
        original.colloscope.period_map.insert(
            period_id,
            colloscopes::ColloscopePeriod {
                slot_map: period_slot_map,
            },
        );

        // Add colloscope group list (for automatic group lists)
        original.colloscope.group_lists.insert(
            group_list_id,
            colloscopes::ColloscopeGroupList {
                groups_for_students: BTreeMap::new(),
            },
        );

        // Round-trip test
        inner_data_to_sqlite(&pool, &original).await.unwrap();
        let restored = sqlite_to_inner_data(&pool).await.unwrap();

        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn test_all_periodicity_types_round_trip() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        create_schema(&pool).await.unwrap();

        let mut original = InnerData::default();

        // Add a period
        let period_id = unsafe { PeriodId::new(1) };
        original
            .params
            .periods
            .ordered_period_list
            .push((period_id, vec![periods::WeekDesc::default(); 10]));

        // Test all 4 periodicity types
        let periodicities = vec![
            subjects::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block: NonZeroU32::new(2).unwrap(),
                minimum_week_separation: NonZeroU32::new(1).unwrap(),
            },
            subjects::SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks: NonZeroU32::new(3).unwrap(),
            },
            subjects::SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year: 2..=4,
                minimum_week_separation: 1,
            },
            subjects::SubjectPeriodicity::AmountForEveryArbitraryBlock {
                blocks: vec![
                    subjects::WeekBlock {
                        delay_in_weeks: 0,
                        size_in_weeks: NonZeroU32::new(3).unwrap(),
                        interrogation_count_in_block: 1..=2,
                    },
                    subjects::WeekBlock {
                        delay_in_weeks: 1,
                        size_in_weeks: NonZeroU32::new(4).unwrap(),
                        interrogation_count_in_block: 1..=3,
                    },
                ],
                minimum_week_separation: 2,
            },
        ];

        for (i, periodicity) in periodicities.into_iter().enumerate() {
            let subject_id = unsafe { SubjectId::new((i + 10) as u64) };
            original.params.subjects.ordered_subject_list.push((
                subject_id,
                subjects::Subject {
                    parameters: subjects::SubjectParameters {
                        name: format!("Subject {}", i),
                        interrogation_parameters: Some(subjects::SubjectInterrogationParameters {
                            students_per_group: NonZeroU32::new(2).unwrap()
                                ..=NonZeroU32::new(3).unwrap(),
                            groups_per_interrogation: NonZeroU32::new(1).unwrap()
                                ..=NonZeroU32::new(1).unwrap(),
                            duration: NonZeroMinutes::new(60).unwrap(),
                            take_duration_into_account: true,
                            periodicity,
                        }),
                    },
                    excluded_periods: BTreeSet::new(),
                },
            ));
        }

        // Initialize the colloscope with an empty period entry (consistent with params)
        original.colloscope.period_map.insert(
            period_id,
            colloscopes::ColloscopePeriod {
                slot_map: BTreeMap::new(),
            },
        );

        // Round-trip test
        inner_data_to_sqlite(&pool, &original).await.unwrap();
        let restored = sqlite_to_inner_data(&pool).await.unwrap();

        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn test_validation_passes_for_valid_data() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        create_schema(&pool).await.unwrap();

        let original = InnerData::default();
        inner_data_to_sqlite(&pool, &original).await.unwrap();

        // Validation should pass for valid empty data
        validate_database(&pool).await.unwrap();
    }
}
