use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SubjectGroupDb {
    name: String,
    optional: i64,
}

#[sqlx::test]
async fn subject_groups_add_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _id = store
        .subject_groups_add(&SubjectGroup {
            name: String::from("LV1"),
            optional: false,
        })
        .await
        .unwrap();

    let subject_groups =
        sqlx::query_as!(SubjectGroupDb, "SELECT name, optional FROM subject_groups")
            .fetch_all(&store.pool)
            .await
            .unwrap();

    let subject_groups_expected = vec![SubjectGroupDb {
        name: String::from("LV1"),
        optional: 0,
    }];

    assert_eq!(subject_groups, subject_groups_expected);
}

#[sqlx::test]
async fn subject_groups_add_multiple(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _id = store
        .subject_groups_add(&SubjectGroup {
            name: String::from("LV1"),
            optional: false,
        })
        .await
        .unwrap();

    let _id = store
        .subject_groups_add(&SubjectGroup {
            name: String::from("LV2"),
            optional: true,
        })
        .await
        .unwrap();

    let _id = store
        .subject_groups_add(&SubjectGroup {
            name: String::from("Spécialité"),
            optional: false,
        })
        .await
        .unwrap();

    let subject_groups =
        sqlx::query_as!(SubjectGroupDb, "SELECT name, optional FROM subject_groups")
            .fetch_all(&store.pool)
            .await
            .unwrap();

    let subject_groups_expected = vec![
        SubjectGroupDb {
            name: String::from("LV1"),
            optional: 0,
        },
        SubjectGroupDb {
            name: String::from("LV2"),
            optional: 1,
        },
        SubjectGroupDb {
            name: String::from("Spécialité"),
            optional: 0,
        },
    ];

    assert_eq!(subject_groups, subject_groups_expected);
}

#[sqlx::test]
async fn subject_groups_get_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO subject_groups (name, optional)
VALUES ("LV1", 0), ("LV2", 1), ("Spécialité", 0);
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    let subject_group = store
        .subject_groups_get(super::super::subject_groups::Id(2))
        .await
        .unwrap();

    let expected_result = SubjectGroup {
        name: String::from("LV2"),
        optional: true,
    };

    assert_eq!(subject_group, expected_result);
}

#[sqlx::test]
async fn teachers_get_all(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO subject_groups (name, optional)
VALUES ("LV1", 0), ("LV2", 1), ("Spécialité", 0);
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    let result = store.subject_groups_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::subject_groups::Id(1),
            SubjectGroup {
                name: String::from("LV1"),
                optional: false,
            },
        ),
        (
            super::super::subject_groups::Id(2),
            SubjectGroup {
                name: String::from("LV2"),
                optional: true,
            },
        ),
        (
            super::super::subject_groups::Id(3),
            SubjectGroup {
                name: String::from("Spécialité"),
                optional: false,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn subject_groups_remove_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO subject_groups (name, optional)
VALUES ("LV1", 0), ("LV2", 1), ("Spécialité", 0);
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    store
        .subject_groups_remove(super::super::subject_groups::Id(2))
        .await
        .unwrap();

    let result = store.subject_groups_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::subject_groups::Id(1),
            SubjectGroup {
                name: String::from("LV1"),
                optional: false,
            },
        ),
        (
            super::super::subject_groups::Id(3),
            SubjectGroup {
                name: String::from("Spécialité"),
                optional: false,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn subject_groups_remove_then_add(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO subject_groups (name, optional)
VALUES ("LV1", 0), ("LV2", 1), ("Spécialité", 0);
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    store
        .subject_groups_remove(super::super::subject_groups::Id(2))
        .await
        .unwrap();

    let id = store
        .subject_groups_add(&SubjectGroup {
            name: String::from("LV2"),
            optional: true,
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::subject_groups::Id(4));

    let result = store.subject_groups_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::subject_groups::Id(1),
            SubjectGroup {
                name: String::from("LV1"),
                optional: false,
            },
        ),
        (
            super::super::subject_groups::Id(3),
            SubjectGroup {
                name: String::from("Spécialité"),
                optional: false,
            },
        ),
        (
            super::super::subject_groups::Id(4),
            SubjectGroup {
                name: String::from("LV2"),
                optional: true,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn subject_groups_update(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO subject_groups (name, optional)
VALUES ("LV1", 0), ("LV2", 1), ("Spécialité", 0);
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    store
        .subject_groups_update(
            super::super::subject_groups::Id(2),
            &SubjectGroup {
                name: String::from("LVB"),
                optional: false,
            },
        )
        .await
        .unwrap();

    let result = store.subject_groups_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::subject_groups::Id(1),
            SubjectGroup {
                name: String::from("LV1"),
                optional: false,
            },
        ),
        (
            super::super::subject_groups::Id(2),
            SubjectGroup {
                name: String::from("LVB"),
                optional: false,
            },
        ),
        (
            super::super::subject_groups::Id(3),
            SubjectGroup {
                name: String::from("Spécialité"),
                optional: false,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}
