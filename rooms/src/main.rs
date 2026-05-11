use clap::Parser;
use std::path::PathBuf;

mod schedule;

#[derive(Parser, Debug)]
#[command(author, version, about = "Room scheduling tool")]
struct Args {
    /// Convert an xlsx file to CSV (output on stdout)
    #[arg(long)]
    convert: Option<PathBuf>,

    /// Input CSV files for schedule mode: rooms file first, then requests file
    files: Vec<PathBuf>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if let Some(xlsx_path) = args.convert {
        eprintln!("Convert mode: reading {xlsx_path:?}");
        return Ok(());
    }

    if args.files.len() != 2 {
        anyhow::bail!("Schedule mode requires exactly 2 CSV file arguments (rooms, requests)");
    }

    let data = schedule::parse_schedule(&args.files[0], &args.files[1])?;
    eprintln!(
        "Parsed {} rooms and {} requests with characteristics: {:?}",
        data.rooms.len(),
        data.requests.len(),
        data.characteristics,
    );
    Ok(())
}
