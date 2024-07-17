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
) -> std::result::Result<WeekPattern, IdError<Error, Id>> {
    let week_pattern_id = index.0;

    let name_opt = sqlx::query!(
        "SELECT name FROM week_patterns WHERE week_pattern_id = ?",
        week_pattern_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let name = name_opt.ok_or(IdError::InvalidId(index))?;

    let data = sqlx::query!(
        "SELECT week FROM weeks WHERE week_pattern_id = ?",
        week_pattern_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    let weeks = data
        .iter()
        .map(|x| {
            let num = u32::try_from(x.week).map_err(|_| {
                Error::CorruptedDatabase(format!(
                    "Database references invalid u32 week ({}) for week_pattern_id {}",
                    x.week, week_pattern_id
                ))
            })?;
            Ok(Week(num))
        })
        .collect::<Result<BTreeSet<_>>>()?;

    Ok(WeekPattern {
        name: name.name,
        weeks,
    })
}

pub async fn get_all(pool: &SqlitePool) -> Result<BTreeMap<Id, WeekPattern>> {
    let names = sqlx::query!("SELECT week_pattern_id, name FROM week_patterns")
        .fetch_all(pool)
        .await?;

    let mut output = BTreeMap::new();

    for record in names {
        let data = sqlx::query!(
            "SELECT week FROM weeks WHERE week_pattern_id = ?",
            record.week_pattern_id
        )
        .fetch_all(pool)
        .await?;

        let weeks = data
            .iter()
            .map(|x| {
                let num = match u32::try_from(x.week) {
                    Ok(val) => val,
                    Err(_) => {
                        return Err(Error::CorruptedDatabase(format!(
                            "Database references invalid u32 week ({}) for week_pattern_id {}",
                            x.week, record.week_pattern_id
                        )))
                    }
                };

                Ok(Week(num))
            })
            .collect::<Result<BTreeSet<_>>>()?;

        output.insert(
            Id(record.week_pattern_id),
            WeekPattern {
                name: record.name,
                weeks,
            },
        );
    }

    Ok(output)
}

pub async fn add(pool: &SqlitePool, pattern: &WeekPattern) -> Result<Id> {
    let mut conn = pool.acquire().await?;

    let id = sqlx::query!("INSERT INTO week_patterns (name) VALUES (?)", pattern.name)
        .execute(&mut *conn)
        .await?
        .last_insert_rowid();

    for Week(week) in pattern.weeks.iter().copied() {
        let _ = sqlx::query!(
            "INSERT INTO weeks (week_pattern_id, week) VALUES (?1, ?2)",
            id,
            week
        )
        .execute(&mut *conn)
        .await?;
    }

    let week_pattern_id = Id(id);

    Ok(week_pattern_id)
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), Error> {
    let week_pattern_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM weeks WHERE week_pattern_id = ?",
        week_pattern_id
    )
    .execute(&mut *conn)
    .await?;

    let count = sqlx::query!(
        "DELETE FROM week_patterns WHERE week_pattern_id = ?",
        week_pattern_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if count > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple week_patterns with id {:?}",
            index
        )));
    }

    Ok(())
}

pub async fn update(
    pool: &SqlitePool,
    index: Id,
    pattern: &WeekPattern,
) -> std::result::Result<(), Error> {
    let week_pattern_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let rows_affected = sqlx::query!(
        "UPDATE week_patterns SET name = ?1 WHERE week_pattern_id = ?2",
        pattern.name,
        week_pattern_id,
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if rows_affected > 1 {
        return Err(Error::CorruptedDatabase(format!(
            "Multiple week_patterns with id {:?}",
            index
        )));
    }

    let _ = sqlx::query!(
        "DELETE FROM weeks WHERE week_pattern_id = ?",
        week_pattern_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    for Week(week) in pattern.weeks.iter().copied() {
        let _ = sqlx::query!(
            "INSERT INTO weeks (week_pattern_id, week) VALUES (?1, ?2)",
            week_pattern_id,
            week
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;
    }

    Ok(())
}
