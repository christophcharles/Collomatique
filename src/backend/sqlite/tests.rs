use super::*;

async fn prepare_empty_db(pool: sqlx::SqlitePool) -> Store {
    Store::fill_empty_db(&pool).await.unwrap();
    Store { pool }
}

mod group_lists;
mod grouping_incompats;
mod groupings;
mod incompat_for_student;
mod incompats;
mod students;
mod subject_group_for_student;
mod subject_groups;
mod subjects;
mod teachers;
mod time_slots;
mod week_patterns;

#[sqlx::test]
async fn general_data_get_1(pool: SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let general_data = store.general_data_get().await.unwrap();

    let general_data_expected = GeneralData {
        interrogations_per_week: None,
        max_interrogations_per_day: None,
        week_count: NonZeroU32::new(30).unwrap(),
    };

    assert_eq!(general_data, general_data_expected);
}

#[sqlx::test]
async fn general_data_get_2(pool: SqlitePool) {
    let store = prepare_empty_db(pool).await;

    let _ = sqlx::query!(
        r#"
UPDATE general_data
SET value = '{"interrogations_per_week":{"start":2,"end":5},"max_interrogations_per_day":2,"week_count":25}'
WHERE id = 1
        "#
    )
    .execute(&store.pool)
    .await
    .unwrap();

    let general_data = store.general_data_get().await.unwrap();

    let general_data_expected = GeneralData {
        interrogations_per_week: Some(2..5),
        max_interrogations_per_day: Some(NonZeroU32::new(2).unwrap()),
        week_count: NonZeroU32::new(25).unwrap(),
    };

    assert_eq!(general_data, general_data_expected);
}

#[sqlx::test]
async fn general_data_set(pool: SqlitePool) {
    let mut store = prepare_empty_db(pool).await;

    unsafe {
        store.general_data_set_unchecked(&GeneralData {
            interrogations_per_week: Some(2..5),
            max_interrogations_per_day: Some(NonZeroU32::new(2).unwrap()),
            week_count: NonZeroU32::new(25).unwrap(),
        })
    }
    .await
    .unwrap();

    let general_data = store.general_data_get().await.unwrap();

    let general_data_expected = GeneralData {
        interrogations_per_week: Some(2..5),
        max_interrogations_per_day: Some(NonZeroU32::new(2).unwrap()),
        week_count: NonZeroU32::new(25).unwrap(),
    };

    assert_eq!(general_data, general_data_expected);
}
