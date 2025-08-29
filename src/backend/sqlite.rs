#[cfg(test)]
mod tests;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlx error")]
    SqlxError(#[from] sqlx::Error),
    #[error("Corrupted database: {0}")]
    CorruptedDatabase(String),
    #[error("Cannot represent some data in database: {0}")]
    RepresentationError(String),
    #[error("json error")]
    JsonError(#[from] serde_json::Error),
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

CREATE TABLE "group_lists" (
    "group_list_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL,
    PRIMARY KEY("group_list_id" AUTOINCREMENT)
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
	"group_list_id"	INTEGER,
	FOREIGN KEY("group_list_id") REFERENCES "group_lists"("group_list_id"),
	FOREIGN KEY("subject_group_id") REFERENCES "subject_groups"("subject_group_id"),
	FOREIGN KEY("incompat_id") REFERENCES "incompats"("incompat_id"),
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

mod group_lists;
mod incompats;
mod students;
mod subject_groups;
mod subjects;
mod teachers;
mod week_patterns;

impl Storage for Store {
    type WeekPatternId = week_patterns::Id;
    type TeacherId = teachers::Id;
    type StudentId = students::Id;
    type SubjectGroupId = subject_groups::Id;
    type IncompatId = incompats::Id;
    type GroupListId = group_lists::Id;
    type SubjectId = subjects::Id;

    type InternalError = Error;

    async fn general_data_set(
        &self,
        general_data: &GeneralData,
    ) -> std::result::Result<(), Self::InternalError> {
        let general_data_json = GeneralDataDb {
            interrogations_per_week: general_data.interrogations_per_week.clone(),
            max_interrogations_per_day: general_data.max_interrogations_per_day.clone(),
        };

        let mut conn = self.pool.acquire().await.map_err(Error::from)?;

        let general_data_id = 1;
        let general_data_string = serde_json::to_string(&general_data_json)?;
        let rows_affected = sqlx::query!(
            "UPDATE general_data SET value = ?1 WHERE id = ?2",
            general_data_string,
            general_data_id,
        )
        .execute(&mut *conn)
        .await
        .map_err(Error::from)?
        .rows_affected();

        if rows_affected > 1 {
            return Err(Error::CorruptedDatabase(format!(
                "Multiple general_data with id {:?}",
                1
            )));
        } else if rows_affected == 0 {
            return Err(Error::CorruptedDatabase(format!(
                "No general_data with id {:?}",
                1
            )));
        }

        Ok(())
    }
    async fn general_data_get(&self) -> std::result::Result<GeneralData, Self::InternalError> {
        let general_data_id = 1;
        let record_opt = sqlx::query!(
            "SELECT value FROM general_data WHERE id = ?",
            general_data_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let record = record_opt.ok_or(Error::CorruptedDatabase(format!(
            "No general_data with id {:?}",
            1
        )))?;

        let general_data_json: GeneralDataDb = serde_json::from_str(&record.value)?;

        let general_data = GeneralData {
            interrogations_per_week: general_data_json.interrogations_per_week,
            max_interrogations_per_day: general_data_json.max_interrogations_per_day,
        };

        Ok(general_data)
    }

    fn week_patterns_get(
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
    fn week_patterns_get_all(
        &self,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            BTreeMap<Self::WeekPatternId, WeekPattern>,
            Self::InternalError,
        >,
    > + Send {
        week_patterns::get_all(&self.pool)
    }
    fn week_patterns_add(
        &self,
        pattern: &WeekPattern,
    ) -> impl core::future::Future<
        Output = std::result::Result<Self::WeekPatternId, Self::InternalError>,
    > + Send {
        week_patterns::add(&self.pool, pattern)
    }
    fn week_patterns_remove(
        &self,
        index: Self::WeekPatternId,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::WeekPatternId>>,
    > + Send {
        week_patterns::remove(&self.pool, index)
    }
    fn week_patterns_update(
        &self,
        index: Self::WeekPatternId,
        pattern: &WeekPattern,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::WeekPatternId>>,
    > + Send {
        week_patterns::update(&self.pool, index, pattern)
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
        teacher: &Teacher,
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
    fn teachers_update(
        &self,
        index: Self::TeacherId,
        teacher: &Teacher,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>>,
    > + Send {
        teachers::update(&self.pool, index, teacher)
    }

    fn students_get(
        &self,
        index: Self::StudentId,
    ) -> impl core::future::Future<
        Output = std::result::Result<Student, IdError<Self::InternalError, Self::StudentId>>,
    > + Send {
        students::get(&self.pool, index)
    }
    fn students_get_all(
        &self,
    ) -> impl core::future::Future<
        Output = std::result::Result<BTreeMap<Self::StudentId, Student>, Self::InternalError>,
    > + Send {
        students::get_all(&self.pool)
    }
    fn students_add(
        &self,
        student: &Student,
    ) -> impl core::future::Future<Output = std::result::Result<Self::StudentId, Self::InternalError>>
           + Send {
        students::add(&self.pool, student)
    }
    fn students_remove(
        &self,
        index: Self::StudentId,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::StudentId>>,
    > + Send {
        students::remove(&self.pool, index)
    }
    fn students_update(
        &self,
        index: Self::StudentId,
        student: &Student,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::StudentId>>,
    > + Send {
        students::update(&self.pool, index, student)
    }

    fn subject_groups_get(
        &self,
        index: Self::SubjectGroupId,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            SubjectGroup,
            IdError<Self::InternalError, Self::SubjectGroupId>,
        >,
    > + Send {
        subject_groups::get(&self.pool, index)
    }
    fn subject_groups_get_all(
        &self,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            BTreeMap<Self::SubjectGroupId, SubjectGroup>,
            Self::InternalError,
        >,
    > + Send {
        subject_groups::get_all(&self.pool)
    }
    fn subject_groups_add(
        &self,
        subject_group: &SubjectGroup,
    ) -> impl core::future::Future<
        Output = std::result::Result<Self::SubjectGroupId, Self::InternalError>,
    > + Send {
        subject_groups::add(&self.pool, subject_group)
    }
    fn subject_groups_remove(
        &self,
        index: Self::SubjectGroupId,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>,
    > + Send {
        subject_groups::remove(&self.pool, index)
    }
    fn subject_groups_update(
        &self,
        index: Self::SubjectGroupId,
        subject_group: &SubjectGroup,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>,
    > + Send {
        subject_groups::update(&self.pool, index, subject_group)
    }

    fn incompats_get(
        &self,
        index: Self::IncompatId,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            Incompat<Self::WeekPatternId>,
            IdError<Self::InternalError, Self::IncompatId>,
        >,
    > + Send {
        incompats::get(&self.pool, index)
    }
    fn incompats_get_all(
        &self,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            BTreeMap<Self::IncompatId, Incompat<Self::WeekPatternId>>,
            Self::InternalError,
        >,
    > + Send {
        incompats::get_all(&self.pool)
    }
    fn incompats_add(
        &self,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            Self::IncompatId,
            CrossError<Self::InternalError, Self::WeekPatternId>,
        >,
    > + Send {
        incompats::add(&self.pool, incompat)
    }
    fn incompats_remove(
        &self,
        index: Self::IncompatId,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::IncompatId>>,
    > + Send {
        incompats::remove(&self.pool, index)
    }
    fn incompats_update(
        &self,
        index: Self::IncompatId,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            (),
            CrossIdError<Self::InternalError, Self::IncompatId, Self::WeekPatternId>,
        >,
    > + Send {
        incompats::update(&self.pool, index, incompat)
    }

    fn group_lists_get(
        &self,
        index: Self::GroupListId,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            GroupList<Self::StudentId>,
            IdError<Self::InternalError, Self::GroupListId>,
        >,
    > + Send {
        group_lists::get(&self.pool, index)
    }
    fn group_lists_get_all(
        &self,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            BTreeMap<Self::GroupListId, GroupList<Self::StudentId>>,
            Self::InternalError,
        >,
    > + Send {
        group_lists::get_all(&self.pool)
    }
    fn group_lists_add(
        &self,
        group_list: &GroupList<Self::StudentId>,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            Self::GroupListId,
            InvalidCrossError<Self::InternalError, GroupList<Self::StudentId>, Self::StudentId>,
        >,
    > + Send {
        group_lists::add(&self.pool, group_list)
    }
    fn group_lists_remove(
        &self,
        index: Self::GroupListId,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::GroupListId>>,
    > + Send {
        group_lists::remove(&self.pool, index)
    }
    fn group_lists_update(
        &self,
        index: Self::GroupListId,
        group_list: &GroupList<Self::StudentId>,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            (),
            InvalidCrossIdError<
                Self::InternalError,
                GroupList<Self::StudentId>,
                Self::GroupListId,
                Self::StudentId,
            >,
        >,
    > + Send {
        group_lists::update(&self.pool, index, group_list)
    }

    fn subjects_get_all(
        &self,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            BTreeMap<
                Self::SubjectId,
                Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
            >,
            Self::InternalError,
        >,
    > + Send {
        subjects::get_all(&self.pool)
    }
    fn subjects_get(
        &self,
        index: Self::SubjectId,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
            IdError<Self::InternalError, Self::SubjectId>,
        >,
    > + Send {
        subjects::get(&self.pool, index)
    }
    fn subjects_add(
        &self,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            Self::SubjectId,
            Cross3Error<
                Self::InternalError,
                Self::SubjectGroupId,
                Self::IncompatId,
                Self::GroupListId,
            >,
        >,
    > + Send {
        subjects::add(&self.pool, subject)
    }
    fn subjects_remove(
        &self,
        index: Self::SubjectId,
    ) -> impl core::future::Future<
        Output = std::result::Result<(), IdError<Self::InternalError, Self::SubjectId>>,
    > + Send {
        subjects::remove(&self.pool, index)
    }
    fn subjects_update(
        &self,
        index: Self::SubjectId,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            (),
            Cross3IdError<
                Self::InternalError,
                Self::SubjectId,
                Self::SubjectGroupId,
                Self::IncompatId,
                Self::GroupListId,
            >,
        >,
    > + Send {
        subjects::update(&self.pool, index, subject)
    }
}
