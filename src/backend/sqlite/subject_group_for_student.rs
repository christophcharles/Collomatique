use super::*;

async fn check_student_id(pool: &SqlitePool, student_id: students::Id) -> Result<bool> {
    let student_ids = sqlx::query!("SELECT student_id FROM students")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?
        .iter()
        .map(|x| x.student_id)
        .collect::<BTreeSet<_>>();

    Ok(student_ids.contains(&student_id.0))
}

async fn check_subject_group_id(
    pool: &SqlitePool,
    subject_group_id: subject_groups::Id,
) -> Result<bool> {
    let subject_group_ids = sqlx::query!("SELECT subject_group_id FROM subject_groups")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?
        .iter()
        .map(|x| x.subject_group_id)
        .collect::<BTreeSet<_>>();

    Ok(subject_group_ids.contains(&subject_group_id.0))
}

pub async fn set(
    pool: &SqlitePool,
    student_id: students::Id,
    subject_group_id: subject_groups::Id,
    subject_id: Option<subjects::Id>,
) -> std::result::Result<(), Error> {
    let _ = sqlx::query!(
        r#"
DELETE FROM student_subjects
WHERE student_id = ?1
AND subject_id IN (SELECT subject_id FROM subjects WHERE subject_group_id = ?2)
        "#,
        student_id.0,
        subject_group_id.0
    )
    .execute(pool)
    .await
    .map_err(Error::from)?;

    if let Some(id) = subject_id {
        let _ = sqlx::query!(
            "INSERT INTO student_subjects (student_id, subject_id) VALUES (?1, ?2)",
            student_id.0,
            id.0
        )
        .execute(pool)
        .await
        .map_err(Error::from)?;
    }

    Ok(())
}

pub async fn get(
    pool: &SqlitePool,
    student_id: students::Id,
    subject_group_id: subject_groups::Id,
) -> std::result::Result<Option<subjects::Id>, Id2Error<Error, students::Id, subject_groups::Id>> {
    if !check_student_id(pool, student_id).await? {
        return Err(Id2Error::InvalidId1(student_id));
    }
    if !check_subject_group_id(pool, subject_group_id).await? {
        return Err(Id2Error::InvalidId2(subject_group_id));
    }

    let records = sqlx::query!(
        r#"
SELECT student_subjects.subject_id AS subject_id
FROM student_subjects
JOIN subjects ON subjects.subject_id = student_subjects.subject_id
WHERE subjects.subject_group_id = ?1 AND student_subjects.student_id = ?2
        "#,
        subject_group_id.0,
        student_id.0
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    if records.len() > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "More than one subject for subject_group {} with student {}",
            subject_group_id.0, student_id.0
        )))?;
    }

    Ok(records
        .first()
        .map(|record| subjects::Id(record.subject_id)))
}
