use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        Ok(())
    }
}

pub async fn get_all(
    pool: &SqlitePool,
) -> std::result::Result<BTreeMap<Id, GroupingIncompat<super::groupings::Id>>, Error> {
    let records = sqlx::query!("SELECT grouping_incompat_id, max_count FROM grouping_incompats")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

    let data =
        sqlx::query!("SELECT grouping_incompat_id, grouping_id FROM grouping_incompat_items")
            .fetch_all(pool)
            .await
            .map_err(Error::from)?;

    let mut output = BTreeMap::new();

    for record in records {
        let max_count_usize = usize::try_from(record.max_count).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "max_count (= {}) does not fit in usize for grouping_incompat {}",
                record.max_count, record.grouping_incompat_id
            ))
        })?;
        let max_count =
            NonZeroUsize::new(max_count_usize).ok_or(Error::CorruptedDatabase(format!(
                "max_count (= {}) does not fit in NonZeroUsize for grouping_incompat {}",
                record.max_count, record.grouping_incompat_id
            )))?;

        output.insert(
            Id(record.grouping_incompat_id),
            GroupingIncompat {
                max_count,
                groupings: data
                    .iter()
                    .filter_map(|x| {
                        if x.grouping_incompat_id != record.grouping_incompat_id {
                            return None;
                        }
                        Some(groupings::Id(x.grouping_id))
                    })
                    .collect(),
            },
        );
    }

    Ok(output)
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<GroupingIncompat<super::groupings::Id>, IdError<Error, Id>> {
    let grouping_incompat_id = index.0;

    let record_opt = sqlx::query!(
        "SELECT max_count FROM grouping_incompats WHERE grouping_incompat_id = ?",
        grouping_incompat_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    let data = sqlx::query!(
        "SELECT grouping_id FROM grouping_incompat_items WHERE grouping_incompat_id = ?",
        grouping_incompat_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    let max_count_usize = usize::try_from(record.max_count).map_err(|_| {
        Error::CorruptedDatabase(format!(
            "max_count (= {}) does not fit in usize for grouping_incompat {}",
            record.max_count, grouping_incompat_id
        ))
    })?;
    let max_count = NonZeroUsize::new(max_count_usize).ok_or(Error::CorruptedDatabase(format!(
        "max_count (= {}) does not fit in NonZeroUsize for grouping_incompat {}",
        record.max_count, grouping_incompat_id
    )))?;

    Ok(GroupingIncompat {
        max_count,
        groupings: data
            .into_iter()
            .map(|x| groupings::Id(x.grouping_id))
            .collect(),
    })
}

pub async fn add(
    pool: &SqlitePool,
    grouping_incompat: &GroupingIncompat<super::groupings::Id>,
) -> std::result::Result<Id, Error> {
    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let max_count = i64::try_from(grouping_incompat.max_count.get()).map_err(|_| {
        Error::RepresentationError(format!(
            "max_count (= {}) does not fit in i64",
            grouping_incompat.max_count.get()
        ))
    })?;

    let grouping_incompat_id = sqlx::query!(
        "INSERT INTO grouping_incompats (max_count) VALUES (?1)",
        max_count
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .last_insert_rowid();

    for grouping in &grouping_incompat.groupings {
        let _ = sqlx::query!(
            r#"
INSERT INTO grouping_incompat_items (grouping_incompat_id, grouping_id)
VALUES (?1, ?2)
            "#,
            grouping_incompat_id,
            grouping.0
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;
    }

    Ok(Id(grouping_incompat_id))
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), Error> {
    let grouping_incompat_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM grouping_incompat_items WHERE grouping_incompat_id = ?",
        grouping_incompat_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let count = sqlx::query!(
        "DELETE FROM grouping_incompats WHERE grouping_incompat_id = ?",
        grouping_incompat_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if count > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple grouping_incompat with id {:?}",
            index
        )));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    grouping_incompat: &GroupingIncompat<super::groupings::Id>,
) -> std::result::Result<(), Error> {
    let max_count = i64::try_from(grouping_incompat.max_count.get()).map_err(|_| {
        Error::RepresentationError(format!(
            "max_count (= {}) does not fit in i64",
            grouping_incompat.max_count.get()
        ))
    })?;

    let grouping_incompat_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM grouping_incompat_items WHERE grouping_incompat_id = ?",
        grouping_incompat_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let rows_affected = sqlx::query!(
        "UPDATE grouping_incompats SET max_count = ?1 WHERE grouping_incompat_id = ?2",
        max_count,
        grouping_incompat_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple grouping_incompats with id {:?}",
            index
        )));
    }

    for grouping in &grouping_incompat.groupings {
        let _ = sqlx::query!(
            r#"
INSERT INTO grouping_incompat_items (grouping_incompat_id, grouping_id)
VALUES (?1, ?2)
            "#,
            grouping_incompat_id,
            grouping.0
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;
    }

    Ok(())
}
