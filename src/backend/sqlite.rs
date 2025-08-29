#[cfg(test)]
mod tests;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlx error")]
    SqlxError(#[from] sqlx::Error),
    #[error("Corrupted database: {0}")]
    CorruptedDatabase(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum NewError {
    #[error("Path is not a valid UTF-8 string")]
    InvalidPath,
    #[error("Trying to override already existing database {0}")]
    DatabaseAlreadyExists(std::path::PathBuf),
    #[error("sqlx error")]
    SqlxError(#[from] sqlx::Error),
}

pub type NewResult<T> = std::result::Result<T, NewError>;

#[derive(Error, Debug)]
pub enum OpenError {
    #[error("Path is not a valid UTF-8 string")]
    InvalidPath,
    #[error("Database {0} does not exist")]
    DatabaseDoesNotExist(std::path::PathBuf),
    #[error("sqlx error")]
    SqlxError(#[from] sqlx::Error),
}

pub type OpenResult<T> = std::result::Result<T, OpenError>;

use sqlx::sqlite::SqlitePool;

#[derive(Debug)]
pub struct Store {
    pool: SqlitePool,
}

use serde::{Deserialize, Serialize};
use sqlx::migrate::MigrateDatabase;
use std::num::NonZeroU32;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GeneralDataDb {
    interrogations_per_week: Option<std::ops::Range<u32>>,
    max_interrogations_per_day: Option<NonZeroU32>,
}

impl Store {
    fn build_url(path: &std::path::Path) -> Option<String> {
        let filename = path.to_str()?;
        Some(format!("sqlite://{}", filename))
    }

    async fn fill_empty_db(pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::query(
            r#"
CREATE TABLE "colloscopes" (
    "colloscope_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL UNIQUE,
    PRIMARY KEY("colloscope_id" AUTOINCREMENT)
);
INSERT INTO "colloscopes" ( "name" ) VALUES ( "Colloscope_1" );

CREATE TABLE "incompats" (
    "incompat_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL,
    "max_count"	INTEGER NOT NULL,
    PRIMARY KEY("incompat_id")
);

CREATE TABLE "incompat_groups" (
    "incompat_group_id"	INTEGER NOT NULL,
    "incompat_id"	INTEGER NOT NULL,
    FOREIGN KEY("incompat_id") REFERENCES "incompats"("incompat_id"),
    PRIMARY KEY("incompat_group_id")
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

CREATE TABLE "incompat_group_items" (
    "incompat_group_id"	INTEGER NOT NULL,
    "week_pattern_id"	INTEGER NOT NULL,
    "start_day"	INTEGER NOT NULL,
    "start_time"	INTEGER NOT NULL,
    "duration"	INTEGER NOT NULL,
    FOREIGN KEY("incompat_group_id") REFERENCES "incompat_groups"("incompat_group_id"),
    PRIMARY KEY("incompat_group_id","week_pattern_id","start_day","start_time","duration"),
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
    "incompat_id"	INTEGER,
    "min_students_per_group"	INTEGER NOT NULL,
    "max_students_per_group"	INTEGER NOT NULL,
    "period"	INTEGER NOT NULL,
    "period_is_strict"	INTEGER NOT NULL,
    "is_tutorial"	INTEGER NOT NULL,
    "max_groups_per_slot"	INTEGER NOT NULL,
    "balance_teachers"	INTEGER NOT NULL,
    "balance_timeslots"	INTEGER NOT NULL,
    FOREIGN KEY("incompat_id") REFERENCES "incompats"("incompat_id"),
    FOREIGN KEY("subject_group_id") REFERENCES "subject_groups"("subject_group_id"),
    PRIMARY KEY("subject_id" AUTOINCREMENT)
);

CREATE TABLE "groupings" (
    "grouping_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL,
    PRIMARY KEY("grouping_id" AUTOINCREMENT)
);

CREATE TABLE "grouping_incompats" (
    "grouping_incompat_id"	INTEGER NOT NULL UNIQUE,
    "max_count"	INTEGER NOT NULL,
    PRIMARY KEY("grouping_incompat_id")
);

CREATE TABLE "grouping_incompat_items" (
    "grouping_incompat_id"	INTEGER NOT NULL,
    "grouping_id"	INTEGER NOT NULL,
    FOREIGN KEY("grouping_id") REFERENCES "groupings"("grouping_id"),
    FOREIGN KEY("grouping_incompat_id") REFERENCES "grouping_incompats"("grouping_incompat_id"),
    PRIMARY KEY("grouping_incompat_id","grouping_id")
);

CREATE TABLE "time_slots" (
    "time_slot_id"	INTEGER NOT NULL,
    "subject_id"	INTEGER NOT NULL,
    "teacher_id"	INTEGER NOT NULL,
    "start_day"	INTEGER NOT NULL,
    "start_time"	INTEGER NOT NULL,
    "week_pattern_id"	INTEGER NOT NULL,
    "room"	TEXT NOT NULL,
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
    FOREIGN KEY("incompat_id") REFERENCES "incompats"("incompat_id")
);

CREATE TABLE "student_subjects" (
    "student_id"	INTEGER NOT NULL,
    "subject_id"	INTEGER NOT NULL,
    FOREIGN KEY("subject_id") REFERENCES "subjects"("subject_id"),
    FOREIGN KEY("student_id") REFERENCES "students"("student_id"),
    PRIMARY KEY("subject_id","student_id")
);

CREATE TABLE "group_lists" (
    "group_list_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL,
    PRIMARY KEY("group_list_id" AUTOINCREMENT)
);

CREATE TABLE "group_list_subjects" (
    "subject_id"	INTEGER NOT NULL,
    "group_list_id"	INTEGER NOT NULL,
    FOREIGN KEY("group_list_id") REFERENCES "group_lists"("group_list_id"),
    FOREIGN KEY("subject_id") REFERENCES "subjects"("subject_id"),
    PRIMARY KEY("subject_id","group_list_id")
);

CREATE TABLE "groups" (
    "group_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL,
    "extendable"	INTEGER NOT NULL,
    UNIQUE("group_id"),
    PRIMARY KEY("group_id" AUTOINCREMENT)
);

CREATE TABLE "group_list_items" (
    "group_list_id"	INTEGER NOT NULL,
    "group_id"	INTEGER NOT NULL UNIQUE,
    FOREIGN KEY("group_list_id") REFERENCES "group_lists"("group_list_id"),
    FOREIGN KEY("group_id") REFERENCES "groups"("group_id"),
    PRIMARY KEY("group_list_id","group_id")
);

CREATE TABLE "group_items" (
    "group_list_id"	INTEGER NOT NULL,
    "group_id"	INTEGER NOT NULL,
    "student_id"	INTEGER NOT NULL,
    UNIQUE("group_list_id","student_id"),
    FOREIGN KEY("student_id") REFERENCES "students"("student_id"),
    PRIMARY KEY("student_id","group_list_id","group_id"),
    FOREIGN KEY ("group_list_id", "group_id") REFERENCES group_list_items("group_list_id", "group_id")
);"#,
        )
        .bind(serde_json::to_string(&GeneralDataDb {
            interrogations_per_week: None,
            max_interrogations_per_day: None,
        }).expect("should serialize to valid json"))
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn new_db(path: &std::path::Path) -> NewResult<Self> {
        let db_url = Self::build_url(path).ok_or(NewError::InvalidPath)?;

        if sqlx::Sqlite::database_exists(&db_url).await? {
            return Err(NewError::DatabaseAlreadyExists(path.to_path_buf()));
        }

        use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
        use std::str::FromStr;
        let options = SqliteConnectOptions::from_str(&db_url)?
            .journal_mode(SqliteJournalMode::Delete)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;

        Self::fill_empty_db(&pool).await?;

        Ok(Store { pool })
    }

    pub async fn open_db(path: &std::path::Path) -> OpenResult<Self> {
        let db_url = Self::build_url(path).ok_or(OpenError::InvalidPath)?;

        if !sqlx::Sqlite::database_exists(&db_url).await? {
            return Err(OpenError::DatabaseDoesNotExist(path.to_path_buf()));
        }

        use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
        use std::str::FromStr;
        let options =
            SqliteConnectOptions::from_str(&db_url)?.journal_mode(SqliteJournalMode::Delete);
        Ok(Store {
            pool: SqlitePool::connect_with(options).await?,
        })
    }
}

impl Store {
    pub fn get_pool(&self) -> &SqlitePool {
        &self.pool
    }
}

use super::*;

mod teachers;
mod week_patterns;

impl Storage for Store {
    type WeekPatternId = week_patterns::Id;
    type TeacherId = teachers::Id;

    type InternalError = Error;

    fn week_pattern_get(
        &self,
        index: Self::WeekPatternId,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            WeekPattern,
            IdError<Self::InternalError, Self::WeekPatternId>,
        >,
    > + Send {
        week_patterns::get(&self.pool, index)
    }
    fn week_pattern_get_all(
        &self,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            BTreeMap<Self::WeekPatternId, WeekPattern>,
            Self::InternalError,
        >,
    > + Send {
        week_patterns::get_all(&self.pool)
    }
    fn week_pattern_add(
        &self,
        pattern: WeekPattern,
    ) -> impl core::future::Future<
        Output = std::result::Result<Self::WeekPatternId, Self::InternalError>,
    > + Send {
        week_patterns::add(&self.pool, pattern)
    }
    fn week_pattern_remove(
        &self,
        index: Self::WeekPatternId,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::WeekPatternId>>,
    > + Send {
        week_patterns::remove(&self.pool, index)
    }

    fn teachers_get(
        &self,
        index: Self::TeacherId,
    ) -> impl core::future::Future<
        Output = std::result::Result<Teacher, IdError<Self::InternalError, Self::TeacherId>>,
    > + Send {
        teachers::get(&self.pool, index)
    }
    fn teachers_get_all(
        &self,
    ) -> impl core::future::Future<
        Output = std::result::Result<BTreeMap<Self::TeacherId, Teacher>, Self::InternalError>,
    > + Send {
        teachers::get_all(&self.pool)
    }
    fn teachers_add(
        &self,
        teacher: Teacher,
    ) -> impl core::future::Future<Output = std::result::Result<Self::TeacherId, Self::InternalError>>
           + Send {
        teachers::add(&self.pool, teacher)
    }
    fn teachers_remove(
        &self,
        index: Self::TeacherId,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>>,
    > + Send {
        teachers::remove(&self.pool, index)
    }
}
