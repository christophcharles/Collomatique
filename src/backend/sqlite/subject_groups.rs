use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<SubjectGroup, IdError<Error, Id>> {
    let subject_group_id = index.0;

    let record_opt = sqlx::query!(
        "SELECT name, optional FROM subject_groups WHERE subject_group_id = ?",
        subject_group_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    Ok(SubjectGroup {
        name: record.name,
        optional: record.optional != 0,
    })
}

pub async fn get_all(pool: &SqlitePool) -> Result<BTreeMap<Id, SubjectGroup>> {
    let records = sqlx::query!("SELECT subject_group_id, name, optional FROM subject_groups",)
        .fetch_all(pool)
        .await?;

    Ok(records
        .into_iter()
        .map(|record| {
            (
                Id(record.subject_group_id),
                SubjectGroup {
                    name: record.name,
                    optional: record.optional != 0,
                },
            )
        })
        .collect())
}

pub async fn add(pool: &SqlitePool, subject_group: SubjectGroup) -> Result<Id> {
    let mut conn = pool.acquire().await?;

    let optional = if subject_group.optional { 1 } else { 0 };

    let id = sqlx::query!(
        "INSERT INTO subject_groups (name, optional) VALUES (?1, ?2)",
        subject_group.name,
        optional,
    )
    .execute(&mut *conn)
    .await?
    .last_insert_rowid();

    let subject_group_id = Id(id);

    Ok(subject_group_id)
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    let subject_group_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let count = sqlx::query!(
        "DELETE FROM subject_groups WHERE subject_group_id = ?",
        subject_group_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if count == 0 {
        return Err(IdError::InvalidId(index));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    subject_group: SubjectGroup,
) -> std::result::Result<(), IdError<Error, Id>> {
    let subject_group_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let optional = if subject_group.optional { 1 } else { 0 };
    let rows_affected = sqlx::query!(
        "UPDATE subject_groups SET name = ?1, optional = ?2 WHERE subject_group_id = ?3",
        subject_group.name,
        optional,
        subject_group_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected != 1 {
        return Err(IdError::InvalidId(index));
    }

    Ok(())
}
