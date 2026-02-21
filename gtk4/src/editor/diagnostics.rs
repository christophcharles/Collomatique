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

pub async fn export_to_mps<V, C, P>(
    problem: &collomatique_ilp::Problem<V, C, P>,
    path: &std::path::Path,
) -> Result<(), anyhow::Error>
where
    V: collomatique_ilp::UsableData,
    C: collomatique_ilp::UsableData,
    P: collomatique_ilp::mat_repr::ProblemRepr<V>,
{
    let names = collomatique_mps::generate_names(problem);
    let mps_content = collomatique_mps::generate_mps(problem, &names);
    tokio::fs::write(path, mps_content).await?;
    Ok(())
}
