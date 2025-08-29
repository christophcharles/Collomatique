use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    pool: &SqlitePool,
) -> std::result::Result<
    BTreeMap<Id, Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>>,
    Error,
> {
    let records = sqlx::query!(
        r#"
SELECT subject_id, name, subject_group_id, incompat_id, group_list_id, duration,
min_students_per_group, max_students_per_group, period, period_is_strict, is_tutorial, max_groups_per_slot, balance_teachers, balance_timeslots
FROM subjects
        "#
    )
    .fetch_all(pool)
    .await.map_err(Error::from)?;

    let mut output = BTreeMap::new();

    for record in records {
        let duration = NonZeroU32::new(u32::try_from(record.duration).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "invalid duration ({}) stored in database",
                record.duration
            ))
        })?)
        .ok_or(Error::CorruptedDatabase(format!(
            "invalid duration ({}) stored in database",
            record.duration
        )))?;
        let min_students_per_group =
            NonZeroUsize::new(usize::try_from(record.min_students_per_group).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "invalid min_students_per_group ({}) stored in database",
                    record.duration
                ))
            })?)
            .ok_or(Error::CorruptedDatabase(format!(
                "invalid min_students_per_group ({}) stored in database",
                record.duration
            )))?;
        let max_students_per_group =
            NonZeroUsize::new(usize::try_from(record.max_students_per_group).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "invalid max_students_per_group ({}) stored in database",
                    record.duration
                ))
            })?)
            .ok_or(Error::CorruptedDatabase(format!(
                "invalid max_students_per_group ({}) stored in database",
                record.duration
            )))?;
        let students_per_group = min_students_per_group..=max_students_per_group;

        let period = NonZeroU32::new(u32::try_from(record.period).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "invalid period ({}) stored in database",
                record.duration
            ))
        })?)
        .ok_or(Error::CorruptedDatabase(format!(
            "invalid period ({}) stored in database",
            record.duration
        )))?;

        let max_groups_per_slot =
            NonZeroUsize::new(usize::try_from(record.max_groups_per_slot).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "invalid max_groups_per_slot ({}) stored in database",
                    record.duration
                ))
            })?)
            .ok_or(Error::CorruptedDatabase(format!(
                "invalid max_groups_per_slot ({}) stored in database",
                record.duration
            )))?;

        output.insert(
            Id(record.subject_id),
            Subject {
                name: record.name,
                subject_group_id: subject_groups::Id(record.subject_group_id),
                incompat_id: record.incompat_id.map(|id| incompats::Id(id)),
                group_list_id: record.group_list_id.map(|id| group_lists::Id(id)),
                duration,
                students_per_group: students_per_group,
                period: period,
                period_is_strict: record.period_is_strict != 0,
                is_tutorial: record.is_tutorial != 0,
                max_groups_per_slot,
                balancing_requirements: BalancingRequirements {
                    teachers: record.balance_teachers != 0,
                    timeslots: record.balance_timeslots != 0,
                },
            },
        );
    }

    Ok(output)
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<
    Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
    IdError<Error, Id>,
> {
    let record_opt = sqlx::query!(
        r#"
SELECT name, subject_group_id, incompat_id, group_list_id, duration,
min_students_per_group, max_students_per_group, period, period_is_strict, is_tutorial, max_groups_per_slot, balance_teachers, balance_timeslots
FROM subjects WHERE subject_id = ?
        "#,
        index.0
    )
    .fetch_optional(pool)
    .await.map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    let duration = NonZeroU32::new(u32::try_from(record.duration).map_err(|_| {
        Error::CorruptedDatabase(format!(
            "invalid duration ({}) stored in database",
            record.duration
        ))
    })?)
    .ok_or(Error::CorruptedDatabase(format!(
        "invalid duration ({}) stored in database",
        record.duration
    )))?;
    let min_students_per_group =
        NonZeroUsize::new(usize::try_from(record.min_students_per_group).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "invalid min_students_per_group ({}) stored in database",
                record.duration
            ))
        })?)
        .ok_or(Error::CorruptedDatabase(format!(
            "invalid min_students_per_group ({}) stored in database",
            record.duration
        )))?;
    let max_students_per_group =
        NonZeroUsize::new(usize::try_from(record.max_students_per_group).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "invalid max_students_per_group ({}) stored in database",
                record.duration
            ))
        })?)
        .ok_or(Error::CorruptedDatabase(format!(
            "invalid max_students_per_group ({}) stored in database",
            record.duration
        )))?;
    let students_per_group = min_students_per_group..=max_students_per_group;

    let period = NonZeroU32::new(u32::try_from(record.period).map_err(|_| {
        Error::CorruptedDatabase(format!(
            "invalid period ({}) stored in database",
            record.duration
        ))
    })?)
    .ok_or(Error::CorruptedDatabase(format!(
        "invalid period ({}) stored in database",
        record.duration
    )))?;

    let max_groups_per_slot =
        NonZeroUsize::new(usize::try_from(record.max_groups_per_slot).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "invalid max_groups_per_slot ({}) stored in database",
                record.duration
            ))
        })?)
        .ok_or(Error::CorruptedDatabase(format!(
            "invalid max_groups_per_slot ({}) stored in database",
            record.duration
        )))?;

    let output = Subject {
        name: record.name,
        subject_group_id: subject_groups::Id(record.subject_group_id),
        incompat_id: record.incompat_id.map(|id| incompats::Id(id)),
        group_list_id: record.group_list_id.map(|id| group_lists::Id(id)),
        duration,
        students_per_group: students_per_group,
        period: period,
        period_is_strict: record.period_is_strict != 0,
        is_tutorial: record.is_tutorial != 0,
        max_groups_per_slot,
        balancing_requirements: BalancingRequirements {
            teachers: record.balance_teachers != 0,
            timeslots: record.balance_timeslots != 0,
        },
    };

    Ok(output)
}

pub async fn add(
    pool: &SqlitePool,
    subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> std::result::Result<Id, Error> {
    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let min_students_per_group =
        i64::try_from(subject.students_per_group.start().get()).map_err(|_| {
            Error::RepresentationError(format!(
                "cannot represent as i64 min_students_per_group with value {}",
                subject.students_per_group.start().get()
            ))
        })?;
    let max_students_per_group =
        i64::try_from(subject.students_per_group.end().get()).map_err(|_| {
            Error::RepresentationError(format!(
                "cannot represent as i64 max_students_per_group with value {}",
                subject.students_per_group.end().get()
            ))
        })?;

    let max_groups_per_slot = i64::try_from(subject.max_groups_per_slot.get()).map_err(|_| {
        Error::RepresentationError(format!(
            "cannot represent as i64 max_groups_per_slot with value {}",
            subject.max_groups_per_slot.get()
        ))
    })?;

    let incompat_id = subject.incompat_id.map(|x| x.0);
    let group_list_id = subject.group_list_id.map(|x| x.0);
    let duration = subject.duration.get();
    let period = subject.period.get();
    let period_is_strict = if subject.period_is_strict { 1 } else { 0 };
    let is_tutorial = if subject.is_tutorial { 1 } else { 0 };
    let balance_teachers = if subject.balancing_requirements.teachers {
        1
    } else {
        0
    };
    let balance_timeslots = if subject.balancing_requirements.timeslots {
        1
    } else {
        0
    };

    let subject_id = sqlx::query!(
        r#"
INSERT INTO subjects
(name, subject_group_id, incompat_id, group_list_id,
duration, min_students_per_group, max_students_per_group, period, period_is_strict,
is_tutorial, max_groups_per_slot, balance_teachers, balance_timeslots)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13);
        "#,
        subject.name,
        subject.subject_group_id.0,
        incompat_id,
        group_list_id,
        duration,
        min_students_per_group,
        max_students_per_group,
        period,
        period_is_strict,
        is_tutorial,
        max_groups_per_slot,
        balance_teachers,
        balance_timeslots,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .last_insert_rowid();

    Ok(Id(subject_id))
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    let subject_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let count = sqlx::query!("DELETE FROM subjects WHERE subject_id = ?", subject_id)
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .rows_affected();

    if count > 1 {
        return Err(IdError::InternalError(Error::CorruptedDatabase(format!(
            "Multiple subjects with id {:?}",
            index
        ))));
    } else if count == 0 {
        return Err(IdError::InvalidId(index));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> std::result::Result<(), Error> {
    let subject_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let min_students_per_group =
        i64::try_from(subject.students_per_group.start().get()).map_err(|_| {
            Error::RepresentationError(format!(
                "cannot represent as i64 min_students_per_group with value {}",
                subject.students_per_group.start().get()
            ))
        })?;
    let max_students_per_group =
        i64::try_from(subject.students_per_group.end().get()).map_err(|_| {
            Error::RepresentationError(format!(
                "cannot represent as i64 max_students_per_group with value {}",
                subject.students_per_group.end().get()
            ))
        })?;

    let max_groups_per_slot = i64::try_from(subject.max_groups_per_slot.get()).map_err(|_| {
        Error::RepresentationError(format!(
            "cannot represent as i64 max_groups_per_slot with value {}",
            subject.max_groups_per_slot.get()
        ))
    })?;

    let incompat_id = subject.incompat_id.map(|x| x.0);
    let group_list_id = subject.group_list_id.map(|x| x.0);
    let duration = subject.duration.get();
    let period = subject.period.get();
    let period_is_strict = if subject.period_is_strict { 1 } else { 0 };
    let is_tutorial = if subject.is_tutorial { 1 } else { 0 };
    let balance_teachers = if subject.balancing_requirements.teachers {
        1
    } else {
        0
    };
    let balance_timeslots = if subject.balancing_requirements.timeslots {
        1
    } else {
        0
    };

    let rows_affected = sqlx::query!(
        r#"
UPDATE subjects
SET name = ?1, subject_group_id = ?2, incompat_id = ?3, group_list_id = ?4,
duration = ?5, min_students_per_group = ?6, max_students_per_group = ?7, period = ?8, period_is_strict = ?9,
is_tutorial = ?10, max_groups_per_slot = ?11, balance_teachers = ?12, balance_timeslots = ?13
WHERE subject_id = ?14
        "#,
        subject.name,
        subject.subject_group_id.0,
        incompat_id,
        group_list_id,
        duration,
        min_students_per_group,
        max_students_per_group,
        period,
        period_is_strict,
        is_tutorial,
        max_groups_per_slot,
        balance_teachers,
        balance_timeslots,
        subject_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple subjects with id {:?}",
            index
        )));
    }

    Ok(())
}
