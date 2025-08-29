use super::*;

async fn prepare_db(pool: sqlx::SqlitePool) -> Store {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname)
VALUES ("Roth", ""), ("Marin", ""), ("Bordes", ""), ("Bresson", ""), ("Gosset",""),
("Martel", ""), ("Delarue", ""), ("Chauvet", ""), ("Bourdon", ""), ("Lafond", ""),
("Rondeau", ""), ("Vigneron", ""), ("Davy", ""), ("Gosselin", ""), ("Jeannin", ""),
("Sicard", ""), ("Mounier", ""), ("Lafon", ""), ("Brun", ""), ("Hardy", ""),
("Girault", ""), ("Delahaye", ""), ("Levasseur", ""), ("Gonthier", "");

INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);

INSERT INTO incompats (name, max_count) VALUES ("ESH", 0), ("LV2 - Espagnol", 0), ("LV2 - Allemand", 0),
("Repas midi - lundi", 2), ("Repas midi - mardi", 2), ("Repas midi - mercredi", 2), ("Repas midi - jeudi", 2), ("Repas midi - vendredi", 2);
INSERT INTO incompat_groups (incompat_id) VALUES (1), (2), (3), (4), (4), (4), (5), (5), (5), (6), (6), (6), (7), (7), (7), (8), (8), (8);
INSERT INTO incompat_group_items (incompat_group_id, week_pattern_id, start_day, start_time, duration)
VALUES (1,1,1,600,60), (2,1,0,720,60), (2, 1, 3, 720, 120), (3, 1, 0, 480, 120), (4, 1, 0, 660, 60), (5, 1, 0, 720, 60), (6, 1, 0, 780, 60),
(7, 1, 1, 660, 60), (8, 1, 1, 720, 60), (9, 1, 1, 780, 60), (10, 1, 2, 660, 60), (11, 1, 2, 720, 60), (12, 1, 2, 780, 60),
(13, 1, 3, 660, 60), (14, 1, 3, 720, 60), (15, 1, 3, 780, 60), (16, 1, 4, 660, 60), (17, 1, 4, 720, 60), (18, 1, 4, 780, 60);
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    store
}

async fn prepare_example_db(pool: sqlx::SqlitePool) -> Store {
    let store = prepare_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO student_incompats (student_id, incompat_id)
VALUES (1, 4), (1, 5), (1, 6), (1, 7), (1, 8),
(2, 4), (2, 5), (2, 6), (2, 7), (2, 8),
(3, 4), (3, 5), (3, 6), (3, 7), (3, 8),
(4, 4), (4, 5), (4, 6), (4, 7), (4, 8),
(5, 4), (5, 5), (5, 6), (5, 7), (5, 8),
(6, 4), (6, 5), (6, 6), (6, 7), (6, 8),
(7, 4), (7, 5), (7, 6), (7, 7), (7, 8),
(8, 4), (8, 5), (8, 6), (8, 7), (8, 8),
(9, 4), (9, 5), (9, 6), (9, 7), (9, 8),
(10, 4), (10, 5), (10, 6), (10, 7), (10, 8),
(11, 4), (11, 5), (11, 6), (11, 7), (11, 8),
(12, 4), (12, 5), (12, 6), (12, 7), (12, 8),
(13, 4), (13, 5), (13, 6), (13, 7), (13, 8),
(14, 4), (14, 5), (14, 6), (14, 7), (14, 8),
(15, 4), (15, 5), (15, 6), (15, 7), (15, 8),
(16, 4), (16, 5), (16, 6), (16, 7), (16, 8),
(17, 4), (17, 5), (17, 6), (17, 7), (17, 8),
(18, 4), (18, 5), (18, 6), (18, 7), (18, 8),
(19, 4), (19, 5), (19, 6), (19, 7), (19, 8),
(20, 4), (20, 5), (20, 6), (20, 7), (20, 8),
(21, 4), (21, 5), (21, 6), (21, 7), (21, 8),
(22, 4), (22, 5), (22, 6), (22, 7), (22, 8),
(23, 4), (23, 5), (23, 6), (23, 8),
(24, 4), (24, 5), (24, 6), (24, 7);
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    store
}

#[sqlx::test]
async fn incompat_for_student_get(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(5),
            super::super::incompats::Id(4),
        )
        .await
        .unwrap();

    assert_eq!(value, true);

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(22),
            super::super::incompats::Id(7),
        )
        .await
        .unwrap();

    assert_eq!(value, true);

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(23),
            super::super::incompats::Id(7),
        )
        .await
        .unwrap();

    assert_eq!(value, false);

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(23),
            super::super::incompats::Id(8),
        )
        .await
        .unwrap();

    assert_eq!(value, true);

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(24),
            super::super::incompats::Id(8),
        )
        .await
        .unwrap();

    assert_eq!(value, false);
}

#[sqlx::test]
async fn incompat_for_student_set(pool: sqlx::SqlitePool) {
    let mut store = prepare_example_db(pool).await;

    unsafe {
        store
            .incompat_for_student_set_unchecked(
                super::super::students::Id(1),
                super::super::incompats::Id(4),
                false,
            )
            .await
            .unwrap();
    }

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(1),
            super::super::incompats::Id(4),
        )
        .await
        .unwrap();

    assert_eq!(value, false);

    unsafe {
        store
            .incompat_for_student_set_unchecked(
                super::super::students::Id(5),
                super::super::incompats::Id(4),
                true,
            )
            .await
            .unwrap();
    }

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(5),
            super::super::incompats::Id(4),
        )
        .await
        .unwrap();

    assert_eq!(value, true);

    unsafe {
        store
            .incompat_for_student_set_unchecked(
                super::super::students::Id(23),
                super::super::incompats::Id(7),
                false,
            )
            .await
            .unwrap();
    }

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(23),
            super::super::incompats::Id(7),
        )
        .await
        .unwrap();

    assert_eq!(value, false);

    unsafe {
        store
            .incompat_for_student_set_unchecked(
                super::super::students::Id(23),
                super::super::incompats::Id(7),
                true,
            )
            .await
            .unwrap();
    }

    let value = store
        .incompat_for_student_get(
            super::super::students::Id(23),
            super::super::incompats::Id(7),
        )
        .await
        .unwrap();

    assert_eq!(value, true);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StudentIncompatDb {
    student_id: i64,
    incompat_id: i64,
}

#[sqlx::test]
async fn remove_student(pool: sqlx::SqlitePool) {
    let mut store = prepare_example_db(pool).await;

    store
        .students_remove(super::super::students::Id(3))
        .await
        .unwrap();

    let records = sqlx::query_as!(
        StudentIncompatDb,
        "SELECT student_id, incompat_id FROM student_incompats"
    )
    .fetch_all(&store.pool)
    .await
    .unwrap();

    let records_expected = vec![
        // Student 1
        StudentIncompatDb {
            student_id: 1,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 1,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 1,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 1,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 1,
            incompat_id: 8,
        },
        // Student 2
        StudentIncompatDb {
            student_id: 2,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 2,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 2,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 2,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 2,
            incompat_id: 8,
        },
        // Student 3 (nothing should be left)
        // Student 4
        StudentIncompatDb {
            student_id: 4,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 4,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 4,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 4,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 4,
            incompat_id: 8,
        },
        // Student 5
        StudentIncompatDb {
            student_id: 5,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 5,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 5,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 5,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 5,
            incompat_id: 8,
        },
        // Student 6
        StudentIncompatDb {
            student_id: 6,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 6,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 6,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 6,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 6,
            incompat_id: 8,
        },
        // Student 7
        StudentIncompatDb {
            student_id: 7,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 7,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 7,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 7,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 7,
            incompat_id: 8,
        },
        // Student 8
        StudentIncompatDb {
            student_id: 8,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 8,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 8,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 8,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 8,
            incompat_id: 8,
        },
        // Student 9
        StudentIncompatDb {
            student_id: 9,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 9,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 9,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 9,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 9,
            incompat_id: 8,
        },
        // Student 10
        StudentIncompatDb {
            student_id: 10,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 10,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 10,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 10,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 10,
            incompat_id: 8,
        },
        // Student 11
        StudentIncompatDb {
            student_id: 11,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 11,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 11,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 11,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 11,
            incompat_id: 8,
        },
        // Student 12
        StudentIncompatDb {
            student_id: 12,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 12,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 12,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 12,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 12,
            incompat_id: 8,
        },
        // Student 13
        StudentIncompatDb {
            student_id: 13,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 13,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 13,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 13,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 13,
            incompat_id: 8,
        },
        // Student 14
        StudentIncompatDb {
            student_id: 14,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 14,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 14,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 14,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 14,
            incompat_id: 8,
        },
        // Student 15
        StudentIncompatDb {
            student_id: 15,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 15,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 15,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 15,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 15,
            incompat_id: 8,
        },
        // Student 16
        StudentIncompatDb {
            student_id: 16,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 16,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 16,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 16,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 16,
            incompat_id: 8,
        },
        // Student 17
        StudentIncompatDb {
            student_id: 17,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 17,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 17,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 17,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 17,
            incompat_id: 8,
        },
        // Student 18
        StudentIncompatDb {
            student_id: 18,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 18,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 18,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 18,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 18,
            incompat_id: 8,
        },
        // Student 19
        StudentIncompatDb {
            student_id: 19,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 19,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 19,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 19,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 19,
            incompat_id: 8,
        },
        // Student 20
        StudentIncompatDb {
            student_id: 20,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 20,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 20,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 20,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 20,
            incompat_id: 8,
        },
        // Student 21
        StudentIncompatDb {
            student_id: 21,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 21,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 21,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 21,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 21,
            incompat_id: 8,
        },
        // Student 22
        StudentIncompatDb {
            student_id: 22,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 22,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 22,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 22,
            incompat_id: 7,
        },
        StudentIncompatDb {
            student_id: 22,
            incompat_id: 8,
        },
        // Student 23
        StudentIncompatDb {
            student_id: 23,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 23,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 23,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 23,
            incompat_id: 8,
        },
        // Student 24
        StudentIncompatDb {
            student_id: 24,
            incompat_id: 4,
        },
        StudentIncompatDb {
            student_id: 24,
            incompat_id: 5,
        },
        StudentIncompatDb {
            student_id: 24,
            incompat_id: 6,
        },
        StudentIncompatDb {
            student_id: 24,
            incompat_id: 7,
        },
    ];

    assert_eq!(records, records_expected);
}
