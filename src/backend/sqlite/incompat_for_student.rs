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

async fn check_incompat_id(pool: &SqlitePool, incompat_id: incompats::Id) -> Result<bool> {
    let incompat_ids = sqlx::query!("SELECT incompat_id FROM incompats")
        .fetch_all(pool)
        .await
        .map_err(Error::from)?
        .iter()
        .map(|x| x.incompat_id)
        .collect::<BTreeSet<_>>();

    Ok(incompat_ids.contains(&incompat_id.0))
}

async fn enable(
    pool: &SqlitePool,
    student_id: students::Id,
    incompat_id: incompats::Id,
) -> std::result::Result<(), Error> {
    let _ = sqlx::query!(
        r#"
INSERT INTO student_incompats (student_id, incompat_id)
VALUES (?1, ?2)
        "#,
        student_id.0,
        incompat_id.0
    )
    .execute(pool)
    .await
    .map_err(Error::from)?;

    Ok(())
}

async fn disable(
    pool: &SqlitePool,
    student_id: students::Id,
    incompat_id: incompats::Id,
) -> std::result::Result<(), Error> {
    let rows_affected = sqlx::query!(
        r#"
DELETE FROM student_incompats
WHERE student_id = ?1 AND incompat_id = ?2
        "#,
        student_id.0,
        incompat_id.0
    )
    .execute(pool)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "more than one registration for student_incompat (student_id = {}, incomapt_id = {})",
            student_id.0, incompat_id.0
        )));
    }

    Ok(())
}

pub async fn set(
    pool: &SqlitePool,
    student_id: students::Id,
    incompat_id: incompats::Id,
    enabled: bool,
) -> std::result::Result<(), Error> {
    let current_value = get(pool, student_id, incompat_id)
        .await
        .map_err(|e| match e {
            Id2Error::InternalError(int_err) => int_err,
            _ => panic!("Unexpected invalid id: {:?}", e),
        })?;

    if current_value != enabled {
        if enabled {
            enable(pool, student_id, incompat_id).await?;
        } else {
            disable(pool, student_id, incompat_id).await?;
        }
    }

    Ok(())
}

pub async fn get(
    pool: &SqlitePool,
    student_id: students::Id,
    incompat_id: incompats::Id,
) -> std::result::Result<bool, Id2Error<Error, students::Id, incompats::Id>> {
    if !check_student_id(pool, student_id).await? {
        return Err(Id2Error::InvalidId1(student_id));
    }
    if !check_incompat_id(pool, incompat_id).await? {
        return Err(Id2Error::InvalidId2(incompat_id));
    }

    let record_opt = sqlx::query!(
        r#"
SELECT *
FROM student_incompats
WHERE student_id = ?1 AND incompat_id = ?2
        "#,
        student_id.0,
        incompat_id.0
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    Ok(record_opt.is_some())
}
