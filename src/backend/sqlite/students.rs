use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get(pool: &SqlitePool, index: Id) -> std::result::Result<Student, IdError<Error, Id>> {
    let student_id = index.0;

    let record_opt = sqlx::query!(
        "SELECT surname, firstname, email, phone, no_consecutive_slots FROM students WHERE student_id = ?",
        student_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    let student = Student {
        surname: record.surname,
        firstname: record.firstname,
        email: record.email,
        phone: record.phone,
        no_consecutive_slots: record.no_consecutive_slots != 0,
    };

    Ok(student)
}

pub async fn get_all(pool: &SqlitePool) -> Result<BTreeMap<Id, Student>> {
    let records = sqlx::query!(
        "SELECT student_id, surname, firstname, email, phone, no_consecutive_slots FROM students"
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|record| {
            (
                Id(record.student_id),
                Student {
                    surname: record.surname,
                    firstname: record.firstname,
                    email: record.email,
                    phone: record.phone,
                    no_consecutive_slots: record.no_consecutive_slots != 0,
                },
            )
        })
        .collect())
}

pub async fn add(pool: &SqlitePool, student: &Student) -> Result<Id> {
    let mut conn = pool.acquire().await?;

    let no_consecutive_slots = if student.no_consecutive_slots { 1 } else { 0 };
    let id = sqlx::query!(
        "INSERT INTO students (surname, firstname, email, phone, no_consecutive_slots) VALUES (?1, ?2, ?3, ?4, ?5)",
        student.surname,
        student.firstname,
        student.email,
        student.phone,
        no_consecutive_slots,
    )
    .execute(&mut *conn)
    .await?
    .last_insert_rowid();

    let student_id = Id(id);

    Ok(student_id)
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), Error> {
    let student_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM student_incompats WHERE student_id = ?",
        index.0,
    )
    .execute(pool)
    .await?;

    let _ = sqlx::query!("DELETE FROM student_subjects WHERE student_id = ?", index.0,)
        .execute(pool)
        .await?;

    let count = sqlx::query!("DELETE FROM students WHERE student_id = ?", student_id)
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .rows_affected();

    if count > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple students with id {:?}",
            index
        )));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    student: &Student,
) -> std::result::Result<(), IdError<Error, Id>> {
    let student_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let no_consecutive_slots = if student.no_consecutive_slots { 1 } else { 0 };
    let rows_affected = sqlx::query!(
        "UPDATE students SET surname = ?1, firstname = ?2, email = ?3, phone = ?4, no_consecutive_slots = ?5 WHERE student_id = ?6",
        student.surname,
        student.firstname,
        student.email,
        student.phone,
        no_consecutive_slots,
        student_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected > 1 {
        return Err(IdError::InternalError(Error::CorruptedDatabase(format!(
            "Multiple students with id {:?}",
            index
        ))));
    } else if rows_affected == 0 {
        return Err(IdError::InvalidId(index));
    }

    Ok(())
}
