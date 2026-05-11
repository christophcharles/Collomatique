use clap::Parser;
use std::path::PathBuf;

use collomatique_rooms::schedule;

#[derive(Parser, Debug)]
#[command(author, version, about = "Assign rooms to scheduling requests")]
struct Args {
    /// CSV file describing available rooms and their characteristics
    rooms: PathBuf,

    /// CSV file describing scheduling requests and their constraints
    requests: PathBuf,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    schedule::run(&args.rooms, &args.requests)?;
    Ok(())
}
