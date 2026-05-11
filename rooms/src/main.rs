use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Room scheduling tool")]
struct Args {
    /// Convert an xlsx file to CSV (output on stdout)
    #[arg(long)]
    convert: Option<PathBuf>,

    /// Input CSV files for schedule mode (required when not using --convert)
    files: Vec<PathBuf>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if let Some(xlsx_path) = args.convert {
        eprintln!("Convert mode: reading {xlsx_path:?}");
        return Ok(());
    }

    if args.files.len() != 2 {
        anyhow::bail!("Schedule mode requires exactly 2 CSV file arguments");
    }
    eprintln!("Schedule mode: files {:?} and {:?}", args.files[0], args.files[1]);
    Ok(())
}
