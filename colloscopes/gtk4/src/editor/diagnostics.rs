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
