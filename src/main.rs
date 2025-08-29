use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Sqlite file (to open or create) that contains the database
    db: std::path::PathBuf,
}

fn main() {
    let args = Args::parse();

    println!("{}", args.db.to_string_lossy());
}
