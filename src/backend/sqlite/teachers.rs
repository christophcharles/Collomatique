use super::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Teacher id {0} is invalid")]
    InvalidId(TeacherId),
    #[error("sqlx error")]
    SqlxError(#[from] sqlx::Error),
    #[error("Corrupted database: {0}")]
    CorruptedDatabase(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub async fn get(pool: &SqlitePool, index: TeacherId) -> Result<Teacher> {
    let teacher_id = i64::try_from(index.0).map_err(|_| Error::InvalidId(index))?;

    let record_opt = sqlx::query_as!(
        Teacher,
        "SELECT surname, firstname, contact FROM teachers WHERE teacher_id = ?",
        teacher_id
    )
    .fetch_optional(pool)
    .await?;

    let record = record_opt.ok_or(Error::InvalidId(index))?;

    Ok(record)
}

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Teacher>> {
    let records = sqlx::query_as!(Teacher, "SELECT surname, firstname, contact FROM teachers",)
        .fetch_all(pool)
        .await?;

    Ok(records)
}

pub async fn add(pool: &SqlitePool, teacher: Teacher) -> Result<TeacherId> {
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

    let teacher_id = TeacherId(usize::try_from(id).expect("Should be valid usize id"));

    Ok(teacher_id)
}

pub async fn remove(pool: &SqlitePool, index: TeacherId) -> Result<()> {
    let teacher_id = i64::try_from(index.0).map_err(|_| Error::InvalidId(index))?;

    let mut conn = pool.acquire().await?;

    let count = sqlx::query!("DELETE FROM teachers WHERE teacher_id = ?", teacher_id)
        .execute(&mut *conn)
        .await?
        .rows_affected();

    if count == 0 {
        return Err(Error::InvalidId(index));
    }

    Ok(())
}
