use clap::Parser;
use std::path::PathBuf;

use collomatique_rooms::schedule;

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

    schedule::run(&args.files)?;
    Ok(())
}
