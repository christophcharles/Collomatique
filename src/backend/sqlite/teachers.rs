use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get(pool: &SqlitePool, index: Id) -> std::result::Result<Teacher, IdError<Error, Id>> {
    let teacher_id = index.0;

    let record_opt = sqlx::query_as!(
        Teacher,
        "SELECT surname, firstname, contact FROM teachers WHERE teacher_id = ?",
        teacher_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    Ok(record)
}

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Teacher>> {
    let records = sqlx::query_as!(Teacher, "SELECT surname, firstname, contact FROM teachers",)
        .fetch_all(pool)
        .await?;

    Ok(records)
}

pub async fn add(pool: &SqlitePool, teacher: Teacher) -> Result<Id> {
    let mut conn = pool.acquire().await?;

    let id = sqlx::query!(
        "INSERT INTO teachers (surname, firstname, contact) VALUES (?1, ?2, ?3)",
        teacher.surname,
        teacher.firstname,
        teacher.contact,
    )
    .execute(&mut *conn)
    .await?
    .last_insert_rowid();

    let teacher_id = Id(id);

    Ok(teacher_id)
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    let teacher_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let count = sqlx::query!("DELETE FROM teachers WHERE teacher_id = ?", teacher_id)
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .rows_affected();

    if count == 0 {
        return Err(IdError::InvalidId(index));
    }

    Ok(())
}
