use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

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
        let start = TimeSlot { day, time };

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

pub async fn get_all(_pool: &SqlitePool) -> Result<BTreeMap<Id, Incompat<week_patterns::Id>>> {
    todo!()
}

pub async fn add(
    _pool: &SqlitePool,
    _incompat: Incompat<week_patterns::Id>,
) -> std::result::Result<Id, CrossError<Error, week_patterns::Id>> {
    todo!()
}

pub async fn remove(_pool: &SqlitePool, _index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    todo!()
}

pub async fn update(
    _pool: &SqlitePool,
    _index: Id,
    _incompat: Incompat<week_patterns::Id>,
) -> std::result::Result<(), CrossIdError<Error, Id, week_patterns::Id>> {
    todo!()
}
