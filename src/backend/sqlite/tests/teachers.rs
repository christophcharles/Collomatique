use super::*;

#[sqlx::test]
async fn teachers_add_one(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _id = store
        .teachers_add(&Teacher {
            surname: String::from("Durand"),
            firstname: String::from("Bernard"),
            contact: String::from("test@example.com"),
        })
        .await
        .unwrap();

    let teachers = sqlx::query_as!(Teacher, "SELECT surname, firstname, contact FROM teachers")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let teachers_expected = vec![Teacher {
        surname: String::from("Durand"),
        firstname: String::from("Bernard"),
        contact: String::from("test@example.com"),
    }];

    assert_eq!(teachers, teachers_expected);
}

#[sqlx::test]
async fn teachers_add_multiple(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _id = store
        .teachers_add(&Teacher {
            surname: String::from("Durand"),
            firstname: String::from("Bernard"),
            contact: String::from("test@example.com"),
        })
        .await
        .unwrap();

    let _id = store
        .teachers_add(&Teacher {
            surname: String::from("Dupont"),
            firstname: String::from("Leonard"),
            contact: String::from("06 07 08 09 10"),
        })
        .await
        .unwrap();

    let _id = store
        .teachers_add(&Teacher {
            surname: String::from("Tessier"),
            firstname: String::from("Lucie"),
            contact: String::from(""),
        })
        .await
        .unwrap();

    let teachers = sqlx::query_as!(Teacher, "SELECT surname, firstname, contact FROM teachers")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let teachers_expected = vec![
        Teacher {
            surname: String::from("Durand"),
            firstname: String::from("Bernard"),
            contact: String::from("test@example.com"),
        },
        Teacher {
            surname: String::from("Dupont"),
            firstname: String::from("Leonard"),
            contact: String::from("06 07 08 09 10"),
        },
        Teacher {
            surname: String::from("Tessier"),
            firstname: String::from("Lucie"),
            contact: String::from(""),
        },
    ];

    assert_eq!(teachers, teachers_expected);
}

#[sqlx::test]
async fn teachers_get_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO teachers (surname, firstname, contact)
VALUES ("Durand", "Bernard", "test@example.com"), ("Dupont", "Leonard", "06 07 08 09 10"), ("Tessier", "Lucie", "");
        "#
    ).execute(&store.pool).await.unwrap();

    let teacher = store
        .teachers_get(super::super::teachers::Id(2))
        .await
        .unwrap();

    let expected_result = Teacher {
        surname: String::from("Dupont"),
        firstname: String::from("Leonard"),
        contact: String::from("06 07 08 09 10"),
    };

    assert_eq!(teacher, expected_result);
}

#[sqlx::test]
async fn teachers_get_all(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO teachers (surname, firstname, contact)
VALUES ("Durand", "Bernard", "test@example.com"), ("Dupont", "Leonard", "06 07 08 09 10"), ("Tessier", "Lucie", "");
        "#
    ).execute(&store.pool).await.unwrap();

    let result = store.teachers_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::teachers::Id(1),
            Teacher {
                surname: String::from("Durand"),
                firstname: String::from("Bernard"),
                contact: String::from("test@example.com"),
            },
        ),
        (
            super::super::teachers::Id(2),
            Teacher {
                surname: String::from("Dupont"),
                firstname: String::from("Leonard"),
                contact: String::from("06 07 08 09 10"),
            },
        ),
        (
            super::super::teachers::Id(3),
            Teacher {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                contact: String::from(""),
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn teachers_remove_one(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO teachers (surname, firstname, contact)
VALUES ("Durand", "Bernard", "test@example.com"), ("Dupont", "Leonard", "06 07 08 09 10"), ("Tessier", "Lucie", "");
        "#
    ).execute(&store.pool).await.unwrap();

    unsafe {
        store
            .teachers_remove_unchecked(super::super::teachers::Id(2))
            .await
            .unwrap();
    }

    let result = store.teachers_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::teachers::Id(1),
            Teacher {
                surname: String::from("Durand"),
                firstname: String::from("Bernard"),
                contact: String::from("test@example.com"),
            },
        ),
        (
            super::super::teachers::Id(3),
            Teacher {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                contact: String::from(""),
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn teachers_remove_then_add(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO teachers (surname, firstname, contact)
VALUES ("Durand", "Bernard", "test@example.com"), ("Dupont", "Leonard", "06 07 08 09 10"), ("Tessier", "Lucie", "");
        "#
    ).execute(&store.pool).await.unwrap();

    unsafe {
        store
            .teachers_remove_unchecked(super::super::teachers::Id(2))
            .await
            .unwrap();
    }

    let id = store
        .teachers_add(&Teacher {
            surname: String::from("Dupont"),
            firstname: String::from("Leonard"),
            contact: String::from("06 07 08 09 10"),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::teachers::Id(4));

    let result = store.teachers_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::teachers::Id(1),
            Teacher {
                surname: String::from("Durand"),
                firstname: String::from("Bernard"),
                contact: String::from("test@example.com"),
            },
        ),
        (
            super::super::teachers::Id(3),
            Teacher {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                contact: String::from(""),
            },
        ),
        (
            super::super::teachers::Id(4),
            Teacher {
                surname: String::from("Dupont"),
                firstname: String::from("Leonard"),
                contact: String::from("06 07 08 09 10"),
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn teachers_update(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO teachers (surname, firstname, contact)
VALUES ("Durand", "Bernard", "test@example.com"), ("Dupont", "Leonard", "06 07 08 09 10"), ("Tessier", "Lucie", "");
        "#
    ).execute(&store.pool).await.unwrap();

    store
        .teachers_update(
            super::super::teachers::Id(2),
            &Teacher {
                surname: String::from("Dupond"),
                firstname: String::from("Leonard"),
                contact: String::from("07 06 08 09 10"),
            },
        )
        .await
        .unwrap();

    let result = store.teachers_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::teachers::Id(1),
            Teacher {
                surname: String::from("Durand"),
                firstname: String::from("Bernard"),
                contact: String::from("test@example.com"),
            },
        ),
        (
            super::super::teachers::Id(2),
            Teacher {
                surname: String::from("Dupond"),
                firstname: String::from("Leonard"),
                contact: String::from("07 06 08 09 10"),
            },
        ),
        (
            super::super::teachers::Id(3),
            Teacher {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                contact: String::from(""),
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}
