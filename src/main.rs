use async_std::task;
use clap::Parser;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Sqlite file (to open or create) that contains the database
    db: std::path::PathBuf,
}

async fn test_db() -> Result<()> {
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("{}", args.db.to_string_lossy());

    let fut = test_db();
    task::block_on(fut)?;

    Ok(())
}
