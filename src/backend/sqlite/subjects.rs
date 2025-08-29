use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    _pool: &SqlitePool,
) -> std::result::Result<
    BTreeMap<Id, Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>>,
    Error,
> {
    todo!()
}

pub async fn get(
    _pool: &SqlitePool,
    _index: Id,
) -> std::result::Result<
    Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
    IdError<Error, Id>,
> {
    todo!()
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

pub async fn remove(_pool: &SqlitePool, _index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    todo!()
}

pub async fn update(
    _pool: &SqlitePool,
    _index: Id,
    _subject: &Subject<super::subject_groups::Id, super::incompats::Id, super::group_lists::Id>,
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
    todo!()
}
