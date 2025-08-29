use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(
    pool: &SqlitePool,
) -> std::result::Result<
    BTreeMap<Id, Colloscope<super::teachers::Id, super::subjects::Id, super::students::Id>>,
    Error,
> {
    let colloscope_ids = sqlx::query!("SELECT colloscope_id FROM colloscopes")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

    let mut output = BTreeMap::new();

    for record in colloscope_ids {
        let colloscope = get(pool, Id(record.colloscope_id)).await.map_err(
            |e| match e {
                IdError::InternalError(int_err) => int_err,
                IdError::InvalidId(id) => panic!("Colloscope id {} is apparently invalid but it was extracted directly from the db", id.0),
            }
        )?;

        output.insert(Id(record.colloscope_id), colloscope);
    }

    Ok(output)
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<
    Colloscope<super::teachers::Id, super::subjects::Id, super::students::Id>,
    IdError<Error, Id>,
> {
    let colloscope_id = index.0;

    let colloscope_name = sqlx::query!(
        "SELECT name FROM colloscopes WHERE colloscope_id = ?",
        colloscope_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?
    .ok_or(IdError::InvalidId(index))?
    .name;

    let mut output = Colloscope {
        name: colloscope_name,
        subjects: BTreeMap::new(),
    };

    let subject_list = sqlx::query!(
        "SELECT collo_subject_id, subject_id, group_list_name FROM collo_subjects WHERE colloscope_id = ?",
        colloscope_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    for subject in subject_list {
        let group_list_records = sqlx::query!(
            "SELECT collo_group_id, name FROM collo_groups WHERE collo_subject_id = ?",
            subject.collo_subject_id
        )
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

        let mut group_list = ColloscopeGroupList {
            name: subject.group_list_name,
            groups: Vec::new(),
            students_mapping: BTreeMap::new(),
        };

        let mut group_map = BTreeMap::new();

        for group in group_list_records {
            let group_num = group_list.groups.len();
            group_list.groups.push(group.name);

            group_map.insert(group.collo_group_id, group_num);

            let group_items = sqlx::query!(
                "SELECT student_id FROM collo_group_items WHERE collo_group_id = ?",
                group.collo_group_id
            )
            .fetch_all(pool)
            .await
            .map_err(Error::from)?;

            for group_item in group_items {
                if group_list
                    .students_mapping
                    .insert(super::students::Id(group_item.student_id), group_num)
                    .is_some()
                {
                    return Err(Error::CorruptedDatabase(
                        format!(
                            "Student {} is present in two different groups for subject {} in colloscope {}",
                            group_item.student_id,
                            subject.subject_id,
                            colloscope_id,
                        )
                    ).into());
                }
            }
        }

        let time_slot_records = sqlx::query!(
            "SELECT collo_time_slot_id, teacher_id, start_day, start_time, room FROM collo_time_slots WHERE collo_subject_id = ?",
            subject.collo_subject_id
        )
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

        let mut time_slots = Vec::new();

        for time_slot in time_slot_records {
            let start_day_usize = usize::try_from(time_slot.start_day).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "Database references invalid start day ({}) for collo_time_slot_id {}",
                    time_slot.start_day, time_slot.collo_time_slot_id
                ))
            })?;
            let day = crate::time::Weekday::try_from(start_day_usize).map_err(|e| {
                Error::CorruptedDatabase(format!(
                    "Database references invalid start day ({}) for collo_time_slot_id {}: {}",
                    start_day_usize, time_slot.collo_time_slot_id, e
                ))
            })?;
            let start_time_u32 = u32::try_from(time_slot.start_time).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "Database references invalid start time ({}) for collo_time_slot_id {}",
                    time_slot.start_time, time_slot.collo_time_slot_id
                ))
            })?;
            let time =
                crate::time::Time::new(start_time_u32).ok_or(Error::CorruptedDatabase(format!(
                    "Database references invalid start time ({}) for collo_time_slot_id {}",
                    start_time_u32, time_slot.collo_time_slot_id
                )))?;
            let start = SlotStart { day, time };

            let mut new_time_slot = ColloscopeTimeSlot {
                teacher_id: super::teachers::Id(time_slot.teacher_id),
                start,
                room: time_slot.room,
                group_assignments: BTreeMap::new(),
            };

            let week_records = sqlx::query!(
                "SELECT collo_week_id, week FROM collo_weeks WHERE collo_time_slot_id = ?",
                time_slot.collo_time_slot_id
            )
            .fetch_all(pool)
            .await
            .map_err(Error::from)?;

            for week in week_records {
                let week_item_records = sqlx::query!(
                    "SELECT collo_group_id FROM collo_week_items WHERE collo_week_id = ?",
                    week.collo_week_id
                )
                .fetch_all(pool)
                .await
                .map_err(Error::from)?;

                let group_assignments = week_item_records
                    .into_iter()
                    .map(|x| {
                        group_map
                            .get(&x.collo_group_id)
                            .copied()
                            .ok_or(Error::CorruptedDatabase(format!(
                            "Database references invalid collo_group_id {} for collo_week_id {}",
                            x.collo_group_id,
                            week.collo_week_id,
                        )))
                    })
                    .collect::<Result<Vec<_>>>()?;

                let week_num = u32::try_from(week.week).map_err(|_| {
                    Error::CorruptedDatabase(format!(
                        "Database references invalid u32 week ({}) for collo_week_id {}",
                        week.week, week.collo_week_id
                    ))
                })?;

                if new_time_slot
                    .group_assignments
                    .insert(Week(week_num), group_assignments)
                    .is_some()
                {
                    return Err(Error::CorruptedDatabase(format!(
                        "Database references week {} multiple times for collo_time_slot_id {}",
                        week_num, time_slot.collo_time_slot_id,
                    ))
                    .into());
                }
            }

            time_slots.push(new_time_slot);
        }

        let new_subject = ColloscopeSubject {
            time_slots,
            group_list,
        };

        if output
            .subjects
            .insert(super::subjects::Id(subject.subject_id), new_subject)
            .is_some()
        {
            return Err(Error::CorruptedDatabase(format!(
                "Multiple occurences of subject {} in colloscope {}",
                subject.subject_id, colloscope_id,
            ))
            .into());
        }
    }

    Ok(output)
}

pub async fn add(
    pool: &SqlitePool,
    colloscope: &Colloscope<super::teachers::Id, super::subjects::Id, super::students::Id>,
) -> std::result::Result<Id, Error> {
    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let colloscope_id = sqlx::query!("INSERT INTO colloscopes (name) VALUES (?)", colloscope.name,)
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

    for (subject_id, subject) in &colloscope.subjects {
        let collo_subject_id = sqlx::query!(
            "INSERT INTO collo_subjects (colloscope_id, subject_id, group_list_name) VALUES (?1, ?2, ?3)",
            colloscope_id,
            subject_id.0,
            subject.group_list.name,
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

        let mut group_map = Vec::new();

        for group in &subject.group_list.groups {
            let collo_group_id = sqlx::query!(
                "INSERT INTO collo_groups (collo_subject_id, name) VALUES (?1, ?2)",
                collo_subject_id,
                *group,
            )
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?
            .last_insert_rowid();

            group_map.push(collo_group_id);
        }

        for (student, student_group) in &subject.group_list.students_mapping {
            let _ = sqlx::query!(
                "INSERT INTO collo_group_items (collo_group_id, student_id) VALUES (?1, ?2)",
                group_map[*student_group],
                student.0,
            )
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?;
        }

        for time_slot in &subject.time_slots {
            let start_day: i64 = usize::from(time_slot.start.day)
                .try_into()
                .expect("day number should fit in i64");
            let start_time = time_slot.start.time.get();

            let collo_time_slot_id = sqlx::query!(
                "INSERT INTO collo_time_slots (collo_subject_id, teacher_id, start_day, start_time, room) VALUES (?1, ?2, ?3, ?4, ?5)",
                collo_subject_id,
                time_slot.teacher_id.0,
                start_day,
                start_time,
                time_slot.room,
            )
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?
            .last_insert_rowid();

            for (week, groups) in &time_slot.group_assignments {
                let collo_week_id = sqlx::query!(
                    "INSERT INTO collo_weeks (collo_time_slot_id, week) VALUES (?1, ?2)",
                    collo_time_slot_id,
                    week.0,
                )
                .execute(&mut *conn)
                .await
                .map_err(Error::from)?
                .last_insert_rowid();

                for group in groups {
                    let _ = sqlx::query!(
                        "INSERT INTO collo_week_items (collo_week_id, collo_group_id) VALUES (?1, ?2)",
                        collo_week_id,
                        group_map[*group],
                    )
                    .execute(&mut *conn)
                    .await
                    .map_err(Error::from)?;
                }
            }
        }
    }

    Ok(Id(colloscope_id))
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), Error> {
    let colloscope_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        r#"
DELETE FROM collo_week_items
WHERE collo_week_id IN
(
    SELECT collo_week_id FROM collo_weeks
    WHERE collo_time_slot_id IN
    (
        SELECT collo_time_slot_id FROM collo_time_slots
        WHERE collo_subject_id IN
        (
            SELECT collo_subject_id FROM collo_subjects WHERE colloscope_id = ?
        )
    )
);

DELETE FROM collo_weeks
WHERE collo_time_slot_id IN
(
    SELECT collo_time_slot_id FROM collo_time_slots
    WHERE collo_subject_id IN
    (
        SELECT collo_subject_id FROM collo_subjects WHERE colloscope_id = ?
    )
);

DELETE FROM collo_time_slots
WHERE collo_subject_id IN
(
    SELECT collo_subject_id FROM collo_subjects WHERE colloscope_id = ?
);

DELETE FROM collo_group_items
WHERE collo_group_id IN
(
    SELECT collo_group_id FROM collo_groups
    WHERE collo_subject_id IN
    (
        SELECT collo_subject_id FROM collo_subjects WHERE colloscope_id = ?
    )
);

DELETE FROM collo_groups
WHERE collo_subject_id IN
(
    SELECT collo_subject_id FROM collo_subjects WHERE colloscope_id = ?
);

DELETE FROM collo_subjects WHERE colloscope_id = ?;
        "#,
        colloscope_id,
        colloscope_id,
        colloscope_id,
        colloscope_id,
        colloscope_id,
        colloscope_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let count = sqlx::query!(
        "DELETE FROM colloscopes WHERE colloscope_id = ?",
        colloscope_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if count > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple colloscopes with id {:?}",
            index
        )));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    colloscope: &Colloscope<super::teachers::Id, super::subjects::Id, super::students::Id>,
) -> std::result::Result<(), Error> {
    todo!()
}
