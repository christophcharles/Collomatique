use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StudentDb {
    surname: String,
    firstname: String,
    email: Option<String>,
    phone: Option<String>,
    no_consecutive_slots: i64,
}

#[sqlx::test]
async fn students_add_one(pool: sqlx::SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    let _id = store
        .students_add(&Student {
            surname: String::from("Durand"),
            firstname: String::from("Bernard"),
            email: None,
            phone: Some(String::from("07 99 99 99 01")),
            no_consecutive_slots: true,
        })
        .await
        .unwrap();

    let students = sqlx::query_as!(
        StudentDb,
        "SELECT surname, firstname, email, phone, no_consecutive_slots FROM students"
    )
    .fetch_all(&store.pool)
    .await
    .unwrap();

    let students_expected = vec![StudentDb {
        surname: String::from("Durand"),
        firstname: String::from("Bernard"),
        email: None,
        phone: Some(String::from("07 99 99 99 01")),
        no_consecutive_slots: 1,
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
            no_consecutive_slots: true,
        })
        .await
        .unwrap();

    let _id = store
        .students_add(&Student {
            surname: String::from("Dupont"),
            firstname: String::from("Leonard"),
            email: Some(String::from("old_school_is_cool@gmail.com")),
            phone: Some(String::from("06 99 98 97 96")),
            no_consecutive_slots: false,
        })
        .await
        .unwrap();

    let _id = store
        .students_add(&Student {
            surname: String::from("Tessier"),
            firstname: String::from("Lucie"),
            email: None,
            phone: None,
            no_consecutive_slots: false,
        })
        .await
        .unwrap();

    let students = sqlx::query_as!(
        StudentDb,
        "SELECT surname, firstname, email, phone, no_consecutive_slots FROM students"
    )
    .fetch_all(&store.pool)
    .await
    .unwrap();

    let students_expected = vec![
        StudentDb {
            surname: String::from("Durand"),
            firstname: String::from("Bernard"),
            email: None,
            phone: Some(String::from("07 99 99 99 01")),
            no_consecutive_slots: 1,
        },
        StudentDb {
            surname: String::from("Dupont"),
            firstname: String::from("Leonard"),
            email: Some(String::from("old_school_is_cool@gmail.com")),
            phone: Some(String::from("06 99 98 97 96")),
            no_consecutive_slots: 0,
        },
        StudentDb {
            surname: String::from("Tessier"),
            firstname: String::from("Lucie"),
            email: None,
            phone: None,
            no_consecutive_slots: 0,
        },
    ];

    assert_eq!(students, students_expected);
}

#[sqlx::test]
async fn students_get_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname, email, phone, no_consecutive_slots)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01", 1), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96", 0), ("Tessier", "Lucie", NULL, NULL, 0);
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
        no_consecutive_slots: false,
    };

    assert_eq!(student, expected_result);
}

#[sqlx::test]
async fn students_get_all(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname, email, phone, no_consecutive_slots)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01", 1), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96", 0), ("Tessier", "Lucie", NULL, NULL, 0);
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
                no_consecutive_slots: true,
            },
        ),
        (
            super::super::students::Id(2),
            Student {
                surname: String::from("Dupont"),
                firstname: String::from("Leonard"),
                email: Some(String::from("old_school_is_cool@gmail.com")),
                phone: Some(String::from("06 99 98 97 96")),
                no_consecutive_slots: false,
            },
        ),
        (
            super::super::students::Id(3),
            Student {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                email: None,
                phone: None,
                no_consecutive_slots: false,
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
INSERT INTO students (surname, firstname, email, phone, no_consecutive_slots)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01", 1), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96", 0), ("Tessier", "Lucie", NULL, NULL, 0);
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
                no_consecutive_slots: true,
            },
        ),
        (
            super::super::students::Id(3),
            Student {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                email: None,
                phone: None,
                no_consecutive_slots: false,
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
INSERT INTO students (surname, firstname, email, phone, no_consecutive_slots)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01", 1), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96", 0), ("Tessier", "Lucie", NULL, NULL, 0);
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
            no_consecutive_slots: true,
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
                no_consecutive_slots: true,
            },
        ),
        (
            super::super::students::Id(3),
            Student {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                email: None,
                phone: None,
                no_consecutive_slots: false,
            },
        ),
        (
            super::super::students::Id(4),
            Student {
                surname: String::from("Dupont"),
                firstname: String::from("Leonard"),
                email: Some(String::from("old_school_is_cool@gmail.com")),
                phone: None,
                no_consecutive_slots: true,
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
INSERT INTO students (surname, firstname, email, phone, no_consecutive_slots)
VALUES ("Durand", "Bernard", NULL, "07 99 99 99 01", 1), ("Dupont", "Leonard", "old_school_is_cool@gmail.com", "06 99 98 97 96", 0), ("Tessier", "Lucie", NULL, NULL, 0);
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
                no_consecutive_slots: true,
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
                no_consecutive_slots: true,
            },
        ),
        (
            super::super::students::Id(2),
            Student {
                surname: String::from("Dupond"),
                firstname: String::from("Leonard"),
                email: Some(String::from("old_school_is_cool@gmail.com")),
                phone: None,
                no_consecutive_slots: true,
            },
        ),
        (
            super::super::students::Id(3),
            Student {
                surname: String::from("Tessier"),
                firstname: String::from("Lucie"),
                email: None,
                phone: None,
                no_consecutive_slots: false,
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}
