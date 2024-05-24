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
use sqlx::sqlite::SqlitePool;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GeneralDataDb {
    interrogations_per_week: Option<std::ops::Range<u32>>,
}

async fn create_db(db_url: &str) -> Result<SqlitePool> {
    if sqlx::Sqlite::database_exists(db_url).await? {
        return Err(anyhow!("Database \"{}\" already exists", db_url));
    }

    sqlx::Sqlite::create_database(db_url).await?;
    let db = SqlitePool::connect(&db_url).await?;

    sqlx::query(
        r#"
CREATE TABLE "colloscopes" (
    "colloscope_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL,
    PRIMARY KEY("colloscope_id" AUTOINCREMENT)
);

CREATE TABLE "course_incompats" (
	"course_incompat_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	PRIMARY KEY("course_incompat_id")
);

CREATE TABLE "week_patterns" (
	"week_pattern_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	PRIMARY KEY("week_pattern_id")
);

CREATE TABLE "weeks" (
	"week_pattern_id"	INTEGER NOT NULL,
	"week"	INTEGER NOT NULL,
	FOREIGN KEY("week_pattern_id") REFERENCES "week_patterns"("week_pattern_id"),
	PRIMARY KEY("week_pattern_id","week")
);

CREATE TABLE "course_incompat_items" (
	"course_incompat_id"	INTEGER NOT NULL,
	"week_pattern_id"	INTEGER NOT NULL,
	"start_day"	INTEGER NOT NULL,
	"start_time"	INTEGER NOT NULL,
	"duration"	INTEGER NOT NULL,
    FOREIGN KEY("course_incompat_id") REFERENCES "course_incompats"("course_incompat_id"),
	PRIMARY KEY("course_incompat_id","week_pattern_id","start_day","start_time","duration"),
    FOREIGN KEY("week_pattern_id") REFERENCES "week_patterns"("week_pattern_id")
);

CREATE TABLE "general_data" (
	"id"	INTEGER NOT NULL,
	"value"	TEXT NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT)
);

INSERT INTO "general_data" ( "value" ) VALUES ( ? );

CREATE TABLE "teachers" (
	"teacher_id"	INTEGER NOT NULL,
	"surname"	TEXT NOT NULL,
	"firstname"	TEXT NOT NULL,
	"contact"	TEXT NOT NULL,
	PRIMARY KEY("teacher_id" AUTOINCREMENT)
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
	"subject_group_id"	INTEGER NOT NULL,
	"duration"	INTEGER NOT NULL,
	"course_incompat_id"	INTEGER,
	"min_students_per_slot"	INTEGER NOT NULL,
	"max_students_per_slot"	INTEGER NOT NULL,
	"period"	INTEGER NOT NULL,
	"period_is_strict"	INTEGER NOT NULL,
	FOREIGN KEY("course_incompat_id") REFERENCES "course_incompats"("course_incompat_id"),
	PRIMARY KEY("subject_id" AUTOINCREMENT),
    FOREIGN KEY("subject_group_id") REFERENCES "subject_groups"("subject_group_id")
);

CREATE TABLE "groupings" (
	"grouping_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	PRIMARY KEY("grouping_id" AUTOINCREMENT)
);

CREATE TABLE "grouping_incompats" (
	"id1"	INTEGER NOT NULL,
	"id2"	INTEGER NOT NULL,
	FOREIGN KEY("id1") REFERENCES "groupings"("grouping_id"),
	PRIMARY KEY("id1","id2"),
	FOREIGN KEY("id2") REFERENCES "groupings"("grouping_id")
);

CREATE TABLE "time_slots" (
	"time_slot_id"	INTEGER NOT NULL,
	"subject_id"	INTEGER NOT NULL,
	"teacher_id"	INTEGER NOT NULL,
	"start_day"	INTEGER NOT NULL,
	"start_time"	INTEGER NOT NULL,
	"week_pattern_id"	INTEGER NOT NULL,
	FOREIGN KEY("week_pattern_id") REFERENCES "week_patterns"("week_pattern_id"),
	PRIMARY KEY("time_slot_id" AUTOINCREMENT),
	FOREIGN KEY("subject_id") REFERENCES "subjects"("subject_id"),
	FOREIGN KEY("teacher_id") REFERENCES "teachers"("teacher_id")
);

CREATE TABLE "grouping_items" (
	"grouping_id"	INTEGER NOT NULL,
	"time_slot_id"	INTEGER NOT NULL,
    FOREIGN KEY("grouping_id") REFERENCES "groupings"("grouping_id"),
	FOREIGN KEY("time_slot_id") REFERENCES "time_slots"("time_slot_id"),
	PRIMARY KEY("grouping_id","time_slot_id")
);

CREATE TABLE "student_incompats" (
	"student_id"	INTEGER NOT NULL,
	"incompat_id"	INTEGER NOT NULL,
	PRIMARY KEY("student_id","incompat_id"),
    FOREIGN KEY("student_id") REFERENCES "students"("student_id"),
	FOREIGN KEY("incompat_id") REFERENCES "course_incompats"("course_incompat_id")
);

CREATE TABLE "student_subjects" (
	"student_id"	INTEGER NOT NULL,
	"subject_id"	INTEGER NOT NULL,
    FOREIGN KEY("subject_id") REFERENCES "subjects"("subject_id"),
	FOREIGN KEY("student_id") REFERENCES "students"("student_id"),
	PRIMARY KEY("subject_id","student_id")
);"#,
    )
    .bind(serde_json::to_string(&GeneralDataDb {
        interrogations_per_week: None,
    })?)
    .execute(&db)
    .await?;

    Ok(db)
}

async fn open_db(db_url: &str) -> Result<SqlitePool> {
    if !sqlx::Sqlite::database_exists(db_url).await? {
        return Err(anyhow!("Database \"{}\" does not exist", db_url));
    }

    Ok(SqlitePool::connect(db_url).await?)
}

async fn connect_db(create: bool, path: &std::path::Path) -> Result<SqlitePool> {
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

async fn generate_general_data(
    db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::GeneralData> {
    use collomatique::gen::colloscope::*;
    use std::num::NonZeroU32;

    let teacher_count_req =
        sqlx::query!("SELECT COUNT(*) AS teacher_count FROM teachers").fetch_all(db_conn);
    let week_count_req = sqlx::query!("SELECT MAX(week) AS week_max FROM weeks").fetch_all(db_conn);
    let interrogations_per_week_req =
        sqlx::query!("SELECT value FROM general_data WHERE id = ?", 1).fetch_all(db_conn);

    let teacher_count = usize::try_from(teacher_count_req.await?[0].teacher_count).unwrap();
    let week_count = match week_count_req.await?[0].week_max {
        Some(week_max) => NonZeroU32::new(u32::try_from(week_max).unwrap() + 1).unwrap(),
        None => NonZeroU32::new(1).unwrap(),
    };
    let interrogations_per_week = match interrogations_per_week_req.await?.first() {
        Some(val) => {
            let general_data_db: GeneralDataDb = serde_json::from_str(&val.value)?;
            general_data_db.interrogations_per_week
        }
        None => {
            return Err(anyhow!("Bad general_data in database"));
        }
    };

    Ok(GeneralData {
        teacher_count,
        week_count,
        interrogations_per_week,
    })
}

async fn generate_subjects(
    _db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::SubjectList> {
    use collomatique::gen::colloscope::*;

    Ok(SubjectList::new())
}

async fn generate_incompatibilies(
    _db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::IncompatibilityList> {
    use collomatique::gen::colloscope::*;

    Ok(IncompatibilityList::new())
}

async fn generate_students(
    _db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::StudentList> {
    use collomatique::gen::colloscope::*;

    Ok(StudentList::new())
}

async fn generate_slot_groupings(
    _db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::SlotGroupingList> {
    use collomatique::gen::colloscope::*;

    Ok(SlotGroupingList::new())
}

async fn generate_grouping_incompats(
    _db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::SlotGroupingIncompatSet> {
    use collomatique::gen::colloscope::*;

    Ok(SlotGroupingIncompatSet::new())
}

async fn generate_colloscope_data(
    db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::ValidatedData> {
    use collomatique::gen::colloscope::*;

    let general = generate_general_data(db_conn);
    let subjects = generate_subjects(db_conn);
    let incompatibilities = generate_incompatibilies(db_conn);
    let students = generate_students(db_conn);
    let slot_groupings = generate_slot_groupings(db_conn);
    let grouping_incompats = generate_grouping_incompats(db_conn);

    Ok(ValidatedData::new(
        general.await?,
        subjects.await?,
        incompatibilities.await?,
        students.await?,
        slot_groupings.await?,
        grouping_incompats.await?,
    )?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let db = connect_db(args.create, args.db.as_path()).await?;

    let result = generate_colloscope_data(&db).await?;

    println!("{:?}", result);

    Ok(())
}
