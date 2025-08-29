use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Create new database if does not exist already
    #[arg(short, long, default_value_t = false)]
    create: bool,
    /// Sqlite file (to open or create) that contains the database
    db: std::path::PathBuf,
}

use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::SqliteConnection;
use sqlx::Connection;

async fn create_db(db_url: &str) -> Result<SqliteConnection> {
    if sqlx::Sqlite::database_exists(db_url).await? {
        return Err(anyhow!("Database \"{}\" already exists", db_url));
    }

    sqlx::Sqlite::create_database(db_url).await?;
    let mut db = SqliteConnection::connect(&db_url).await?;

    sqlx::query(
        r#"
CREATE TABLE "colloscopes" (
    "colloscope_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL,
    PRIMARY KEY("colloscope_id" AUTOINCREMENT)
);

CREATE TABLE "course_incompat_items" (
	"course_incompat_id"	INTEGER NOT NULL,
	"week_pattern_id"	INTEGER NOT NULL,
	"start_day"	INTEGER NOT NULL,
	"start_time"	INTEGER NOT NULL,
	"duration"	INTEGER NOT NULL,
	PRIMARY KEY("course_incompat_id","week_pattern_id","start_day","start_time","duration")
);

CREATE TABLE "course_incompats" (
	"course_incompat_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	PRIMARY KEY("course_incompat_id")
);

CREATE TABLE "general_data" (
	"name"	TEXT NOT NULL,
	"value"	INTEGER,
	PRIMARY KEY("name")
);

CREATE TABLE "grouping_incompats" (
	"id1"	INTEGER NOT NULL,
	"id2"	INTEGER NOT NULL,
	PRIMARY KEY("id1","id2")
);

CREATE TABLE "grouping_items" (
	"grouping_id"	INTEGER NOT NULL,
	"time_slot_id"	INTEGER NOT NULL,
	PRIMARY KEY("grouping_id","time_slot_id")
);

CREATE TABLE "groupings" (
	"grouping_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	PRIMARY KEY("grouping_id" AUTOINCREMENT)
);

CREATE TABLE "student_incompats" (
	"student_id"	INTEGER NOT NULL,
	"incompat_id"	INTEGER NOT NULL,
	PRIMARY KEY("student_id","incompat_id")
);

CREATE TABLE "student_subjects" (
	"student_id"	INTEGER NOT NULL,
	"subject_id"	INTEGER NOT NULL,
	PRIMARY KEY("subject_id","student_id")
);

CREATE TABLE "students" (
	"student_id"	INTEGER NOT NULL,
	"surname"	TEXT NOT NULL,
	"firstname"	TEXT NOT NULL,
	"email"	TEXT,
	"phone"	TEXT,
	PRIMARY KEY("student_id" AUTOINCREMENT)
);

CREATE TABLE "subject_groups" (
	"subject_group_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	"optional"	INTEGER NOT NULL,
	PRIMARY KEY("subject_group_id" AUTOINCREMENT)
);

CREATE TABLE "subjects" (
	"subject_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	"subject_group"	INTEGER NOT NULL,
	"duration"	INTEGER NOT NULL,
	"course_incompat_id"	INTEGER,
	"min_students_per_slot"	INTEGER NOT NULL,
	"max_students_per_slot"	INTEGER NOT NULL,
	"period"	INTEGER NOT NULL,
	"period_is_strict"	INTEGER NOT NULL,
	PRIMARY KEY("subject_id" AUTOINCREMENT)
);

CREATE TABLE "teachers" (
	"teacher_id"	INTEGER NOT NULL,
	"surname"	TEXT NOT NULL,
	"firstname"	TEXT NOT NULL,
	"contact"	TEXT NOT NULL,
	PRIMARY KEY("teacher_id" AUTOINCREMENT)
);

CREATE TABLE "time_slots" (
	"time_slot_id"	INTEGER NOT NULL,
	"subject_id"	INTEGER NOT NULL,
	"teacher_id"	INTEGER NOT NULL,
	"start_day"	INTEGER NOT NULL,
	"start_time"	INTEGER NOT NULL,
	"week_pattern"	INTEGER NOT NULL,
	PRIMARY KEY("time_slot_id" AUTOINCREMENT)
);

CREATE TABLE "week_patterns" (
	"week_pattern_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	PRIMARY KEY("week_pattern_id")
);

CREATE TABLE "weeks" (
	"week_pattern_id"	INTEGER NOT NULL,
	"week"	INTEGER NOT NULL,
	PRIMARY KEY("week_pattern_id","week")
);"#,
    )
    .execute(&mut db)
    .await?;

    Ok(SqliteConnection::connect(&db_url).await?)
}

async fn open_db(db_url: &str) -> Result<SqliteConnection> {
    if !sqlx::Sqlite::database_exists(db_url).await? {
        return Err(anyhow!("Database \"{}\" does not exist", db_url));
    }

    Ok(SqliteConnection::connect(db_url).await?)
}

async fn connect_db(create: bool, path: &std::path::Path) -> Result<SqliteConnection> {
    let filename = match path.to_str() {
        Some(f) => f,
        None => return Err(anyhow!("Non UTF-8 file name")),
    };
    let db_url = format!("sqlite://{}", filename);

    if create {
        create_db(&db_url).await
    } else {
        open_db(&db_url).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut db = connect_db(args.create, args.db.as_path()).await?;
    let result = sqlx::query!("SELECT * FROM students")
        .fetch_all(&mut db)
        .await?;

    println!("{:?}", result);

    Ok(())
}
