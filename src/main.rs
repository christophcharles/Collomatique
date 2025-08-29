use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Sqlite file (to open or create) that contains the database
    db: std::path::PathBuf,
}

use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::SqliteConnection;
use sqlx::Connection;

async fn create_db(db_url: &str) -> Result<()> {
    sqlx::Sqlite::create_database(db_url).await?;
    let mut db = SqliteConnection::connect(&db_url).await?;

    sqlx::query("CREATE TABLE IF NOT EXISTS students (id INTEGER PRIMARY KEY NOT NULL, surname STRING NOT NULL, firstname STRING NOT NULL);")
        .execute(&mut db).await?;

    Ok(())
}

async fn open_db(path: &std::path::Path) -> Result<SqliteConnection> {
    let filename = match path.to_str() {
        Some(f) => f,
        None => return Err(anyhow!("Non UTF-8 file name")),
    };
    let db_url = format!("sqlite://{}", filename);

    if !sqlx::Sqlite::database_exists(&db_url)
        .await
        .unwrap_or(false)
    {
        create_db(&db_url).await?;
    }

    Ok(SqliteConnection::connect(&db_url).await?)
}

#[async_std::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut db = open_db(args.db.as_path()).await?;
    let result = sqlx::query("SELECT * FROM students;")
        .execute(&mut db)
        .await?;

    println!("{:?}", result);

    Ok(())
}
