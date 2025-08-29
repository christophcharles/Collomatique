use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        Ok(())
    }
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<Incompat<week_patterns::Id>, IdError<Error, Id>> {
    let incompat_id = index.0;

    let record_opt = sqlx::query!(
        "SELECT name, max_count FROM incompats WHERE incompat_id = ?",
        incompat_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    let data = sqlx::query!(
        r#"
SELECT incompat_group_items.incompat_group_id AS incompat_group_id, week_pattern_id, start_day, start_time, duration
FROM incompat_groups JOIN incompat_group_items
ON incompat_groups.incompat_group_id = incompat_group_items.incompat_group_id
WHERE incompat_id = ?
        "#,
        incompat_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    let mut temp_groups: BTreeMap<i64, IncompatGroup<week_patterns::Id>> = BTreeMap::new();

    for x in data {
        let week_pattern_id = week_patterns::Id(x.week_pattern_id);

        let start_day_usize = usize::try_from(x.start_day).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "Database references invalid start day ({}) for incompat_group_id {}",
                x.start_day, x.incompat_group_id
            ))
        })?;
        let day = crate::time::Weekday::try_from(start_day_usize).map_err(|e| {
            Error::CorruptedDatabase(format!(
                "Database references invalid start day ({}) for incompat_group_id {}: {}",
                start_day_usize, x.incompat_group_id, e
            ))
        })?;
        let start_time_u32 = u32::try_from(x.start_time).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "Database references invalid start time ({}) for incompat_group_id {}",
                x.start_time, x.incompat_group_id
            ))
        })?;
        let time =
            crate::time::Time::new(start_time_u32).ok_or(Error::CorruptedDatabase(format!(
                "Database references invalid start time ({}) for incompat_group_id {}",
                start_time_u32, x.incompat_group_id
            )))?;
        let start = SlotStart { day, time };

        let duration_u32 = u32::try_from(x.duration).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "Database references invalid duration ({}) for incompat_group_id {}",
                x.duration, x.incompat_group_id
            ))
        })?;
        let duration = NonZeroU32::new(duration_u32).ok_or(Error::CorruptedDatabase(format!(
            "Database references invalid duration ({}) for incompat_group_id {}",
            duration_u32, x.incompat_group_id
        )))?;

        let slot = IncompatSlot {
            week_pattern_id,
            start,
            duration,
        };
        match temp_groups.get_mut(&x.incompat_group_id) {
            Some(group) => {
                group.slots.insert(slot);
            }
            None => {
                temp_groups.insert(
                    x.incompat_group_id,
                    IncompatGroup {
                        slots: BTreeSet::from([slot]),
                    },
                );
            }
        }
    }

    let groups = temp_groups.into_iter().map(|(_key, group)| group).collect();

    Ok(Incompat {
        name: record.name,
        max_count: usize::try_from(record.max_count).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "Database has invalid max_count ({}) for incompat_id {}",
                record.max_count, incompat_id
            ))
        })?,
        groups,
    })
}

pub async fn get_all(pool: &SqlitePool) -> Result<BTreeMap<Id, Incompat<week_patterns::Id>>> {
    let incompats_db = sqlx::query!("SELECT incompat_id, name, max_count FROM incompats")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

    let mut incompats = incompats_db
        .into_iter()
        .map(|x| {
            let max_count_usize = usize::try_from(x.max_count).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "Database has invalid max_count ({}) for incompat_id {}",
                    x.max_count, x.incompat_id
                ))
            })?;

            Result::Ok((
                Id(x.incompat_id),
                Incompat {
                    name: x.name,
                    max_count: max_count_usize,
                    groups: BTreeSet::<IncompatGroup<week_patterns::Id>>::new(),
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    for (id, incompat) in incompats.iter_mut() {
        let incompat_id = id.0;

        let data = sqlx::query!(
            r#"
SELECT incompat_group_items.incompat_group_id AS incompat_group_id, week_pattern_id, start_day, start_time, duration
FROM incompat_groups JOIN incompat_group_items
ON incompat_groups.incompat_group_id = incompat_group_items.incompat_group_id
WHERE incompat_id = ?
            "#,
            incompat_id
        )
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

        let mut temp_groups: BTreeMap<i64, IncompatGroup<week_patterns::Id>> = BTreeMap::new();

        for x in data {
            let week_pattern_id = week_patterns::Id(x.week_pattern_id);

            let start_day_usize = usize::try_from(x.start_day).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "Database references invalid start day ({}) for incompat_group_id {}",
                    x.start_day, x.incompat_group_id
                ))
            })?;
            let day = crate::time::Weekday::try_from(start_day_usize).map_err(|e| {
                Error::CorruptedDatabase(format!(
                    "Database references invalid start day ({}) for incompat_group_id {}: {}",
                    start_day_usize, x.incompat_group_id, e
                ))
            })?;
            let start_time_u32 = u32::try_from(x.start_time).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "Database references invalid start time ({}) for incompat_group_id {}",
                    x.start_time, x.incompat_group_id
                ))
            })?;
            let time =
                crate::time::Time::new(start_time_u32).ok_or(Error::CorruptedDatabase(format!(
                    "Database references invalid start time ({}) for incompat_group_id {}",
                    start_time_u32, x.incompat_group_id
                )))?;
            let start = SlotStart { day, time };

            let duration_u32 = u32::try_from(x.duration).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "Database references invalid duration ({}) for incompat_group_id {}",
                    x.duration, x.incompat_group_id
                ))
            })?;
            let duration =
                NonZeroU32::new(duration_u32).ok_or(Error::CorruptedDatabase(format!(
                    "Database references invalid duration ({}) for incompat_group_id {}",
                    duration_u32, x.incompat_group_id
                )))?;

            let slot = IncompatSlot {
                week_pattern_id,
                start,
                duration,
            };
            match temp_groups.get_mut(&x.incompat_group_id) {
                Some(group) => {
                    group.slots.insert(slot);
                }
                None => {
                    temp_groups.insert(
                        x.incompat_group_id,
                        IncompatGroup {
                            slots: BTreeSet::from([slot]),
                        },
                    );
                }
            }
        }

        incompat.groups = temp_groups.into_iter().map(|(_key, group)| group).collect();
    }

    Ok(incompats)
}

pub async fn add(
    pool: &SqlitePool,
    incompat: &Incompat<week_patterns::Id>,
) -> std::result::Result<Id, Error> {
    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let max_count_i64 = i64::try_from(incompat.max_count).map_err(|_| {
        Error::RepresentationError(format!(
            "Cannot represent max_count (value: {}) as an i64 for the database",
            incompat.max_count
        ))
    })?;

    let incompat_id = sqlx::query!(
        "INSERT INTO incompats (name, max_count) VALUES (?1, ?2)",
        incompat.name,
        max_count_i64,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .last_insert_rowid();

    for incompat_group in &incompat.groups {
        let incompat_group_id = sqlx::query!(
            "INSERT INTO incompat_groups (incompat_id) VALUES (?)",
            incompat_id
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

        for slot in &incompat_group.slots {
            let start_day: i64 = usize::from(slot.start.day)
                .try_into()
                .expect("day number should fit in i64");
            let start_time = slot.start.time.get();
            let duration = slot.duration.get();

            let _ = sqlx::query!(
                r#"
INSERT INTO incompat_group_items (incompat_group_id, week_pattern_id, start_day, start_time, duration)
VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                incompat_group_id,
                slot.week_pattern_id.0,
                start_day,
                start_time,
                duration
            )
            .execute(&mut *conn)
            .await.map_err(Error::from)?;
        }
    }

    Ok(Id(incompat_id))
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), Error> {
    let incompat_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        r#"
DELETE FROM incompat_group_items
WHERE incompat_group_id IN
(SELECT incompat_group_id FROM incompat_groups WHERE incompat_id = ?)
        "#,
        incompat_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM incompat_groups WHERE incompat_id = ?",
        incompat_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let count = sqlx::query!("DELETE FROM incompats WHERE incompat_id = ?", incompat_id)
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .rows_affected();

    if count > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple incompats with id {:?}",
            index
        )));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    incompat: &Incompat<week_patterns::Id>,
) -> std::result::Result<(), Error> {
    let incompat_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let max_count_i64 = i64::try_from(incompat.max_count).map_err(|_| {
        Error::RepresentationError(format!(
            "Cannot represent max_count (value: {}) as an i64 for the database",
            incompat.max_count
        ))
    })?;

    let rows_affected = sqlx::query!(
        "UPDATE incompats SET name = ?1, max_count = ?2 WHERE incompat_id = ?3",
        incompat.name,
        max_count_i64,
        incompat_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple incompats with id {:?}",
            index
        )));
    }

    let _ = sqlx::query!(
        r#"
DELETE FROM incompat_group_items
WHERE incompat_group_id IN
(SELECT incompat_group_id FROM incompat_groups WHERE incompat_id = ?)
        "#,
        incompat_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM incompat_groups WHERE incompat_id = ?",
        incompat_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    for incompat_group in &incompat.groups {
        let incompat_group_id = sqlx::query!(
            "INSERT INTO incompat_groups (incompat_id) VALUES (?)",
            incompat_id
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

        for slot in &incompat_group.slots {
            let start_day: i64 = usize::from(slot.start.day)
                .try_into()
                .expect("day number should fit in i64");
            let start_time = slot.start.time.get();
            let duration = slot.duration.get();

            let _ = sqlx::query!(
                r#"
INSERT INTO incompat_group_items (incompat_group_id, week_pattern_id, start_day, start_time, duration)
VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                incompat_group_id,
                slot.week_pattern_id.0,
                start_day,
                start_time,
                duration
            )
            .execute(&mut *conn)
            .await.map_err(Error::from)?;
        }
    }

    Ok(())
}
