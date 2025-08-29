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
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    store
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SubjectDb {
    subject_id: i64,
    name: String,
    subject_group_id: i64,
    duration: i64,
    incompat_id: Option<i64>,
    min_students_per_group: i64,
    max_students_per_group: i64,
    period: i64,
    period_is_strict: i64,
    is_tutorial: i64,
    max_groups_per_slot: i64,
    balance_teachers: i64,
    balance_timeslots: i64,
    group_list_id: Option<i64>,
}

#[sqlx::test]
async fn subjects_add_one_1(pool: sqlx::SqlitePool) {
    let store = prepare_db(pool).await;

    let id = store
        .subjects_add(&Subject {
            name: String::from("HGG"),
            subject_group_id: super::super::subject_groups::Id(1),
            duration: NonZeroU32::new(60).unwrap(),
            incompat_id: None,
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            balancing_requirements: BalancingRequirements {
                teachers: false,
                timeslots: false,
            },
            group_list_id: Some(super::super::group_lists::Id(2)),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::subjects::Id(1));

    let subjects = sqlx::query_as!(SubjectDb, "SELECT * FROM subjects")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let subjects_expected = vec![SubjectDb {
        subject_id: 1,
        name: String::from("HGG"),
        subject_group_id: 1,
        duration: 60,
        incompat_id: None,
        min_students_per_group: 2,
        max_students_per_group: 3,
        period: 2,
        period_is_strict: 0,
        is_tutorial: 0,
        max_groups_per_slot: 1,
        balance_teachers: 0,
        balance_timeslots: 0,
        group_list_id: Some(2),
    }];

    assert_eq!(subjects, subjects_expected);
}

#[sqlx::test]
async fn subjects_add_one_2(pool: sqlx::SqlitePool) {
    let store = prepare_db(pool).await;

    let id = store
        .subjects_add(&Subject {
            name: String::from("ESH"),
            subject_group_id: super::super::subject_groups::Id(1),
            duration: NonZeroU32::new(60).unwrap(),
            incompat_id: Some(super::super::incompats::Id(1)),
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            balancing_requirements: BalancingRequirements {
                teachers: false,
                timeslots: false,
            },
            group_list_id: Some(super::super::group_lists::Id(1)),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::subjects::Id(1));

    let subjects = sqlx::query_as!(SubjectDb, "SELECT * FROM subjects")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let subjects_expected = vec![SubjectDb {
        subject_id: 1,
        name: String::from("ESH"),
        subject_group_id: 1,
        duration: 60,
        incompat_id: Some(1),
        min_students_per_group: 2,
        max_students_per_group: 3,
        period: 2,
        period_is_strict: 0,
        is_tutorial: 0,
        max_groups_per_slot: 1,
        balance_teachers: 0,
        balance_timeslots: 0,
        group_list_id: Some(1),
    }];

    assert_eq!(subjects, subjects_expected);
}

#[sqlx::test]
async fn subjects_add_multiple(pool: sqlx::SqlitePool) {
    let store = prepare_db(pool).await;

    let id = store
        .subjects_add(&Subject {
            name: String::from("HGG"),
            subject_group_id: super::super::subject_groups::Id(1),
            duration: NonZeroU32::new(60).unwrap(),
            incompat_id: None,
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            balancing_requirements: BalancingRequirements {
                teachers: false,
                timeslots: false,
            },
            group_list_id: Some(super::super::group_lists::Id(2)),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::subjects::Id(1));

    let id = store
        .subjects_add(&Subject {
            name: String::from("ESH"),
            subject_group_id: super::super::subject_groups::Id(1),
            duration: NonZeroU32::new(60).unwrap(),
            incompat_id: Some(super::super::incompats::Id(1)),
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            balancing_requirements: BalancingRequirements {
                teachers: false,
                timeslots: false,
            },
            group_list_id: Some(super::super::group_lists::Id(1)),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::subjects::Id(2));

    let id = store
        .subjects_add(&Subject {
            name: String::from("Lettres-Philo"),
            subject_group_id: super::super::subject_groups::Id(5),
            duration: NonZeroU32::new(60).unwrap(),
            incompat_id: None,
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            balancing_requirements: BalancingRequirements {
                teachers: false,
                timeslots: false,
            },
            group_list_id: Some(super::super::group_lists::Id(1)),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::subjects::Id(3));

    let id = store
        .subjects_add(&Subject {
            name: String::from("TP Info"),
            subject_group_id: super::super::subject_groups::Id(6),
            duration: NonZeroU32::new(120).unwrap(),
            incompat_id: None,
            students_per_group: NonZeroUsize::new(10).unwrap()..=NonZeroUsize::new(19).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: true,
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            balancing_requirements: BalancingRequirements {
                teachers: false,
                timeslots: false,
            },
            group_list_id: Some(super::super::group_lists::Id(3)),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::subjects::Id(4));

    let subjects = sqlx::query_as!(SubjectDb, "SELECT * FROM subjects")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let subjects_expected = vec![
        SubjectDb {
            subject_id: 1,
            name: String::from("HGG"),
            subject_group_id: 1,
            duration: 60,
            incompat_id: None,
            min_students_per_group: 2,
            max_students_per_group: 3,
            period: 2,
            period_is_strict: 0,
            is_tutorial: 0,
            max_groups_per_slot: 1,
            balance_teachers: 0,
            balance_timeslots: 0,
            group_list_id: Some(2),
        },
        SubjectDb {
            subject_id: 2,
            name: String::from("ESH"),
            subject_group_id: 1,
            duration: 60,
            incompat_id: Some(1),
            min_students_per_group: 2,
            max_students_per_group: 3,
            period: 2,
            period_is_strict: 0,
            is_tutorial: 0,
            max_groups_per_slot: 1,
            balance_teachers: 0,
            balance_timeslots: 0,
            group_list_id: Some(1),
        },
        SubjectDb {
            subject_id: 3,
            name: String::from("Lettres-Philo"),
            subject_group_id: 5,
            duration: 60,
            incompat_id: None,
            min_students_per_group: 2,
            max_students_per_group: 3,
            period: 2,
            period_is_strict: 0,
            is_tutorial: 0,
            max_groups_per_slot: 1,
            balance_teachers: 0,
            balance_timeslots: 0,
            group_list_id: Some(1),
        },
        SubjectDb {
            subject_id: 4,
            name: String::from("TP Info"),
            subject_group_id: 6,
            duration: 120,
            incompat_id: None,
            min_students_per_group: 10,
            max_students_per_group: 19,
            period: 2,
            period_is_strict: 0,
            is_tutorial: 1,
            max_groups_per_slot: 1,
            balance_teachers: 0,
            balance_timeslots: 0,
            group_list_id: Some(3),
        },
    ];

    assert_eq!(subjects, subjects_expected);
}
