use anyhow::Result;

pub fn run_gui(create: bool, db: Option<std::path::PathBuf>) -> Result<()> {
    println!("GUI was run with file: {:?}", db);
    if create {
        println!("File should be created (and non-existent)");
    } else {
        println!("File should already exist");
    }
    Ok(())
}
