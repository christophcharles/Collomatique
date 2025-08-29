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

async fn search_invalid_subject_group_id(
    pool: &SqlitePool,
    subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> Result<Option<super::subject_groups::Id>> {
    let subject_groups_ids = sqlx::query!("SELECT subject_group_id FROM subject_groups")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?
        .iter()
        .map(|x| x.subject_group_id)
        .collect::<BTreeSet<_>>();

    if !subject_groups_ids.contains(&subject.subject_group_id.0) {
        return Ok(Some(subject.subject_group_id));
    }

    Ok(None)
}

async fn search_invalid_incompat_id(
    pool: &SqlitePool,
    subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> Result<Option<super::incompats::Id>> {
    if let Some(incompat_id) = subject.incompat_id {
        let incompat_ids = sqlx::query!("SELECT incompat_id FROM incompats")
            .fetch_all(pool)
            .await
            .map_err(Error::from)?
            .iter()
            .map(|x| x.incompat_id)
            .collect::<BTreeSet<_>>();

        if !incompat_ids.contains(&incompat_id.0) {
            return Ok(Some(incompat_id));
        }
    }

    Ok(None)
}

async fn search_invalid_group_list_id(
    pool: &SqlitePool,
    subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> Result<Option<super::group_lists::Id>> {
    if let Some(group_list_id) = subject.group_list_id {
        let group_list_ids = sqlx::query!("SELECT group_list_id FROM group_lists")
            .fetch_all(pool)
            .await
            .map_err(Error::from)?
            .iter()
            .map(|x| x.group_list_id)
            .collect::<BTreeSet<_>>();

        if !group_list_ids.contains(&group_list_id.0) {
            return Ok(Some(group_list_id));
        }
    }

    Ok(None)
}

trait CheckIds: std::fmt::Debug + std::error::Error + Sized {
    async fn check_ids(
        pool: &SqlitePool,
        subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
    ) -> std::result::Result<(), Self>;
}

impl CheckIds
    for Cross3Error<Error, super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>
{
    async fn check_ids(
        pool: &SqlitePool,
        subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
    ) -> std::result::Result<(), Self> {
        if let Some(subject_group_id) = search_invalid_subject_group_id(pool, subject).await? {
            return Err(Cross3Error::InvalidCrossId1(subject_group_id));
        }

        if let Some(incompat_id) = search_invalid_incompat_id(pool, subject).await? {
            return Err(Cross3Error::InvalidCrossId2(incompat_id));
        }

        if let Some(group_list_id) = search_invalid_group_list_id(pool, subject).await? {
            return Err(Cross3Error::InvalidCrossId3(group_list_id));
        }

        Ok(())
    }
}

pub async fn add(
    pool: &SqlitePool,
    subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> std::result::Result<
    Id,
    Cross3Error<Error, super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
> {
    Cross3Error::check_ids(pool, subject).await?;

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

impl CheckIds
    for Cross3IdError<
        Error,
        Id,
        super::subject_groups::Id,
        super::incompats::Id,
        super::group_lists::Id,
    >
{
    async fn check_ids(
        pool: &SqlitePool,
        subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
    ) -> std::result::Result<(), Self> {
        if let Some(subject_group_id) = search_invalid_subject_group_id(pool, subject).await? {
            return Err(Cross3IdError::InvalidCrossId1(subject_group_id));
        }

        if let Some(incompat_id) = search_invalid_incompat_id(pool, subject).await? {
            return Err(Cross3IdError::InvalidCrossId2(incompat_id));
        }

        if let Some(group_list_id) = search_invalid_group_list_id(pool, subject).await? {
            return Err(Cross3IdError::InvalidCrossId3(group_list_id));
        }

        Ok(())
    }
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
) -> std::result::Result<
    (),
    Cross3IdError<
        Error,
        Id,
        super::subject_groups::Id,
        super::incompats::Id,
        super::group_lists::Id,
    >,
> {
    Cross3IdError::check_ids(pool, subject).await?;

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
        return Err(Cross3IdError::InternalError(Error::CorruptedDatabase(
            format!("Multiple subjects with id {:?}", index),
        )));
    } else if rows_affected == 0 {
        return Err(Cross3IdError::InvalidId(index));
    }

    Ok(())
}
