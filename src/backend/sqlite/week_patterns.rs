use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

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

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<WeekPattern>> {
    let names = sqlx::query!("SELECT week_pattern_id, name FROM week_patterns")
        .fetch_all(pool)
        .await?;

    let mut output = Vec::with_capacity(names.len());

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

        output.push(WeekPattern {
            name: record.name,
            weeks,
        });
    }

    Ok(output)
}

pub async fn add(pool: &SqlitePool, pattern: WeekPattern) -> Result<Id> {
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

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    let week_pattern_id = index.0;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM weeks WHERE week_pattern_id = ?",
        week_pattern_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM week_patterns WHERE week_pattern_id = ?",
        week_pattern_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    Ok(())
}
