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
) -> std::result::Result<
    BTreeMap<Id, TimeSlot<super::subjects::Id, super::teachers::Id, super::week_patterns::Id>>,
    Error,
> {
    let records = sqlx::query!(
        r#"
SELECT time_slot_id, subject_id, teacher_id, start_day, start_time, week_pattern_id, room
FROM time_slots
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    let mut output = BTreeMap::new();

    for record in records {
        let start_day_usize = usize::try_from(record.start_day).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "Database references invalid start day ({}) for time_slot_id {}",
                record.start_day, record.time_slot_id
            ))
        })?;
        let day = crate::time::Weekday::try_from(start_day_usize).map_err(|e| {
            Error::CorruptedDatabase(format!(
                "Database references invalid start day ({}) for time_slot_id {}: {}",
                start_day_usize, record.time_slot_id, e
            ))
        })?;
        let start_time_u32 = u32::try_from(record.start_time).map_err(|_| {
            Error::CorruptedDatabase(format!(
                "Database references invalid start time ({}) for time_slot_id {}",
                record.start_time, record.time_slot_id
            ))
        })?;
        let time =
            crate::time::Time::new(start_time_u32).ok_or(Error::CorruptedDatabase(format!(
                "Database references invalid start time ({}) for time_slot_id {}",
                start_time_u32, record.time_slot_id
            )))?;
        let start = SlotStart { day, time };

        output.insert(
            Id(record.time_slot_id),
            TimeSlot {
                subject_id: subjects::Id(record.subject_id),
                teacher_id: teachers::Id(record.teacher_id),
                start,
                week_pattern_id: week_patterns::Id(record.week_pattern_id),
                room: record.room,
            },
        );
    }

    Ok(output)
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<
    TimeSlot<super::subjects::Id, super::teachers::Id, super::week_patterns::Id>,
    IdError<Error, Id>,
> {
    let record_opt = sqlx::query!(
        r#"
SELECT subject_id, teacher_id, start_day, start_time, week_pattern_id, room
FROM time_slots WHERE time_slot_id = ?
        "#,
        index.0
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    let start_day_usize = usize::try_from(record.start_day).map_err(|_| {
        Error::CorruptedDatabase(format!(
            "Database references invalid start day ({}) for time_slot_id {}",
            record.start_day, index.0
        ))
    })?;
    let day = crate::time::Weekday::try_from(start_day_usize).map_err(|e| {
        Error::CorruptedDatabase(format!(
            "Database references invalid start day ({}) for time_slot_id {}: {}",
            start_day_usize, index.0, e
        ))
    })?;
    let start_time_u32 = u32::try_from(record.start_time).map_err(|_| {
        Error::CorruptedDatabase(format!(
            "Database references invalid start time ({}) for time_slot_id {}",
            record.start_time, index.0
        ))
    })?;
    let time = crate::time::Time::new(start_time_u32).ok_or(Error::CorruptedDatabase(format!(
        "Database references invalid start time ({}) for time_slot_id {}",
        start_time_u32, index.0
    )))?;
    let start = SlotStart { day, time };

    let output = TimeSlot {
        subject_id: subjects::Id(record.subject_id),
        teacher_id: teachers::Id(record.teacher_id),
        start,
        week_pattern_id: week_patterns::Id(record.week_pattern_id),
        room: record.room,
    };

    Ok(output)
}

pub async fn add(
    pool: &SqlitePool,
    time_slot: &TimeSlot<super::subjects::Id, super::teachers::Id, super::week_patterns::Id>,
) -> std::result::Result<Id, Error> {
    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let start_day: i64 = usize::from(time_slot.start.day)
        .try_into()
        .expect("day number should fit in i64");
    let start_time = time_slot.start.time.get();

    let time_slot_id = sqlx::query!(
        r#"
INSERT INTO time_slots
(subject_id, teacher_id, start_day, start_time, week_pattern_id, room)
VALUES (?1, ?2, ?3, ?4, ?5, ?6);
        "#,
        time_slot.subject_id.0,
        time_slot.teacher_id.0,
        start_day,
        start_time,
        time_slot.week_pattern_id.0,
        time_slot.room,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .last_insert_rowid();

    Ok(Id(time_slot_id))
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), Error> {
    let time_slot_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let count = sqlx::query!(
        "DELETE FROM time_slots WHERE time_slot_id = ?",
        time_slot_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if count > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple time_slots with id {:?}",
            index
        )));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    time_slot: &TimeSlot<super::subjects::Id, super::teachers::Id, super::week_patterns::Id>,
) -> std::result::Result<(), Error> {
    let time_slot_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let start_day: i64 = usize::from(time_slot.start.day)
        .try_into()
        .expect("day number should fit in i64");
    let start_time = time_slot.start.time.get();

    let rows_affected = sqlx::query!(
        r#"
UPDATE time_slots
SET subject_id = ?1, teacher_id = ?2, start_day = ?3, start_time = ?4, week_pattern_id = ?5, room = ?6
WHERE time_slot_id = ?7
        "#,
        time_slot.subject_id.0,
        time_slot.teacher_id.0,
        start_day,
        start_time,
        time_slot.week_pattern_id.0,
        time_slot.room,
        time_slot_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple time_slots with id {:?}",
            index
        )));
    }

    Ok(())
}
