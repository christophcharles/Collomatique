use super::*;

async fn build_example_group_list(pool: sqlx::SqlitePool) -> Store {
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
("5", 0), ("6", 0), ("3+7", 1), ("P", 1), ("I", 1);

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
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    store
}

#[sqlx::test]
async fn group_lists_get_one_1(pool: sqlx::SqlitePool) {
    let store = build_example_group_list(pool).await;

    let group_list = store
        .group_lists_get(super::super::group_lists::Id(2))
        .await
        .unwrap();

    let expected_result = GroupList {
        name: String::from("HGG"),
        groups: vec![
            Group {
                name: String::from("5"),
                extendable: false,
            },
            Group {
                name: String::from("6"),
                extendable: false,
            },
            Group {
                name: String::from("3+7"),
                extendable: true,
            },
        ],
        students_mapping: BTreeMap::from([
            (super::super::students::Id(13), 0),
            (super::super::students::Id(14), 0),
            (super::super::students::Id(15), 0),
            (super::super::students::Id(16), 1),
            (super::super::students::Id(17), 1),
            (super::super::students::Id(18), 1),
            (super::super::students::Id(9), 2),
            (super::super::students::Id(21), 2),
        ]),
    };

    assert_eq!(group_list, expected_result);
}

#[sqlx::test]
async fn group_lists_get_one_2(pool: sqlx::SqlitePool) {
    let store = build_example_group_list(pool).await;

    let group_list = store
        .group_lists_get(super::super::group_lists::Id(3))
        .await
        .unwrap();

    let expected_result = GroupList {
        name: String::from("TP Info"),
        groups: vec![
            Group {
                name: String::from("P"),
                extendable: true,
            },
            Group {
                name: String::from("I"),
                extendable: true,
            },
        ],
        students_mapping: BTreeMap::from([
            (super::super::students::Id(1), 1),
            (super::super::students::Id(2), 1),
            (super::super::students::Id(3), 1),
            (super::super::students::Id(4), 0),
            (super::super::students::Id(5), 0),
            (super::super::students::Id(6), 0),
            (super::super::students::Id(7), 1),
            (super::super::students::Id(8), 1),
            (super::super::students::Id(9), 1),
            (super::super::students::Id(10), 0),
            (super::super::students::Id(11), 0),
            (super::super::students::Id(12), 0),
            (super::super::students::Id(13), 1),
            (super::super::students::Id(14), 1),
            (super::super::students::Id(15), 1),
            (super::super::students::Id(16), 0),
            (super::super::students::Id(17), 0),
            (super::super::students::Id(18), 0),
            (super::super::students::Id(19), 1),
            (super::super::students::Id(20), 1),
            (super::super::students::Id(21), 1),
            (super::super::students::Id(22), 0),
            (super::super::students::Id(23), 0),
            (super::super::students::Id(24), 0),
        ]),
    };

    assert_eq!(group_list, expected_result);
}

#[sqlx::test]
async fn group_lists_get_one_3(pool: sqlx::SqlitePool) {
    let store = build_example_group_list(pool).await;

    let group_list = store
        .group_lists_get(super::super::group_lists::Id(1))
        .await
        .unwrap();

    let expected_result = GroupList {
        name: String::from("Groupes"),
        groups: vec![
            Group {
                name: String::from("1"),
                extendable: false,
            },
            Group {
                name: String::from("2"),
                extendable: false,
            },
            Group {
                name: String::from("3"),
                extendable: false,
            },
            Group {
                name: String::from("4"),
                extendable: false,
            },
            Group {
                name: String::from("5"),
                extendable: false,
            },
            Group {
                name: String::from("6"),
                extendable: false,
            },
            Group {
                name: String::from("7"),
                extendable: false,
            },
            Group {
                name: String::from("8"),
                extendable: false,
            },
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
            (super::super::students::Id(13), 4),
            (super::super::students::Id(14), 4),
            (super::super::students::Id(15), 4),
            (super::super::students::Id(16), 5),
            (super::super::students::Id(17), 5),
            (super::super::students::Id(18), 5),
            (super::super::students::Id(19), 6),
            (super::super::students::Id(20), 6),
            (super::super::students::Id(21), 6),
            (super::super::students::Id(22), 7),
            (super::super::students::Id(23), 7),
            (super::super::students::Id(24), 7),
        ]),
    };

    assert_eq!(group_list, expected_result);
}

#[sqlx::test]
async fn group_lists_get_all(pool: sqlx::SqlitePool) {
    let store = build_example_group_list(pool).await;

    let group_lists = store.group_lists_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::group_lists::Id(1),
            GroupList {
                name: String::from("Groupes"),
                groups: vec![
                    Group {
                        name: String::from("1"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("2"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("3"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("4"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("5"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("6"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("7"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("8"),
                        extendable: false,
                    },
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
                    (super::super::students::Id(13), 4),
                    (super::super::students::Id(14), 4),
                    (super::super::students::Id(15), 4),
                    (super::super::students::Id(16), 5),
                    (super::super::students::Id(17), 5),
                    (super::super::students::Id(18), 5),
                    (super::super::students::Id(19), 6),
                    (super::super::students::Id(20), 6),
                    (super::super::students::Id(21), 6),
                    (super::super::students::Id(22), 7),
                    (super::super::students::Id(23), 7),
                    (super::super::students::Id(24), 7),
                ]),
            },
        ),
        (
            super::super::group_lists::Id(2),
            GroupList {
                name: String::from("HGG"),
                groups: vec![
                    Group {
                        name: String::from("5"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("6"),
                        extendable: false,
                    },
                    Group {
                        name: String::from("3+7"),
                        extendable: true,
                    },
                ],
                students_mapping: BTreeMap::from([
                    (super::super::students::Id(13), 0),
                    (super::super::students::Id(14), 0),
                    (super::super::students::Id(15), 0),
                    (super::super::students::Id(16), 1),
                    (super::super::students::Id(17), 1),
                    (super::super::students::Id(18), 1),
                    (super::super::students::Id(9), 2),
                    (super::super::students::Id(21), 2),
                ]),
            },
        ),
        (
            super::super::group_lists::Id(3),
            GroupList {
                name: String::from("TP Info"),
                groups: vec![
                    Group {
                        name: String::from("P"),
                        extendable: true,
                    },
                    Group {
                        name: String::from("I"),
                        extendable: true,
                    },
                ],
                students_mapping: BTreeMap::from([
                    (super::super::students::Id(1), 1),
                    (super::super::students::Id(2), 1),
                    (super::super::students::Id(3), 1),
                    (super::super::students::Id(4), 0),
                    (super::super::students::Id(5), 0),
                    (super::super::students::Id(6), 0),
                    (super::super::students::Id(7), 1),
                    (super::super::students::Id(8), 1),
                    (super::super::students::Id(9), 1),
                    (super::super::students::Id(10), 0),
                    (super::super::students::Id(11), 0),
                    (super::super::students::Id(12), 0),
                    (super::super::students::Id(13), 1),
                    (super::super::students::Id(14), 1),
                    (super::super::students::Id(15), 1),
                    (super::super::students::Id(16), 0),
                    (super::super::students::Id(17), 0),
                    (super::super::students::Id(18), 0),
                    (super::super::students::Id(19), 1),
                    (super::super::students::Id(20), 1),
                    (super::super::students::Id(21), 1),
                    (super::super::students::Id(22), 0),
                    (super::super::students::Id(23), 0),
                    (super::super::students::Id(24), 0),
                ]),
            },
        ),
    ]);

    assert_eq!(group_lists, expected_result);
}
