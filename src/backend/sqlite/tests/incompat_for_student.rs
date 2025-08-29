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

INSERT INTO teachers (surname, firstname, contact)
VALUES
("DURAND", "Gontran", "gontran.durand@yahoo.fr"),
("BEAUREGARD", "Stéphane", "sbeauregard42@orange.fr"),
("RIVOUX", "Jérôme", "jejeriri@gmail.com"),
("DUPONT", "Suzanne", "s.dupont158@wanadoo.fr"),
("MORET", "Béatrice", "bebemoret@yahoo.fr"),
("SELLIER", "Geoffroy", "geoffroy.sellier@ac-lyon.fr"),
("WEBER", "Xavier", "xavier.weber@gmail.com"),
("LALLEMAND", "Gérard", "gerardlallemand@orange.fr"),
("VILLARD", "Josette", "josette.villard@ac-lyon.fr"),
("COLAS", "Filibustine", "fcolas@gmail.com"),
("HUARD", "Violette", "vhuard42@orange.fr"),
("RAMOS", "Camille", "camille.ramos@ac-lyon.fr"),
("MARTEAU", "Fabrice", "fmarteau73@orange.fr"),
("TOURNIER", "Alexandre", "alexandre.tournier@ac-lyon.fr"),
("VIGOUROUX", "Maud", "maud.vigouroux@orange.fr"),
("PEYRE", "Elisabeth", "epeyre@laposte.net"),
("DE SOUSA", "Gabriel", "gabidesousa42@yahoo.fr"),
("BUISSON", "Louise", "louise.buisson@ac-lyon.fr");

INSERT INTO time_slots
(subject_id, teacher_id, start_day, start_time, week_pattern_id, room)
VALUES
(1, 1, 3, 960, 1, ""), (1, 1, 3, 1020, 2, ""),
(2, 2, 1, 840, 1, ""), (2, 2, 1, 960, 1, ""), (2, 3, 3, 960, 1, ""), (2, 4, 3, 1020, 1, ""),
(3, 5, 0, 840, 1, ""), (3, 5, 3, 960, 1, ""), (3, 6, 3, 960, 1, ""), (3, 7, 2, 1020, 1, ""),
(4, 8, 0, 900, 1, ""), (4, 9, 0, 900, 1, ""), (4, 10, 2, 1020, 1, ""), (4, 11, 3, 1020, 1, ""),
(7, 12, 0, 840, 1, ""), (7, 13, 0, 840, 1, ""), (7, 14, 1, 960, 1, ""), (7, 14, 1, 1020, 1, ""),
(5, 15, 0, 840, 1, ""), (5, 16, 0, 900, 1, ""), (5, 17, 4, 1020, 1, ""),
(6, 18, 3, 780, 1, ""),
(8, 14, 2, 960, 1, "");
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
    let store = prepare_example_db(pool).await;

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
