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

pub async fn add(
    _pool: &SqlitePool,
    _group_list: &GroupList<super::students::Id>,
) -> std::result::Result<
    Id,
    InvalidCrossError<Error, GroupList<super::students::Id>, super::students::Id>,
> {
    todo!()
}

pub async fn remove(_pool: &SqlitePool, _index: Id) -> std::result::Result<(), IdError<Error, Id>> {
    todo!()
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
