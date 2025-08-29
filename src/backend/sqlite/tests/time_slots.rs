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

#[sqlx::test]
async fn time_slots_get_one_1(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    let time_slot = store
        .time_slots_get(super::super::time_slots::Id(1))
        .await
        .unwrap();

    let time_slot_expected = TimeSlot {
        subject_id: super::super::subjects::Id(1),
        teacher_id: super::super::teachers::Id(1),
        start: SlotStart {
            day: crate::time::Weekday::Thursday,
            time: crate::time::Time::from_hm(16, 0).unwrap(),
        },
        week_pattern_id: super::super::week_patterns::Id(1),
        room: String::from(""),
    };

    assert_eq!(time_slot, time_slot_expected);
}

#[sqlx::test]
async fn time_slots_get_one_2(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    let time_slot = store
        .time_slots_get(super::super::time_slots::Id(2))
        .await
        .unwrap();

    let time_slot_expected = TimeSlot {
        subject_id: super::super::subjects::Id(1),
        teacher_id: super::super::teachers::Id(1),
        start: SlotStart {
            day: crate::time::Weekday::Thursday,
            time: crate::time::Time::from_hm(17, 0).unwrap(),
        },
        week_pattern_id: super::super::week_patterns::Id(2),
        room: String::from(""),
    };

    assert_eq!(time_slot, time_slot_expected);
}

#[sqlx::test]
async fn time_slots_get_one_3(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    let time_slot = store
        .time_slots_get(super::super::time_slots::Id(15))
        .await
        .unwrap();

    let time_slot_expected = TimeSlot {
        subject_id: super::super::subjects::Id(7),
        teacher_id: super::super::teachers::Id(12),
        start: SlotStart {
            day: crate::time::Weekday::Monday,
            time: crate::time::Time::from_hm(14, 0).unwrap(),
        },
        week_pattern_id: super::super::week_patterns::Id(1),
        room: String::from(""),
    };

    assert_eq!(time_slot, time_slot_expected);
}

#[sqlx::test]
async fn time_slots_get_all(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    let time_slots = store.time_slots_get_all().await.unwrap();

    let time_slots_expected = BTreeMap::from([
        (
            super::super::time_slots::Id(1),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(2),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(2),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(3),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(4),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(5),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(3),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(6),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(7),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(8),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(9),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(6),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(10),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(7),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(11),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(8),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(12),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(9),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(13),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(10),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(14),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(11),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(15),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(12),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(16),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(13),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(17),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(18),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(19),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(15),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(20),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(16),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(21),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(17),
                start: SlotStart {
                    day: crate::time::Weekday::Friday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(22),
            TimeSlot {
                subject_id: super::super::subjects::Id(6),
                teacher_id: super::super::teachers::Id(18),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(13, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(23),
            TimeSlot {
                subject_id: super::super::subjects::Id(8),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
    ]);

    assert_eq!(time_slots, time_slots_expected);
}

#[sqlx::test]
async fn time_slots_remove_one(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    store
        .time_slots_remove(super::super::time_slots::Id(5))
        .await
        .unwrap();

    let time_slots = store.time_slots_get_all().await.unwrap();

    let time_slots_expected = BTreeMap::from([
        (
            super::super::time_slots::Id(1),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(2),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(2),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(3),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(4),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(6),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(7),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(8),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(9),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(6),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(10),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(7),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(11),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(8),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(12),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(9),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(13),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(10),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(14),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(11),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(15),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(12),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(16),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(13),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(17),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(18),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(19),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(15),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(20),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(16),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(21),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(17),
                start: SlotStart {
                    day: crate::time::Weekday::Friday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(22),
            TimeSlot {
                subject_id: super::super::subjects::Id(6),
                teacher_id: super::super::teachers::Id(18),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(13, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(23),
            TimeSlot {
                subject_id: super::super::subjects::Id(8),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
    ]);

    assert_eq!(time_slots, time_slots_expected);
}

#[sqlx::test]
async fn time_slots_remove_then_add(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    store
        .time_slots_remove(super::super::time_slots::Id(5))
        .await
        .unwrap();
    let id = store
        .time_slots_add(&TimeSlot {
            subject_id: super::super::subjects::Id(3),
            teacher_id: super::super::teachers::Id(4),
            start: SlotStart {
                day: crate::time::Weekday::Wednesday,
                time: crate::time::Time::from_hm(8, 0).unwrap(),
            },
            week_pattern_id: super::super::week_patterns::Id(3),
            room: String::from("Test"),
        })
        .await
        .unwrap();
    assert_eq!(id, super::super::time_slots::Id(24));

    let time_slots = store.time_slots_get_all().await.unwrap();

    let time_slots_expected = BTreeMap::from([
        (
            super::super::time_slots::Id(1),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(2),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(2),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(3),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(4),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(6),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(7),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(8),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(9),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(6),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(10),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(7),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(11),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(8),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(12),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(9),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(13),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(10),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(14),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(11),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(15),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(12),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(16),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(13),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(17),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(18),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(19),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(15),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(20),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(16),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(21),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(17),
                start: SlotStart {
                    day: crate::time::Weekday::Friday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(22),
            TimeSlot {
                subject_id: super::super::subjects::Id(6),
                teacher_id: super::super::teachers::Id(18),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(13, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(23),
            TimeSlot {
                subject_id: super::super::subjects::Id(8),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(24),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(8, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(3),
                room: String::from("Test"),
            },
        ),
    ]);

    assert_eq!(time_slots, time_slots_expected);
}

#[sqlx::test]
async fn time_slots_update(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    store
        .time_slots_update(
            super::super::time_slots::Id(5),
            &TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(8, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(3),
                room: String::from("Test"),
            },
        )
        .await
        .unwrap();

    let time_slots = store.time_slots_get_all().await.unwrap();

    let time_slots_expected = BTreeMap::from([
        (
            super::super::time_slots::Id(1),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(2),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(2),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(3),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(4),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(5),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(8, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(3),
                room: String::from("Test"),
            },
        ),
        (
            super::super::time_slots::Id(6),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(7),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(8),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(9),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(6),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(10),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(7),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(11),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(8),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(12),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(9),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(13),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(10),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(14),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(11),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(15),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(12),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(16),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(13),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(17),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(18),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(19),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(15),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(20),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(16),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(21),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(17),
                start: SlotStart {
                    day: crate::time::Weekday::Friday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(22),
            TimeSlot {
                subject_id: super::super::subjects::Id(6),
                teacher_id: super::super::teachers::Id(18),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(13, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(23),
            TimeSlot {
                subject_id: super::super::subjects::Id(8),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
    ]);

    assert_eq!(time_slots, time_slots_expected);
}

#[sqlx::test]
async fn time_slots_add_one(pool: sqlx::SqlitePool) {
    let store = prepare_example_db(pool).await;

    let id = store
        .time_slots_add(&TimeSlot {
            subject_id: super::super::subjects::Id(3),
            teacher_id: super::super::teachers::Id(4),
            start: SlotStart {
                day: crate::time::Weekday::Wednesday,
                time: crate::time::Time::from_hm(8, 0).unwrap(),
            },
            week_pattern_id: super::super::week_patterns::Id(3),
            room: String::from("Test"),
        })
        .await
        .unwrap();
    assert_eq!(id, super::super::time_slots::Id(24));

    let time_slots = store.time_slots_get_all().await.unwrap();

    let time_slots_expected = BTreeMap::from([
        (
            super::super::time_slots::Id(1),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(2),
            TimeSlot {
                subject_id: super::super::subjects::Id(1),
                teacher_id: super::super::teachers::Id(1),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(2),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(3),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(4),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(2),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(5),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(3),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(6),
            TimeSlot {
                subject_id: super::super::subjects::Id(2),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(7),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(8),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(5),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(9),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(6),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(10),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(7),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(11),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(8),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(12),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(9),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(13),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(10),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(14),
            TimeSlot {
                subject_id: super::super::subjects::Id(4),
                teacher_id: super::super::teachers::Id(11),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(15),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(12),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(16),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(13),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(17),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(18),
            TimeSlot {
                subject_id: super::super::subjects::Id(7),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Tuesday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(19),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(15),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(14, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(20),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(16),
                start: SlotStart {
                    day: crate::time::Weekday::Monday,
                    time: crate::time::Time::from_hm(15, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(21),
            TimeSlot {
                subject_id: super::super::subjects::Id(5),
                teacher_id: super::super::teachers::Id(17),
                start: SlotStart {
                    day: crate::time::Weekday::Friday,
                    time: crate::time::Time::from_hm(17, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(22),
            TimeSlot {
                subject_id: super::super::subjects::Id(6),
                teacher_id: super::super::teachers::Id(18),
                start: SlotStart {
                    day: crate::time::Weekday::Thursday,
                    time: crate::time::Time::from_hm(13, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(23),
            TimeSlot {
                subject_id: super::super::subjects::Id(8),
                teacher_id: super::super::teachers::Id(14),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(16, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(1),
                room: String::from(""),
            },
        ),
        (
            super::super::time_slots::Id(24),
            TimeSlot {
                subject_id: super::super::subjects::Id(3),
                teacher_id: super::super::teachers::Id(4),
                start: SlotStart {
                    day: crate::time::Weekday::Wednesday,
                    time: crate::time::Time::from_hm(8, 0).unwrap(),
                },
                week_pattern_id: super::super::week_patterns::Id(3),
                room: String::from("Test"),
            },
        ),
    ]);

    assert_eq!(time_slots, time_slots_expected);
}
