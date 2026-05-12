pub mod data_model;
pub mod types;

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
    for name in data.unregistered_rooms() {
        eprintln!(
            "Warning: room \"{name}\" is not registered in the rooms file. \
             In case of double occupancy, we will not be able to find the closest available room."
        );
    }
    Ok(())
}
