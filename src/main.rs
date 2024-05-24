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
CREATE TABLE "Colloscopes" (
    "ColloscopeId"	INTEGER NOT NULL,
    "Name"	TEXT NOT NULL,
    PRIMARY KEY("ColloscopeId" AUTOINCREMENT)
);

CREATE TABLE "CourseIncompatItems" (
    "CourseIncompatId"	INTEGER NOT NULL,
    "WeekPatternId"	INTEGER NOT NULL,
    "StartDay"	INTEGER NOT NULL,
    "StartTime"	INTEGER NOT NULL,
    "Duration"	INTEGER NOT NULL,
    PRIMARY KEY("CourseIncompatId","WeekPatternId","StartDay","StartTime","Duration")
);

CREATE TABLE "CourseIncompats" (
    "CourseIncompatId"	INTEGER NOT NULL,
    "Name"	TEXT NOT NULL,
    PRIMARY KEY("CourseIncompatId")
);

CREATE TABLE "GeneralData" (
    "Name"	TEXT NOT NULL,
    "Value"	INTEGER,
    PRIMARY KEY("Name")
);

CREATE TABLE "GroupingIncompats" (
    "Id1"	INTEGER NOT NULL,
    "Id2"	INTEGER NOT NULL,
    PRIMARY KEY("Id1","Id2")
);

CREATE TABLE "GroupingItems" (
    "GroupingId"	INTEGER NOT NULL,
    "TimeSlotId"	INTEGER NOT NULL,
    PRIMARY KEY("GroupingId","TimeSlotId")
);

CREATE TABLE "Groupings" (
	"GroupingId"	INTEGER NOT NULL,
	"Name"	TEXT NOT NULL,
	PRIMARY KEY("GroupingId" AUTOINCREMENT)
);

CREATE TABLE "StudentIncompats" (
	"StudentId"	INTEGER NOT NULL,
	"IncompatId"	INTEGER NOT NULL,
	PRIMARY KEY("StudentId","IncompatId")
);

CREATE TABLE "StudentSubjects" (
	"StudentId"	INTEGER NOT NULL,
	"SubjectId"	INTEGER NOT NULL,
	PRIMARY KEY("SubjectId","StudentId")
);

CREATE TABLE "Students" (
	"StudentId"	INTEGER NOT NULL,
	"Surname"	TEXT NOT NULL,
	"Firstname"	TEXT NOT NULL,
	"Email"	TEXT,
	"Phone"	TEXT,
	PRIMARY KEY("StudentId" AUTOINCREMENT)
);

CREATE TABLE "SubjectGroups" (
	"SubjectGroupId"	INTEGER NOT NULL,
	"Name"	TEXT NOT NULL,
	"Optional"	INTEGER NOT NULL,
	PRIMARY KEY("SubjectGroupId" AUTOINCREMENT)
);

CREATE TABLE "Subjects" (
	"SubjectId"	INTEGER NOT NULL,
	"Name"	TEXT NOT NULL,
	"SubjectGroup"	INTEGER NOT NULL,
	"Duration"	INTEGER NOT NULL,
	"CourseIncompatId"	INTEGER,
	"MinStudentsPerSlot"	INTEGER NOT NULL,
	"MaxStudentsPerSlot"	INTEGER NOT NULL,
	"Period"	INTEGER NOT NULL,
	"PeriodIsStrict"	INTEGER NOT NULL,
	PRIMARY KEY("SubjectId" AUTOINCREMENT)
);

CREATE TABLE "Teachers" (
	"TeacherId"	INTEGER NOT NULL,
	"Surname"	TEXT NOT NULL,
	"Firstname"	TEXT NOT NULL,
	"Contact"	TEXT NOT NULL,
	PRIMARY KEY("TeacherId" AUTOINCREMENT)
);

CREATE TABLE "TimeSlots" (
	"TimeSlotId"	INTEGER NOT NULL,
	"SubjectId"	INTEGER NOT NULL,
	"TeacherId"	INTEGER NOT NULL,
	"StartDay"	INTEGER NOT NULL,
	"StartTime"	INTEGER NOT NULL,
	"WeekPattern"	INTEGER NOT NULL,
	PRIMARY KEY("TimeSlotId" AUTOINCREMENT)
);

CREATE TABLE "WeekPatterns" (
	"WeekPatternId"	INTEGER NOT NULL,
	"Name"	TEXT NOT NULL,
	PRIMARY KEY("WeekPatternId")
);

CREATE TABLE "Weeks" (
	"WeekPatternId"	INTEGER NOT NULL,
	"Week"	INTEGER NOT NULL,
	PRIMARY KEY("WeekPatternId","Week")
);
        "#,
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

#[async_std::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut db = connect_db(args.create, args.db.as_path()).await?;
    let result = sqlx::query!("SELECT * FROM Students")
        .fetch_all(&mut db)
        .await?;

    println!("{:?}", result);

    Ok(())
}
