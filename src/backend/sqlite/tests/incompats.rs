use super::*;

#[sqlx::test]
async fn incompats_get_one_1(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);

INSERT INTO incompats (name, max_count) VALUES ("LV2 - Allemand", 0), ("Repas midi - lundi", 2);
INSERT INTO incompat_groups (incompat_id) VALUES (1), (1), (2), (2), (2);
INSERT INTO incompat_group_items (incompat_group_id, week_pattern_id, start_day, start_time, duration)
VALUES (1,2,0,480,60), (1,2,2,720,60), (2,3,4,840,120), (3,1,0,660,60), (4,1,0,720,60), (5,1,0,780,60);
        "#
    ).execute(&store.pool).await.unwrap();

    let incompat = store
        .incompats_get(super::super::incompats::Id(2))
        .await
        .unwrap();

    let expected_result = Incompat {
        name: String::from("Repas midi - lundi"),
        max_count: 2,
        groups: BTreeSet::from([
            IncompatGroup {
                slots: BTreeSet::from([IncompatSlot {
                    week_pattern_id: super::super::week_patterns::Id(1),
                    start: TimeSlot {
                        day: crate::time::Weekday::Monday,
                        time: crate::time::Time::from_hm(11, 0).unwrap(),
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                }]),
            },
            IncompatGroup {
                slots: BTreeSet::from([IncompatSlot {
                    week_pattern_id: super::super::week_patterns::Id(1),
                    start: TimeSlot {
                        day: crate::time::Weekday::Monday,
                        time: crate::time::Time::from_hm(12, 0).unwrap(),
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                }]),
            },
            IncompatGroup {
                slots: BTreeSet::from([IncompatSlot {
                    week_pattern_id: super::super::week_patterns::Id(1),
                    start: TimeSlot {
                        day: crate::time::Weekday::Monday,
                        time: crate::time::Time::from_hm(13, 0).unwrap(),
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                }]),
            },
        ]),
    };

    assert_eq!(incompat, expected_result);
}

#[sqlx::test]
async fn incompats_get_one_2(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);

INSERT INTO incompats (name, max_count) VALUES ("LV2 - Allemand", 0), ("Repas midi - lundi", 2);
INSERT INTO incompat_groups (incompat_id) VALUES (1), (1), (2), (2), (2);
INSERT INTO incompat_group_items (incompat_group_id, week_pattern_id, start_day, start_time, duration)
VALUES (1,2,0,480,60), (1,2,2,720,60), (2,3,4,840,120), (3,1,0,660,60), (4,1,0,720,60), (5,1,0,780,60);
        "#
    ).execute(&store.pool).await.unwrap();

    let incompat = store
        .incompats_get(super::super::incompats::Id(1))
        .await
        .unwrap();

    let expected_result = Incompat {
        name: String::from("LV2 - Allemand"),
        max_count: 0,
        groups: BTreeSet::from([
            IncompatGroup {
                slots: BTreeSet::from([
                    IncompatSlot {
                        week_pattern_id: super::super::week_patterns::Id(2),
                        start: TimeSlot {
                            day: crate::time::Weekday::Monday,
                            time: crate::time::Time::from_hm(8, 0).unwrap(),
                        },
                        duration: NonZeroU32::new(60).unwrap(),
                    },
                    IncompatSlot {
                        week_pattern_id: super::super::week_patterns::Id(2),
                        start: TimeSlot {
                            day: crate::time::Weekday::Wednesday,
                            time: crate::time::Time::from_hm(12, 0).unwrap(),
                        },
                        duration: NonZeroU32::new(60).unwrap(),
                    },
                ]),
            },
            IncompatGroup {
                slots: BTreeSet::from([IncompatSlot {
                    week_pattern_id: super::super::week_patterns::Id(3),
                    start: TimeSlot {
                        day: crate::time::Weekday::Friday,
                        time: crate::time::Time::from_hm(14, 0).unwrap(),
                    },
                    duration: NonZeroU32::new(120).unwrap(),
                }]),
            },
        ]),
    };

    assert_eq!(incompat, expected_result);
}

#[sqlx::test]
async fn incompats_get_all(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);

INSERT INTO incompats (name, max_count) VALUES ("LV2 - Allemand", 0), ("Repas midi - lundi", 2);
INSERT INTO incompat_groups (incompat_id) VALUES (1), (1), (2), (2), (2);
INSERT INTO incompat_group_items (incompat_group_id, week_pattern_id, start_day, start_time, duration)
VALUES (1,2,0,480,60), (1,2,2,720,60), (2,3,4,840,120), (3,1,0,660,60), (4,1,0,720,60), (5,1,0,780,60);
        "#
    ).execute(&store.pool).await.unwrap();

    let incompats = store.incompats_get_all().await.unwrap();

    let expected_result = BTreeMap::from([
        (
            super::super::incompats::Id(1),
            Incompat {
                name: String::from("LV2 - Allemand"),
                max_count: 0,
                groups: BTreeSet::from([
                    IncompatGroup {
                        slots: BTreeSet::from([
                            IncompatSlot {
                                week_pattern_id: super::super::week_patterns::Id(2),
                                start: TimeSlot {
                                    day: crate::time::Weekday::Monday,
                                    time: crate::time::Time::from_hm(8, 0).unwrap(),
                                },
                                duration: NonZeroU32::new(60).unwrap(),
                            },
                            IncompatSlot {
                                week_pattern_id: super::super::week_patterns::Id(2),
                                start: TimeSlot {
                                    day: crate::time::Weekday::Wednesday,
                                    time: crate::time::Time::from_hm(12, 0).unwrap(),
                                },
                                duration: NonZeroU32::new(60).unwrap(),
                            },
                        ]),
                    },
                    IncompatGroup {
                        slots: BTreeSet::from([IncompatSlot {
                            week_pattern_id: super::super::week_patterns::Id(3),
                            start: TimeSlot {
                                day: crate::time::Weekday::Friday,
                                time: crate::time::Time::from_hm(14, 0).unwrap(),
                            },
                            duration: NonZeroU32::new(120).unwrap(),
                        }]),
                    },
                ]),
            },
        ),
        (
            super::super::incompats::Id(2),
            Incompat {
                name: String::from("Repas midi - lundi"),
                max_count: 2,
                groups: BTreeSet::from([
                    IncompatGroup {
                        slots: BTreeSet::from([IncompatSlot {
                            week_pattern_id: super::super::week_patterns::Id(1),
                            start: TimeSlot {
                                day: crate::time::Weekday::Monday,
                                time: crate::time::Time::from_hm(11, 0).unwrap(),
                            },
                            duration: NonZeroU32::new(60).unwrap(),
                        }]),
                    },
                    IncompatGroup {
                        slots: BTreeSet::from([IncompatSlot {
                            week_pattern_id: super::super::week_patterns::Id(1),
                            start: TimeSlot {
                                day: crate::time::Weekday::Monday,
                                time: crate::time::Time::from_hm(12, 0).unwrap(),
                            },
                            duration: NonZeroU32::new(60).unwrap(),
                        }]),
                    },
                    IncompatGroup {
                        slots: BTreeSet::from([IncompatSlot {
                            week_pattern_id: super::super::week_patterns::Id(1),
                            start: TimeSlot {
                                day: crate::time::Weekday::Monday,
                                time: crate::time::Time::from_hm(13, 0).unwrap(),
                            },
                            duration: NonZeroU32::new(60).unwrap(),
                        }]),
                    },
                ]),
            },
        ),
    ]);

    assert_eq!(incompats, expected_result);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IncompatDb {
    incompat_id: i64,
    name: String,
    max_count: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IncompatGroupDb {
    incompat_group_id: i64,
    incompat_id: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IncompatGroupItemDb {
    incompat_group_id: i64,
    week_pattern_id: i64,
    start_day: i64,
    start_time: i64,
    duration: i64,
}

#[sqlx::test]
async fn incompats_add_one(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);
        "#
    ).execute(&store.pool).await.unwrap();

    let id = store
        .incompats_add(Incompat {
            name: String::from("LV2 - Allemand"),
            max_count: 0,
            groups: BTreeSet::from([
                IncompatGroup {
                    slots: BTreeSet::from([
                        IncompatSlot {
                            week_pattern_id: super::super::week_patterns::Id(2),
                            start: TimeSlot {
                                day: crate::time::Weekday::Monday,
                                time: crate::time::Time::from_hm(8, 0).unwrap(),
                            },
                            duration: NonZeroU32::new(60).unwrap(),
                        },
                        IncompatSlot {
                            week_pattern_id: super::super::week_patterns::Id(2),
                            start: TimeSlot {
                                day: crate::time::Weekday::Wednesday,
                                time: crate::time::Time::from_hm(12, 0).unwrap(),
                            },
                            duration: NonZeroU32::new(60).unwrap(),
                        },
                    ]),
                },
                IncompatGroup {
                    slots: BTreeSet::from([IncompatSlot {
                        week_pattern_id: super::super::week_patterns::Id(3),
                        start: TimeSlot {
                            day: crate::time::Weekday::Friday,
                            time: crate::time::Time::from_hm(14, 0).unwrap(),
                        },
                        duration: NonZeroU32::new(120).unwrap(),
                    }]),
                },
            ]),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::incompats::Id(1));

    let incompats = sqlx::query_as!(IncompatDb, "SELECT * FROM incompats")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let incompat_groups = sqlx::query_as!(IncompatGroupDb, "SELECT * FROM incompat_groups")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let incompat_group_items =
        sqlx::query_as!(IncompatGroupItemDb, "SELECT * FROM incompat_group_items")
            .fetch_all(&store.pool)
            .await
            .unwrap();

    let incompats_expected = vec![IncompatDb {
        incompat_id: 1,
        name: String::from("LV2 - Allemand"),
        max_count: 0,
    }];

    let incompat_groups_expected = vec![
        IncompatGroupDb {
            incompat_group_id: 1,
            incompat_id: 1,
        },
        IncompatGroupDb {
            incompat_group_id: 2,
            incompat_id: 1,
        },
    ];

    let incompat_group_items_expected = vec![
        IncompatGroupItemDb {
            incompat_group_id: 1,
            week_pattern_id: 2,
            start_day: 0,
            start_time: 480,
            duration: 60,
        },
        IncompatGroupItemDb {
            incompat_group_id: 1,
            week_pattern_id: 2,
            start_day: 2,
            start_time: 720,
            duration: 60,
        },
        IncompatGroupItemDb {
            incompat_group_id: 2,
            week_pattern_id: 3,
            start_day: 4,
            start_time: 840,
            duration: 120,
        },
    ];

    assert_eq!(incompats, incompats_expected);
    assert_eq!(incompat_groups, incompat_groups_expected);
    assert_eq!(incompat_group_items, incompat_group_items_expected);
}

#[sqlx::test]
async fn incompats_add_multiple(pool: sqlx::SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
INSERT INTO week_patterns (name) VALUES ("Toutes"), ("Impaires"), ("Paires");
INSERT INTO weeks (week_pattern_id, week) VALUES (1,0), (1,1), (1,2), (1,3), (1,4), (1,5), (1,6), (1,7), (1,8), (1,9);
INSERT INTO weeks (week_pattern_id, week) VALUES (2,0), (2,2), (2,4), (2,6), (2,8);
INSERT INTO weeks (week_pattern_id, week) VALUES (3,1), (3,3), (3,5), (3,7), (3,9);
        "#
    ).execute(&store.pool).await.unwrap();

    let id = store
        .incompats_add(Incompat {
            name: String::from("LV2 - Allemand"),
            max_count: 0,
            groups: BTreeSet::from([
                IncompatGroup {
                    slots: BTreeSet::from([
                        IncompatSlot {
                            week_pattern_id: super::super::week_patterns::Id(2),
                            start: TimeSlot {
                                day: crate::time::Weekday::Monday,
                                time: crate::time::Time::from_hm(8, 0).unwrap(),
                            },
                            duration: NonZeroU32::new(60).unwrap(),
                        },
                        IncompatSlot {
                            week_pattern_id: super::super::week_patterns::Id(2),
                            start: TimeSlot {
                                day: crate::time::Weekday::Wednesday,
                                time: crate::time::Time::from_hm(12, 0).unwrap(),
                            },
                            duration: NonZeroU32::new(60).unwrap(),
                        },
                    ]),
                },
                IncompatGroup {
                    slots: BTreeSet::from([IncompatSlot {
                        week_pattern_id: super::super::week_patterns::Id(3),
                        start: TimeSlot {
                            day: crate::time::Weekday::Friday,
                            time: crate::time::Time::from_hm(14, 0).unwrap(),
                        },
                        duration: NonZeroU32::new(120).unwrap(),
                    }]),
                },
            ]),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::incompats::Id(1));

    let id = store
        .incompats_add(Incompat {
            name: String::from("Repas midi - lundi"),
            max_count: 2,
            groups: BTreeSet::from([
                IncompatGroup {
                    slots: BTreeSet::from([IncompatSlot {
                        week_pattern_id: super::super::week_patterns::Id(1),
                        start: TimeSlot {
                            day: crate::time::Weekday::Monday,
                            time: crate::time::Time::from_hm(11, 0).unwrap(),
                        },
                        duration: NonZeroU32::new(60).unwrap(),
                    }]),
                },
                IncompatGroup {
                    slots: BTreeSet::from([IncompatSlot {
                        week_pattern_id: super::super::week_patterns::Id(1),
                        start: TimeSlot {
                            day: crate::time::Weekday::Monday,
                            time: crate::time::Time::from_hm(12, 0).unwrap(),
                        },
                        duration: NonZeroU32::new(60).unwrap(),
                    }]),
                },
                IncompatGroup {
                    slots: BTreeSet::from([IncompatSlot {
                        week_pattern_id: super::super::week_patterns::Id(1),
                        start: TimeSlot {
                            day: crate::time::Weekday::Monday,
                            time: crate::time::Time::from_hm(13, 0).unwrap(),
                        },
                        duration: NonZeroU32::new(60).unwrap(),
                    }]),
                },
            ]),
        })
        .await
        .unwrap();

    assert_eq!(id, super::super::incompats::Id(2));

    let incompats = sqlx::query_as!(IncompatDb, "SELECT * FROM incompats")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let incompat_groups = sqlx::query_as!(IncompatGroupDb, "SELECT * FROM incompat_groups")
        .fetch_all(&store.pool)
        .await
        .unwrap();

    let incompat_group_items =
        sqlx::query_as!(IncompatGroupItemDb, "SELECT * FROM incompat_group_items")
            .fetch_all(&store.pool)
            .await
            .unwrap();

    let incompats_expected = vec![
        IncompatDb {
            incompat_id: 1,
            name: String::from("LV2 - Allemand"),
            max_count: 0,
        },
        IncompatDb {
            incompat_id: 2,
            name: String::from("Repas midi - lundi"),
            max_count: 2,
        },
    ];

    let incompat_groups_expected = vec![
        IncompatGroupDb {
            incompat_group_id: 1,
            incompat_id: 1,
        },
        IncompatGroupDb {
            incompat_group_id: 2,
            incompat_id: 1,
        },
        IncompatGroupDb {
            incompat_group_id: 3,
            incompat_id: 2,
        },
        IncompatGroupDb {
            incompat_group_id: 4,
            incompat_id: 2,
        },
        IncompatGroupDb {
            incompat_group_id: 5,
            incompat_id: 2,
        },
    ];

    let incompat_group_items_expected = vec![
        IncompatGroupItemDb {
            incompat_group_id: 1,
            week_pattern_id: 2,
            start_day: 0,
            start_time: 480,
            duration: 60,
        },
        IncompatGroupItemDb {
            incompat_group_id: 1,
            week_pattern_id: 2,
            start_day: 2,
            start_time: 720,
            duration: 60,
        },
        IncompatGroupItemDb {
            incompat_group_id: 2,
            week_pattern_id: 3,
            start_day: 4,
            start_time: 840,
            duration: 120,
        },
        IncompatGroupItemDb {
            incompat_group_id: 3,
            week_pattern_id: 1,
            start_day: 0,
            start_time: 660,
            duration: 60,
        },
        IncompatGroupItemDb {
            incompat_group_id: 4,
            week_pattern_id: 1,
            start_day: 0,
            start_time: 720,
            duration: 60,
        },
        IncompatGroupItemDb {
            incompat_group_id: 5,
            week_pattern_id: 1,
            start_day: 0,
            start_time: 780,
            duration: 60,
        },
    ];

    assert_eq!(incompats, incompats_expected);
    assert_eq!(incompat_groups, incompat_groups_expected);
    assert_eq!(incompat_group_items, incompat_group_items_expected);
}
