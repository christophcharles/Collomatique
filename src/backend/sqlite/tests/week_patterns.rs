use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct WeekPatternDb {
    week_pattern_id: i64,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WeekDb {
    week_pattern_id: i64,
    week: i64,
}

#[sqlx::test]
async fn week_pattern_add_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let weeks = (0..20).into_iter().step_by(2).map(|x| Week(x)).collect();

    let _id = store
        .week_patterns_add(WeekPattern {
            name: String::from("Impaires"),
            weeks,
        })
        .await
        .unwrap();

    let week_patterns = sqlx::query_as!(WeekPatternDb, "SELECT * FROM week_patterns")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let weeks = sqlx::query_as!(WeekDb, "SELECT * FROM weeks")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let week_patterns_expected = vec![WeekPatternDb {
        week_pattern_id: 1,
        name: String::from("Impaires"),
    }];

    assert_eq!(week_patterns, week_patterns_expected);

    let weeks_expected = vec![
        WeekDb {
            week_pattern_id: 1,
            week: 0,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 2,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 4,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 6,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 8,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 10,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 12,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 14,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 16,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 18,
        },
    ];

    assert_eq!(weeks, weeks_expected);
}

#[sqlx::test]
async fn week_pattern_add_multiple(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let weeks = (0..20).into_iter().map(|x| Week(x)).collect();

    let _id = store
        .week_patterns_add(WeekPattern {
            name: String::from("Toutes"),
            weeks,
        })
        .await
        .unwrap();

    let weeks = (0..20).into_iter().step_by(2).map(|x| Week(x)).collect();

    let _id = store
        .week_patterns_add(WeekPattern {
            name: String::from("Impaires"),
            weeks,
        })
        .await
        .unwrap();

    let weeks = (0..20)
        .into_iter()
        .skip(1)
        .step_by(2)
        .map(|x| Week(x))
        .collect();

    let _id = store
        .week_patterns_add(WeekPattern {
            name: String::from("Paires"),
            weeks,
        })
        .await
        .unwrap();

    let week_patterns = sqlx::query_as!(WeekPatternDb, "SELECT * FROM week_patterns")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let weeks = sqlx::query_as!(WeekDb, "SELECT * FROM weeks")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let week_patterns_expected = vec![
        WeekPatternDb {
            week_pattern_id: 1,
            name: String::from("Toutes"),
        },
        WeekPatternDb {
            week_pattern_id: 2,
            name: String::from("Impaires"),
        },
        WeekPatternDb {
            week_pattern_id: 3,
            name: String::from("Paires"),
        },
    ];

    assert_eq!(week_patterns, week_patterns_expected);

    let weeks_expected = vec![
        WeekDb {
            week_pattern_id: 1,
            week: 0,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 1,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 2,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 3,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 4,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 5,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 6,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 7,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 8,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 9,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 10,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 11,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 12,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 13,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 14,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 15,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 16,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 17,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 18,
        },
        WeekDb {
            week_pattern_id: 1,
            week: 19,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 0,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 2,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 4,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 6,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 8,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 10,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 12,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 14,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 16,
        },
        WeekDb {
            week_pattern_id: 2,
            week: 18,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 1,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 3,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 5,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 7,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 9,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 11,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 13,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 15,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 17,
        },
        WeekDb {
            week_pattern_id: 3,
            week: 19,
        },
    ];

    assert_eq!(weeks, weeks_expected);
}

#[sqlx::test]
async fn week_pattern_get_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);
        "#
    ).execute(&store.pool).await.unwrap();

    let pattern = store
        .week_patterns_get(super::super::week_patterns::Id(2))
        .await
        .unwrap();

    let expected_result = WeekPattern {
        name: String::from("Impaires"),
        weeks: BTreeSet::from([Week(0), Week(2), Week(4), Week(6), Week(8)]),
    };

    assert_eq!(pattern, expected_result);
}

#[sqlx::test]
async fn week_pattern_get_all(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);
        "#
    ).execute(&store.pool).await.unwrap();

    let result = store.week_patterns_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::week_patterns::Id(1),
            WeekPattern {
                name: String::from("Toutes"),
                weeks: BTreeSet::from([
                    Week(0),
                    Week(1),
                    Week(2),
                    Week(3),
                    Week(4),
                    Week(5),
                    Week(6),
                    Week(7),
                    Week(8),
                    Week(9),
                ]),
            },
        ),
        (
            super::super::week_patterns::Id(2),
            WeekPattern {
                name: String::from("Impaires"),
                weeks: BTreeSet::from([Week(0), Week(2), Week(4), Week(6), Week(8)]),
            },
        ),
        (
            super::super::week_patterns::Id(3),
            WeekPattern {
                name: String::from("Paires"),
                weeks: BTreeSet::from([Week(1), Week(3), Week(5), Week(7), Week(9)]),
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn week_pattern_remove_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);
        "#
    ).execute(&store.pool).await.unwrap();

    store
        .week_patterns_remove(super::super::week_patterns::Id(2))
        .await
        .unwrap();

    let result = store.week_patterns_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::week_patterns::Id(1),
            WeekPattern {
                name: String::from("Toutes"),
                weeks: BTreeSet::from([
                    Week(0),
                    Week(1),
                    Week(2),
                    Week(3),
                    Week(4),
                    Week(5),
                    Week(6),
                    Week(7),
                    Week(8),
                    Week(9),
                ]),
            },
        ),
        (
            super::super::week_patterns::Id(3),
            WeekPattern {
                name: String::from("Paires"),
                weeks: BTreeSet::from([Week(1), Week(3), Week(5), Week(7), Week(9)]),
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}

#[sqlx::test]
async fn week_pattern_remove_then_add(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);
        "#
    ).execute(&store.pool).await.unwrap();

    store
        .week_patterns_remove(super::super::week_patterns::Id(2))
        .await
        .unwrap();

    let id = store
        .week_patterns_add(WeekPattern {
            name: String::from("Impaires"),
            weeks: BTreeSet::from([Week(0), Week(2), Week(4), Week(6), Week(8)]),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::week_patterns::Id(4));

    let result = store.week_patterns_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::week_patterns::Id(1),
            WeekPattern {
                name: String::from("Toutes"),
                weeks: BTreeSet::from([
                    Week(0),
                    Week(1),
                    Week(2),
                    Week(3),
                    Week(4),
                    Week(5),
                    Week(6),
                    Week(7),
                    Week(8),
                    Week(9),
                ]),
            },
        ),
        (
            super::super::week_patterns::Id(3),
            WeekPattern {
                name: String::from("Paires"),
                weeks: BTreeSet::from([Week(1), Week(3), Week(5), Week(7), Week(9)]),
            },
        ),
        (
            super::super::week_patterns::Id(4),
            WeekPattern {
                name: String::from("Impaires"),
                weeks: BTreeSet::from([Week(0), Week(2), Week(4), Week(6), Week(8)]),
            },
        ),
    ]);

    assert_eq!(result, expected_result);
}
