use super::*;

async fn prepare_db(pool: sqlx::SqlitePool) -> Store {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO students (surname, firstname, no_consecutive_slots)
VALUES ("Roth", "", 0), ("Marin", "", 0), ("Bordes", "", 0), ("Bresson", "", 0), ("Gosset","", 0),
("Martel", "", 0), ("Delarue", "", 0), ("Chauvet", "", 0), ("Bourdon", "", 0), ("Lafond", "", 0),
("Rondeau", "", 0), ("Vigneron", "", 0), ("Davy", "", 0), ("Gosselin", "", 0), ("Jeannin", "", 0),
("Sicard", "", 0), ("Mounier", "", 0), ("Lafon", "", 0), ("Brun", "", 0), ("Hardy", "", 0),
("Girault", "", 0), ("Delahaye", "", 0), ("Levasseur", "", 0), ("Gonthier", "", 0);

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

INSERT INTO subject_groups (name, optional)
VALUES ("Spécialité", 0), ("LV1", 0), ("LV2", 1), ("Mathématiques", 0), ("Lettres-Philo", 0), ("TP Info", 1);

INSERT INTO subjects
(name, subject_group_id, incompat_id, group_list_id,
duration, min_students_per_group, max_students_per_group, period, period_is_strict,
is_tutorial, max_groups_per_slot, balancing_constraints, balancing_slot_selections)
VALUES
("HGG", 1, NULL, 2, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("ESH", 1, 1, 1, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("Lettres-Philo", 5, NULL, 1, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("LV1 - Anglais", 3, NULL, 1, 60, 2, 3, 2, 0, 0, 1, 3, 0),
("LV2 - Espagnol", 2, 2, 1, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("LV2 - Allemand", 2, 3, 1, 60, 2, 3, 2, 0, 0, 1, 0, 0),
("Mathématiques Approfondies", 4, NULL, 1, 60, 2, 3, 2, 0, 0, 1, 3, 0),
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

#[sqlx::test]
async fn simple_test(pool: sqlx::SqlitePool) {
    let mut store = prepare_db(pool).await;

    let colloscope1 = Colloscope {
        name: "Colloscope1".to_string(),
        subjects: BTreeMap::from([(
            super::super::subjects::Id(1),
            ColloscopeSubject {
                time_slots: Vec::from([
                    ColloscopeTimeSlot {
                        teacher_id: super::super::teachers::Id(1),
                        start: SlotStart {
                            day: crate::time::Weekday::Monday,
                            time: crate::time::Time::from_hm(16, 0).unwrap(),
                        },
                        room: "1".to_string(),
                        group_assignments: BTreeMap::from([
                            (Week::new(0), BTreeSet::from([0])),
                            (Week::new(1), BTreeSet::from([1])),
                        ]),
                    },
                    ColloscopeTimeSlot {
                        teacher_id: super::super::teachers::Id(1),
                        start: SlotStart {
                            day: crate::time::Weekday::Tuesday,
                            time: crate::time::Time::from_hm(18, 0).unwrap(),
                        },
                        room: "2".to_string(),
                        group_assignments: BTreeMap::from([
                            (Week::new(0), BTreeSet::from([2])),
                            (Week::new(1), BTreeSet::from([3])),
                        ]),
                    },
                ]),
                group_list: ColloscopeGroupList {
                    name: "HGG".to_string(),
                    groups: vec![
                        "A".to_string(),
                        "B".to_string(),
                        "C".to_string(),
                        "D".to_string(),
                    ],
                    students_mapping: BTreeMap::from([
                        (super::super::students::Id(1), 0),
                        (super::super::students::Id(2), 0),
                        (super::super::students::Id(3), 0),
                        (super::super::students::Id(4), 1),
                        (super::super::students::Id(5), 1),
                        (super::super::students::Id(6), 1),
                        (super::super::students::Id(7), 2),
                        (super::super::students::Id(8), 2),
                        (super::super::students::Id(9), 2),
                        (super::super::students::Id(10), 3),
                        (super::super::students::Id(11), 3),
                        (super::super::students::Id(12), 3),
                    ]),
                },
            },
        )]),
    };

    let colloscope2 = Colloscope {
        name: "Colloscope2".to_string(),
        subjects: BTreeMap::from([(
            super::super::subjects::Id(2),
            ColloscopeSubject {
                time_slots: Vec::from([
                    ColloscopeTimeSlot {
                        teacher_id: super::super::teachers::Id(2),
                        start: SlotStart {
                            day: crate::time::Weekday::Wednesday,
                            time: crate::time::Time::from_hm(14, 0).unwrap(),
                        },
                        room: "3".to_string(),
                        group_assignments: BTreeMap::from([
                            (Week::new(0), BTreeSet::from([0])),
                            (Week::new(1), BTreeSet::from([1])),
                        ]),
                    },
                    ColloscopeTimeSlot {
                        teacher_id: super::super::teachers::Id(2),
                        start: SlotStart {
                            day: crate::time::Weekday::Wednesday,
                            time: crate::time::Time::from_hm(15, 0).unwrap(),
                        },
                        room: "3".to_string(),
                        group_assignments: BTreeMap::from([
                            (Week::new(0), BTreeSet::from([2])),
                            (Week::new(1), BTreeSet::from([3])),
                        ]),
                    },
                ]),
                group_list: ColloscopeGroupList {
                    name: "HGG".to_string(),
                    groups: vec![
                        "A".to_string(),
                        "B".to_string(),
                        "C".to_string(),
                        "D".to_string(),
                    ],
                    students_mapping: BTreeMap::from([
                        (super::super::students::Id(13), 0),
                        (super::super::students::Id(14), 0),
                        (super::super::students::Id(15), 0),
                        (super::super::students::Id(16), 1),
                        (super::super::students::Id(17), 1),
                        (super::super::students::Id(18), 1),
                        (super::super::students::Id(19), 2),
                        (super::super::students::Id(20), 2),
                        (super::super::students::Id(21), 2),
                        (super::super::students::Id(22), 3),
                        (super::super::students::Id(23), 3),
                        (super::super::students::Id(24), 3),
                    ]),
                },
            },
        )]),
    };

    let colloscope1_id = unsafe { store.colloscopes_add_unchecked(&colloscope1) }
        .await
        .unwrap();

    let colloscope2_id = unsafe { store.colloscopes_add_unchecked(&colloscope2) }
        .await
        .unwrap();

    let expected_result =
        BTreeMap::from([(colloscope1_id, colloscope1), (colloscope2_id, colloscope2)]);

    let result = store.colloscopes_get_all().await.unwrap();

    assert_eq!(result, expected_result);
}
