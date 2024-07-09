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
        
INSERT INTO groups (name, extendable)
VALUES ("1", 0), ("2", 0), ("3", 0), ("4", 0), ("5", 0), ("6", 0), ("7", 0), ("8", 0),
("5", 0), ("6", 0), ("3+7", 0), ("P", 0), ("I", 0);

INSERT INTO group_lists (name)
VALUES ("Groupes"), ("HGG"), ("TP Info");

INSERT INTO group_list_items (group_list_id, group_id)
VALUES (1, 1), (1, 2), (1, 3), (1, 4), (1, 5), (1, 6), (1, 7), (1, 8),
(2, 9), (2, 10), (2, 11), (3, 12), (3, 13);

INSERT INTO group_items (group_list_id, group_id, student_id)
VALUES (1, 1, 1), (1, 1, 2), (1, 1, 3), (1, 2, 4), (1, 2, 5), (1, 2, 6),
(1, 3, 7), (1, 3, 8), (1, 3, 9), (1, 4, 10), (1, 4, 11), (1, 4, 12),
(1, 5, 13), (1, 5, 14), (1, 5, 15), (1, 6, 16), (1, 6, 17), (1, 6, 18),
(1, 7, 19), (1, 7, 20), (1, 7, 21), (1, 8, 22), (1, 8, 23), (1, 8, 24),
(2, 9, 13), (2, 9, 14), (2, 9, 15), (2, 10, 16), (2, 10, 17), (2, 10, 18),
(2, 11, 9), (2, 11, 21),
(3, 12, 4), (3, 12, 5), (3, 12, 6), (3, 12, 10), (3, 12, 11), (3, 12, 12),
(3, 12, 16), (3, 12, 17), (3, 12, 18), (3, 12, 22), (3, 12, 23), (3, 12, 24),
(3, 13, 1), (3, 13, 2), (3, 13, 3), (3, 13, 7), (3, 13, 8), (3, 13, 9),
(3, 13, 13), (3, 13, 14), (3, 13, 15), (3, 13, 19), (3, 13, 20), (3, 13, 21);

INSERT INTO subject_groups (name, optional)
VALUES ("Spécialité", 0), ("LV1", 0), ("LV2", 1), ("Mathématiques", 0), ("Lettres-Philo", 0), ("TP Info", 1);

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

INSERT INTO subjects
(name, subject_group_id, incompat_id, group_list_id,
duration, min_students_per_group, max_students_per_group, period, period_is_strict,
is_tutorial, max_groups_per_slot, balance_teachers, balance_timeslots)
VALUES
("HGG", 1, NULL, 2, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("ESH", 1, 1, 1, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("Lettres-Philo", 5, NULL, 1, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("LV1 - Anglais", 3, NULL, 1, 60, 2, 3, 2, 0, 0, 1, 1, 1),
("LV2 - Espagnol", 2, 2, 1, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("LV2 - Allemand", 2, 3, 1, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("Mathématiques Approfondies", 4, NULL, 1, 60, 2, 3, 2, 0, 0, 1, 1, 1),
("TP Info", 6, NULL, 3, 120, 10, 19, 2, 0, 1, 1, 0, 0);
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
INSERT INTO student_subjects (student_id, subject_id)
VALUES (1, 2), (1, 3), (1, 4), (1, 5), (1, 7), (1, 8),
(2, 2), (2, 3), (2, 4), (2, 5), (2, 7), (2, 8),
(3, 2), (3, 3), (3, 4), (3, 5), (3, 7), (3, 8),
(4, 2), (4, 3), (4, 4), (4, 5), (4, 7), (4, 8),
(5, 2), (5, 3), (5, 4), (5, 5), (5, 7), (5, 8),
(6, 2), (6, 3), (6, 4), (6, 5), (6, 7), (6, 8),
(7, 2), (7, 3), (7, 4), (7, 6), (7, 7), (7, 8),
(8, 2), (8, 3), (8, 4), (8, 6), (8, 7), (8, 8),
(9, 1), (9, 3), (9, 4), (9, 6), (9, 7), (9, 8),
(10, 2), (10, 3), (10, 4), (10, 6), (10, 7), (10, 8),
(11, 2), (11, 3), (11, 4), (11, 6), (11, 7), (11, 8),
(12, 2), (12, 3), (12, 4), (12, 6), (12, 7), (12, 8),
(13, 1), (13, 3), (13, 4), (13, 7), (13, 8),
(14, 1), (14, 3), (14, 4), (14, 7), (14, 8),
(15, 1), (15, 3), (15, 4), (15, 7), (15, 8),
(16, 1), (16, 3), (16, 4), (16, 5), (16, 7), (16, 8),
(17, 1), (17, 3), (17, 4), (17, 5), (17, 7), (17, 8),
(18, 1), (18, 3), (18, 4), (18, 5), (18, 7), (18, 8),
(19, 2), (19, 3), (19, 4), (19, 5), (19, 7), (19, 8),
(20, 2), (20, 3), (20, 4), (20, 5), (20, 7), (20, 8),
(21, 1), (21, 3), (21, 4), (21, 5), (21, 7), (21, 8),
(22, 2), (22, 3), (22, 4), (22, 5), (22, 7),
(23, 2), (23, 3), (23, 4), (23, 5), (23, 7),
(24, 2), (24, 3), (24, 4), (24, 5), (24, 7);
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    store
}

#[sqlx::test]
async fn subject_group_for_student_get(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(5),
            super::super::subject_groups::Id(1),
        )
        .await
        .unwrap();

    assert_eq!(id, Some(super::super::subjects::Id(2)));

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(13),
            super::super::subject_groups::Id(1),
        )
        .await
        .unwrap();

    assert_eq!(id, Some(super::super::subjects::Id(1)));

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(20),
            super::super::subject_groups::Id(2),
        )
        .await
        .unwrap();

    assert_eq!(id, Some(super::super::subjects::Id(5)));

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(10),
            super::super::subject_groups::Id(2),
        )
        .await
        .unwrap();

    assert_eq!(id, Some(super::super::subjects::Id(6)));

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(14),
            super::super::subject_groups::Id(2),
        )
        .await
        .unwrap();

    assert_eq!(id, None);

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(10),
            super::super::subject_groups::Id(6),
        )
        .await
        .unwrap();

    assert_eq!(id, Some(super::super::subjects::Id(8)));

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(24),
            super::super::subject_groups::Id(6),
        )
        .await
        .unwrap();

    assert_eq!(id, None);
}

#[sqlx::test]
async fn subject_group_for_student_set(pool: sqlx::SqlitePool) {
    let mut store = prepare_example_db(pool).await;

    unsafe {
        store
            .subject_group_for_student_set_unchecked(
                super::super::students::Id(5),
                super::super::subject_groups::Id(1),
                Some(super::super::subjects::Id(1)),
            )
            .await
            .unwrap();
    }

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(5),
            super::super::subject_groups::Id(1),
        )
        .await
        .unwrap();

    assert_eq!(id, Some(super::super::subjects::Id(1)));

    unsafe {
        store
            .subject_group_for_student_set_unchecked(
                super::super::students::Id(10),
                super::super::subject_groups::Id(6),
                None,
            )
            .await
            .unwrap();
    }

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(10),
            super::super::subject_groups::Id(6),
        )
        .await
        .unwrap();

    assert_eq!(id, None);

    unsafe {
        store
            .subject_group_for_student_set_unchecked(
                super::super::students::Id(24),
                super::super::subject_groups::Id(6),
                Some(super::super::subjects::Id(8)),
            )
            .await
            .unwrap();
    }

    let id = store
        .subject_group_for_student_get(
            super::super::students::Id(24),
            super::super::subject_groups::Id(6),
        )
        .await
        .unwrap();

    assert_eq!(id, Some(super::super::subjects::Id(8)));
}
