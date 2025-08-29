use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    pool: &SqlitePool,
) -> std::result::Result<
    BTreeMap<Id, SlotSelection<super::subjects::Id, super::time_slots::Id>>,
    Error,
> {
    let records = sqlx::query!("SELECT slot_selection_id, subject_id FROM slot_selections")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

    let mut output = BTreeMap::new();

    for record in records {
        let slot_group_list = sqlx::query!(
            "SELECT slot_group_id, count FROM slot_groups WHERE slot_selection_id = ?",
            record.slot_selection_id
        )
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

        let mut slot_groups = Vec::new();
        for slot_group in slot_group_list {
            let slot_group_items = sqlx::query!(
                "SELECT time_slot_id FROM slot_group_items WHERE slot_group_id = ?",
                slot_group.slot_group_id
            )
            .fetch_all(pool)
            .await
            .map_err(Error::from)?;

            let count_usize = usize::try_from(slot_group.count).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "Invalid count ({}) in slot_group {} that does not fit in usize",
                    slot_group.count, slot_group.slot_group_id
                ))
            })?;

            let new_slot_group = SlotGroup {
                count: count_usize,
                slots: slot_group_items
                    .into_iter()
                    .map(|x| super::time_slots::Id(x.time_slot_id))
                    .collect(),
            };

            slot_groups.push(new_slot_group);
        }

        let slot_selection = SlotSelection {
            subject_id: super::subjects::Id(record.subject_id),
            slot_groups,
        };

        output.insert(Id(record.slot_selection_id), slot_selection);
    }

    Ok(output)
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<
    SlotSelection<super::subjects::Id, super::time_slots::Id>,
    IdError<Error, Id>,
> {
    let slot_selection_id = index.0;

    let subject_id = sqlx::query!(
        "SELECT subject_id FROM slot_selections WHERE slot_selection_id = ?",
        slot_selection_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?
    .ok_or(IdError::InvalidId(index))?
    .subject_id;

    let mut output = SlotSelection {
        subject_id: super::subjects::Id(subject_id),
        slot_groups: Vec::new(),
    };

    let slot_group_list = sqlx::query!(
        "SELECT slot_group_id, count FROM slot_groups WHERE slot_selection_id = ?",
        slot_selection_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    for slot_group in slot_group_list {
        let slot_group_items = sqlx::query!(
            "SELECT time_slot_id FROM slot_group_items WHERE slot_group_id = ?",
            slot_group.slot_group_id
        )
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

        let count_usize = usize::try_from(slot_group.count).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "Invalid count ({}) in slot_group {} that does not fit in usize",
                slot_group.count, slot_group.slot_group_id
            ))
        })?;

        let new_slot_group = SlotGroup {
            count: count_usize,
            slots: slot_group_items
                .into_iter()
                .map(|x| super::time_slots::Id(x.time_slot_id))
                .collect(),
        };

        output.slot_groups.push(new_slot_group);
    }

    Ok(output)
}

pub async fn add(
    pool: &SqlitePool,
    slot_selection: &SlotSelection<super::subjects::Id, super::time_slots::Id>,
) -> std::result::Result<Id, Error> {
    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let slot_selection_id = sqlx::query!(
        "INSERT INTO slot_selections (subject_id) VALUES (?)",
        slot_selection.subject_id.0
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .last_insert_rowid();

    for slot_group in &slot_selection.slot_groups {
        let count_i64 = i64::try_from(slot_group.count).map_err(|_| {
            Error::RepresentationError(format!(
                "Cannot represent usize count ({}) into i64 for the database",
                slot_group.count,
            ))
        })?;
        let slot_group_id = sqlx::query!(
            "INSERT INTO slot_groups (slot_selection_id, count) VALUES (?1, ?2)",
            slot_selection_id,
            count_i64,
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

        for time_slot_id in &slot_group.slots {
            sqlx::query!(
                "INSERT INTO slot_group_items (slot_group_id, time_slot_id) VALUES (?1, ?2)",
                slot_group_id,
                time_slot_id.0,
            )
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?;
        }
    }

    Ok(Id(slot_selection_id))
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), Error> {
    let slot_selection_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        r#"
DELETE FROM slot_group_items
WHERE slot_group_id IN
(
    SELECT slot_group_id FROM slot_groups
    WHERE slot_selection_id = ?
);

DELETE FROM slot_groups
WHERE slot_selection_id = ?;
        "#,
        slot_selection_id,
        slot_selection_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let count = sqlx::query!(
        "DELETE FROM slot_selections WHERE slot_selection_id = ?",
        slot_selection_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if count > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple slot_selection with id {:?}",
            index
        )));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    slot_selection: &SlotSelection<super::subjects::Id, super::time_slots::Id>,
) -> std::result::Result<(), Error> {
    let slot_selection_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let rows_affected = sqlx::query!(
        "UPDATE slot_selections SET subject_id = ?1 WHERE slot_selection_id = ?2",
        slot_selection.subject_id.0,
        slot_selection_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple slot_selection with id {:?}",
            index
        )));
    }

    let _ = sqlx::query!(
        r#"
DELETE FROM slot_group_items
WHERE slot_group_id IN
(
    SELECT slot_group_id FROM slot_groups
    WHERE slot_selection_id = ?
);

DELETE FROM slot_groups
WHERE slot_selection_id = ?;
        "#,
        slot_selection_id,
        slot_selection_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    for slot_group in &slot_selection.slot_groups {
        let count_i64 = i64::try_from(slot_group.count).map_err(|_| {
            Error::RepresentationError(format!(
                "Cannot represent usize count ({}) into i64 for the database",
                slot_group.count,
            ))
        })?;
        let slot_group_id = sqlx::query!(
            "INSERT INTO slot_groups (slot_selection_id, count) VALUES (?1, ?2)",
            slot_selection_id,
            count_i64,
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

        for time_slot_id in &slot_group.slots {
            sqlx::query!(
                "INSERT INTO slot_group_items (slot_group_id, time_slot_id) VALUES (?1, ?2)",
                slot_group_id,
                time_slot_id.0,
            )
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?;
        }
    }

    Ok(())
}
