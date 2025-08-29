use super::*;

#[sqlx::test]
async fn students_add_one(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _id = store
        .students_add(&Student {
            surname: String::from("Durand"),
            firstname: String::from("Bernard"),
            email: None,
            phone: Some(String::from("07 99 99 99 01")),
        })
        .await
        .unwrap();

    let students = sqlx::query_as!(
        Student,
        "SELECT surname, firstname, email, phone FROM students"
    )
    .fetch_all(&store.pool)
    .await
    .unwrap();

    let students_expected = vec![Student {
        surname: String::from("Durand"),
        firstname: String::from("Bernard"),
        email: None,
        phone: Some(String::from("07 99 99 99 01")),
    }];

    assert_eq!(students, students_expected);
}

#[sqlx::test]
async fn students_add_multiple(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _id = store
        .students_add(&Student {
            surname: String::from("Durand"),
            firstname: String::from("Bernard"),
            email: None,
            phone: Some(String::from("07 99 99 99 01")),
        })
        .await
        .unwrap();

    let _id = store
        .students_add(&Student {
            surname: String::from("Dupont"),
            firstname: String::from("Leonard"),
            email: Some(String::from("old_school_is_cool@gmail.com")),
            phone: Some(String::from("06 99 98 97 96")),
        })
        .await
        .unwrap();

    let _id = store
        .students_add(&Student {
            surname: String::from("Tessier"),
            firstname: String::from("Lucie"),
            email: None,
            phone: None,
        })
        .await
        .unwrap();

    let students = sqlx::query_as!(
        Student,
        "SELECT surname, firstname, email, phone FROM students"
    )
    .fetch_all(&store.pool)
    .await
    .unwrap();

    let students_expected = vec![
        Student {
            surname: String::from("Durand"),
            firstname: String::from("Bernard"),
            email: None,
            phone: Some(String::from("07 99 99 99 01")),
        },
        Student {
            surname: String::from("Dupont"),
            firstname: String::from("Leonard"),
            email: Some(String::from("old_school_is_cool@gmail.com")),
            phone: Some(String::from("06 99 98 97 96")),
        },
        Student {
            surname: String::from("Tessier"),
            firstname: String::from("Lucie"),
            email: None,
            phone: None,
        },
    ];

    assert_eq!(students, students_expected);
}

#[sqlx::test]
async fn students_get_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname, email, phone)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01"), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96"), ("Tessier", "Lucie", NULL, NULL);
        "#
    ).execute(&store.pool).await.unwrap();

    let student = store
        .students_get(super::super::students::Id(2))
        .await
        .unwrap();

    let expected_result = Student {
        surname: String::from("Dupont"),
        firstname: String::from("Leonard"),
        email: Some(String::from("old_school_is_cool@gmail.com")),
        phone: Some(String::from("06 99 98 97 96")),
    };

    assert_eq!(student, expected_result);
}

#[sqlx::test]
async fn students_get_all(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname, email, phone)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01"), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96"), ("Tessier", "Lucie", NULL, NULL);
        "#
    ).execute(&store.pool).await.unwrap();

    let result = store.students_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::students::Id(1),
            Student {
                surname: String::from("Durand"),
                firstname: String::from("Bernard"),
                email: None,
                phone: Some(String::from("07 99 99 99 01")),
            },
        ),
        (
            super::super::students::Id(2),
            Student {
                surname: String::from("Dupont"),
                firstname: String::from("Leonard"),
                email: Some(String::from("old_school_is_cool@gmail.com")),
                phone: Some(String::from("06 99 98 97 96")),
            },
        ),
        (
            super::super::students::Id(3),
            Student {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                email: None,
                phone: None,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn students_remove_one(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname, email, phone)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01"), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96"), ("Tessier", "Lucie", NULL, NULL);
        "#
    ).execute(&store.pool).await.unwrap();

    unsafe {
        store
            .students_remove_unchecked(super::super::students::Id(2))
            .await
            .unwrap();
    }

    let result = store.students_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::students::Id(1),
            Student {
                surname: String::from("Durand"),
                firstname: String::from("Bernard"),
                email: None,
                phone: Some(String::from("07 99 99 99 01")),
            },
        ),
        (
            super::super::students::Id(3),
            Student {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                email: None,
                phone: None,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn students_remove_then_add(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname, email, phone)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01"), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96"), ("Tessier", "Lucie", NULL, NULL);
        "#
    ).execute(&store.pool).await.unwrap();

    unsafe {
        store
            .students_remove_unchecked(super::super::students::Id(2))
            .await
            .unwrap();
    }

    let id = store
        .students_add(&Student {
            surname: String::from("Dupont"),
            firstname: String::from("Leonard"),
            email: Some(String::from("old_school_is_cool@gmail.com")),
            phone: None,
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::students::Id(4));

    let result = store.students_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::students::Id(1),
            Student {
                surname: String::from("Durand"),
                firstname: String::from("Bernard"),
                email: None,
                phone: Some(String::from("07 99 99 99 01")),
            },
        ),
        (
            super::super::students::Id(3),
            Student {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                email: None,
                phone: None,
            },
        ),
        (
            super::super::students::Id(4),
            Student {
                surname: String::from("Dupont"),
                firstname: String::from("Leonard"),
                email: Some(String::from("old_school_is_cool@gmail.com")),
                phone: None,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn students_update(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname, email, phone)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01"), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96"), ("Tessier", "Lucie", NULL, NULL);
        "#
    ).execute(&store.pool).await.unwrap();

    store
        .students_update(
            super::super::students::Id(2),
            &Student {
                surname: String::from("Dupond"),
                firstname: String::from("Leonard"),
                email: Some(String::from("old_school_is_cool@gmail.com")),
                phone: None,
            },
        )
        .await
        .unwrap();

    let result = store.students_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::students::Id(1),
            Student {
                surname: String::from("Durand"),
                firstname: String::from("Bernard"),
                email: None,
                phone: Some(String::from("07 99 99 99 01")),
            },
        ),
        (
            super::super::students::Id(2),
            Student {
                surname: String::from("Dupond"),
                firstname: String::from("Leonard"),
                email: Some(String::from("old_school_is_cool@gmail.com")),
                phone: None,
            },
        ),
        (
            super::super::students::Id(3),
            Student {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                email: None,
                phone: None,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}
