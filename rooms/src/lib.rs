pub mod data_model;

use std::path::Path;

pub use data_model::ScheduleError;

/// Parse both CSV files and print summary statistics.
pub fn run(rooms: &Path, requests: &Path) -> Result<(), ScheduleError> {
    let data = data_model::parse_schedule(rooms, requests)?;
    eprintln!(
        "Parsed {} rooms and {} requests",
        data.rooms.len(),
        data.requests.len(),
    );
    Ok(())
}
