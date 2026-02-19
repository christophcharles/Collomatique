pub async fn export_to_sqlite(
    data: &collomatique_state_colloscopes::InnerData,
    path: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let pool = sqlx::SqlitePool::connect(":memory:").await?;
    collomatique_sqlite_state::create_schema(&pool).await?;
    collomatique_sqlite_state::inner_data_to_sqlite(&pool, data).await?;
    collomatique_sqlite_state::export_to_file(&pool, path).await?;
    Ok(())
}
