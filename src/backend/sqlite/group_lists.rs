use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub(super) i64);

pub async fn get_all(pool: &SqlitePool) -> Result<BTreeMap<Id, GroupList<super::students::Id>>> {
    let records = sqlx::query!("SELECT group_list_id, name FROM group_lists",)
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

    let mut output = BTreeMap::new();

    for record in records {
        let group_list_id = record.group_list_id;

        let groups_data = sqlx::query!(
            r#"
    SELECT groups.group_id AS group_id, name, extendable
    FROM groups
    JOIN group_list_items ON groups.group_id = group_list_items.group_id
    WHERE group_list_id = ?
            "#,
            group_list_id
        )
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

        let (groups_map, groups): (BTreeMap<_, _>, _) = groups_data
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                (
                    (x.group_id, i),
                    Group {
                        name: x.name,
                        extendable: x.extendable != 0,
                    },
                )
            })
            .unzip();

        let students_data = sqlx::query!(
            "SELECT group_id, student_id FROM group_items WHERE group_list_id = ?",
            group_list_id
        )
        .fetch_all(pool)
        .await
        .map_err(Error::from)?;

        let students_mapping = students_data
            .into_iter()
            .map(|x| {
                Result::Ok((
                    super::students::Id(x.student_id),
                    groups_map
                        .get(&x.group_id)
                        .copied()
                        .ok_or(Error::CorruptedDatabase(format!(
                            "Invalid group_id ({}) for student ({}) in group_items table",
                            x.group_id, x.student_id
                        )))?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;

        output.insert(
            Id(group_list_id),
            GroupList {
                name: record.name,
                groups,
                students_mapping,
            },
        );
    }

    Ok(output)
}

pub async fn get(
    pool: &SqlitePool,
    index: Id,
) -> std::result::Result<GroupList<super::students::Id>, IdError<Error, Id>> {
    let group_list_id = index.0;

    let record_opt = sqlx::query!(
        "SELECT name FROM group_lists WHERE group_list_id = ?",
        group_list_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::from)?;

    let record = record_opt.ok_or(IdError::InvalidId(index))?;

    let groups_data = sqlx::query!(
        r#"
SELECT groups.group_id AS group_id, name, extendable
FROM groups
JOIN group_list_items ON groups.group_id = group_list_items.group_id
WHERE group_list_id = ?
        "#,
        group_list_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    let (groups_map, groups): (BTreeMap<_, _>, _) = groups_data
        .into_iter()
        .enumerate()
        .map(|(i, x)| {
            (
                (x.group_id, i),
                Group {
                    name: x.name,
                    extendable: x.extendable != 0,
                },
            )
        })
        .unzip();

    let students_data = sqlx::query!(
        "SELECT group_id, student_id FROM group_items WHERE group_list_id = ?",
        group_list_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    let students_mapping = students_data
        .into_iter()
        .map(|x| {
            Result::Ok((
                super::students::Id(x.student_id),
                groups_map
                    .get(&x.group_id)
                    .copied()
                    .ok_or(Error::CorruptedDatabase(format!(
                        "Invalid group_id ({}) for student ({}) in group_items table",
                        x.group_id, x.student_id
                    )))?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    let output = GroupList::<super::students::Id> {
        name: record.name,
        groups,
        students_mapping,
    };

    Ok(output)
}

async fn search_invalid_student_id(
    pool: &SqlitePool,
    group_list: &GroupList<super::students::Id>,
) -> Result<Option<super::students::Id>> {
    let students_id = sqlx::query!("SELECT student_id FROM students",)
        .fetch_all(pool)
        .await
        .map_err(Error::from)?
        .iter()
        .map(|x| x.student_id)
        .collect::<BTreeSet<_>>();

    for (student, _group) in &group_list.students_mapping {
        if !students_id.contains(&student.0) {
            return Ok(Some(*student));
        }
    }

    Ok(None)
}

fn validate_groups(group_list: &GroupList<super::students::Id>) -> bool {
    for (_student, &group) in &group_list.students_mapping {
        if group >= group_list.groups.len() {
            return false;
        }
    }
    true
}

pub async fn add(
    pool: &SqlitePool,
    group_list: &GroupList<super::students::Id>,
) -> std::result::Result<
    Id,
    InvalidCrossError<Error, GroupList<super::students::Id>, super::students::Id>,
> {
    if let Some(student_id) = search_invalid_student_id(pool, group_list).await? {
        return Err(InvalidCrossError::InvalidCrossId(student_id));
    }
    if !validate_groups(group_list) {
        return Err(InvalidCrossError::InvalidData(group_list.clone()));
    }

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let group_list_id = sqlx::query!("INSERT INTO group_lists (name) VALUES (?)", group_list.name,)
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

    let mut group_ids = Vec::with_capacity(group_list.groups.len());
    for group in &group_list.groups {
        let extendable = if group.extendable { 1 } else { 0 };
        let group_id = sqlx::query!(
            "INSERT INTO groups (name, extendable) VALUES (?1, ?2)",
            group.name,
            extendable,
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .last_insert_rowid();

        group_ids.push(group_id);

        let _ = sqlx::query!(
            "INSERT INTO group_list_items (group_list_id, group_id) VALUES (?1, ?2)",
            group_list_id,
            group_id,
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;
    }

    for (&student_id, &group_num) in &group_list.students_mapping {
        let group_id = group_ids[group_num];

        let _ = sqlx::query!(
            "INSERT INTO group_items (group_list_id, group_id, student_id) VALUES (?1, ?2, ?3)",
            group_list_id,
            group_id,
            student_id.0
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?;
    }

    Ok(Id(group_list_id))
}

pub async fn remove(pool: &SqlitePool, index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    let group_list_id = index.0;

    let groups_to_delete = sqlx::query!(
        "SELECT group_id FROM group_list_items WHERE group_list_id = ?",
        group_list_id
    )
    .fetch_all(pool)
    .await
    .map_err(Error::from)?;

    let mut conn = pool.acquire().await.map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM group_items WHERE group_list_id = ?",
        group_list_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    let _ = sqlx::query!(
        "DELETE FROM group_list_items WHERE group_list_id = ?",
        group_list_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?;

    for group in groups_to_delete {
        let _ = sqlx::query!("DELETE FROM groups WHERE group_id = ?", group.group_id)
            .execute(&mut *conn)
            .await
            .map_err(Error::from)?;
    }

    let count = sqlx::query!(
        "DELETE FROM group_lists WHERE group_list_id = ?",
        group_list_id
    )
    .execute(&mut *conn)
    .await
    .map_err(Error::from)?
    .rows_affected();

    if count > 1 {
        return Err(IdError::InternalError(Error::CorruptedDatabase(format!(
            "Multiple group_lists with id {:?}",
            index
        ))));
    } else if count == 0 {
        return Err(IdError::InvalidId(index));
    }

    Ok(())
}

pub async fn update(
    _pool: &SqlitePool,
    _index: Id,
    _group_list: &GroupList<super::students::Id>,
) -> std::result::Result<
    (),
    InvalidCrossIdError<Error, GroupList<super::students::Id>, Id, super::students::Id>,
> {
    todo!()
}
