use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    pool: &SqlitePool,
) -> std::result::Result<BTreeMap<Id, Grouping<super::time_slots::Id>>, Error> {
    let records = sqlx::query!("SELECT grouping_id, name FROM groupings")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

    let data = sqlx::query!("SELECT grouping_id, time_slot_id FROM grouping_items")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

    let output = records
        .into_iter()
        .map(|record| {
            (
                Id(record.grouping_id),
                Grouping {
                    name: record.name,
                    slots: data
                        .iter()
                        .filter_map(|x| {
                            if x.grouping_id != record.grouping_id {
                                return None;
                            }
                            Some(time_slots::Id(x.time_slot_id))
                        })
                        .collect(),
                },
            )
        })
        .collect();

    Ok(output)
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<Grouping<super::time_slots::Id>, IdError<Error, Id>> {
    let grouping_id = index.0;

    let record_opt = sqlx::query!(
        "SELECT name FROM groupings WHERE grouping_id = ?",
        grouping_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    let data = sqlx::query!(
        "SELECT time_slot_id FROM grouping_items WHERE grouping_id = ?",
        grouping_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    Ok(Grouping {
        name: record.name,
        slots: data
            .into_iter()
            .map(|x| time_slots::Id(x.time_slot_id))
            .collect(),
    })
}

async fn search_invalid_time_slot_id(
    pool: &SqlitePool,
    grouping: &Grouping<time_slots::Id>,
) -> Result<Option<time_slots::Id>> {
    let time_slot_ids = sqlx::query!("SELECT time_slot_id FROM time_slots",)
        .fetch_all(pool)
        .await
        .map_err(Error::from)?
        .iter()
        .map(|x| x.time_slot_id)
        .collect::<BTreeSet<_>>();

    for slot in &grouping.slots {
        if !time_slot_ids.contains(&slot.0) {
            return Ok(Some(slot.clone()));
        }
    }

    Ok(None)
}

pub async fn add(
    pool: &SqlitePool,
    grouping: &Grouping<super::time_slots::Id>,
) -> std::result::Result<Id, CrossError<Error, super::time_slots::Id>> {
    if let Some(time_slot_id) = search_invalid_time_slot_id(pool, grouping).await? {
        return Err(CrossError::InvalidCrossId(time_slot_id));
    }

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let grouping_id = sqlx::query!("INSERT INTO groupings (name) VALUES (?1)", grouping.name)
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

    for slot in &grouping.slots {
        let _ = sqlx::query!(
            r#"
INSERT INTO grouping_items (grouping_id, time_slot_id)
VALUES (?1, ?2)
            "#,
            grouping_id,
            slot.0
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;
    }

    Ok(Id(grouping_id))
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    let grouping_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM grouping_items WHERE grouping_id = ?",
        grouping_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let count = sqlx::query!("DELETE FROM groupings WHERE grouping_id = ?", grouping_id)
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .rows_affected();

    if count > 1 {
        return Err(IdError::InternalError(Error::CorruptedDatabase(format!(
            "Multiple groupings with id {:?}",
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
    grouping: &Grouping<super::time_slots::Id>,
) -> std::result::Result<(), CrossIdError<Error, Id, super::time_slots::Id>> {
    if let Some(time_slot_id) = search_invalid_time_slot_id(pool, grouping).await? {
        return Err(CrossIdError::InvalidCrossId(time_slot_id));
    }

    let grouping_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM grouping_items WHERE grouping_id = ?",
        grouping_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let grouping_id = sqlx::query!(
        "UPDATE groupings SET name = ?1 WHERE grouping_id = ?2",
        grouping.name,
        grouping_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .last_insert_rowid();

    for slot in &grouping.slots {
        let _ = sqlx::query!(
            r#"
INSERT INTO grouping_items (grouping_id, time_slot_id)
VALUES (?1, ?2)
            "#,
            grouping_id,
            slot.0
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;
    }

    Ok(())
}
