pub fn export_to_xlsx(
    data: &collomatique_state_colloscopes::InnerData,
    path: &std::path::Path,
    xlsx_config: &collomatique_xlsx::Config,
) -> Result<(), anyhow::Error> {
    collomatique_xlsx::write_xlsx(data, path, xlsx_config)
        .map_err(|e| anyhow::anyhow!("Failed to write XLSX: {e}"))?;
    Ok(())
}
