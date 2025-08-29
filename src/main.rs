use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Create new database - won't override an existing one
    #[arg(short, long, default_value_t = false)]
    create: bool,
    /// Sqlite file (to open or create) that contains the database
    db: std::path::PathBuf,
    /// Select what colloscope to compute (default is the first one in the db)
    #[arg(short, long)]
    name: Option<String>,
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

    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
    use std::str::FromStr;
    let options = SqliteConnectOptions::from_str(db_url)?
        .journal_mode(SqliteJournalMode::Delete)
        .create_if_missing(true);
    let db = SqlitePool::connect_with(options).await?;

    sqlx::query(
        r#"
CREATE TABLE "colloscopes" (
    "colloscope_id"	INTEGER NOT NULL,
    "name"	TEXT NOT NULL UNIQUE,
    PRIMARY KEY("colloscope_id" AUTOINCREMENT)
);
INSERT INTO "colloscopes" ( "name" ) VALUES ( "Colloscope_1" );

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
	"course_incompat_id"	INTEGER NOT NULL,
	PRIMARY KEY("student_id","course_incompat_id"),
    FOREIGN KEY("student_id") REFERENCES "students"("student_id"),
	FOREIGN KEY("course_incompat_id") REFERENCES "course_incompats"("course_incompat_id")
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
	"colloscope_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	PRIMARY KEY("group_list_id" AUTOINCREMENT),
	FOREIGN KEY("colloscope_id") REFERENCES "colloscopes"("colloscope_id")
);

CREATE TABLE "group_list_items" (
	"group_list_id"	INTEGER NOT NULL,
	"group_id"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	"extendable"	INTEGER NOT NULL,
	FOREIGN KEY("group_list_id") REFERENCES "group_lists"("group_list_id"),
	PRIMARY KEY("group_id","group_list_id")
);

CREATE TABLE "group_list_item_items" (
	"group_list_id"	INTEGER NOT NULL,
	"group_id"	INTEGER NOT NULL,
	"student_id"	INTEGER NOT NULL,
	UNIQUE("group_list_id","student_id"),
	PRIMARY KEY("student_id","group_list_id","group_id"),
	FOREIGN KEY("group_list_id","group_id") REFERENCES "group_list_items"("group_list_id","group_id"),
	FOREIGN KEY("student_id") REFERENCES "students"("student_id")
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

    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
    use std::str::FromStr;
    let options = SqliteConnectOptions::from_str(db_url)?.journal_mode(SqliteJournalMode::Delete);
    Ok(SqlitePool::connect_with(options).await?)
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
    use std::num::NonZeroU32;

    let teacher_count_req =
        sqlx::query!("SELECT COUNT(*) AS teacher_count FROM teachers").fetch_all(db_conn);
    let week_count_req = sqlx::query!("SELECT MAX(week) AS week_max FROM weeks").fetch_one(db_conn);
    let interrogations_per_week_req =
        sqlx::query!("SELECT value FROM general_data WHERE id = ?", 1).fetch_one(db_conn);

    let teacher_count = usize::try_from(teacher_count_req.await?[0].teacher_count).unwrap();
    let week_count = match week_count_req.await?.week_max {
        Some(week_max) => NonZeroU32::new(u32::try_from(week_max).unwrap() + 1).unwrap(),
        None => NonZeroU32::new(1).unwrap(),
    };
    let general_data_db: GeneralDataDb =
        serde_json::from_str(&interrogations_per_week_req.await?.value)?;
    let interrogations_per_week = general_data_db.interrogations_per_week;

    Ok(collomatique::gen::colloscope::GeneralData {
        teacher_count,
        week_count,
        interrogations_per_week,
    })
}

#[derive(Clone, Debug)]
struct CourseIncompatRecord {
    id: i64,
    week: i64,
    start_day: i64,
    start_time: i64,
    duration: i64,
}

fn generate_incompatibility(
    id: i64,
    course_incompats_data: &Vec<CourseIncompatRecord>,
) -> Result<collomatique::gen::colloscope::Incompatibility> {
    let records_iter = course_incompats_data
        .iter()
        .filter(|x| x.id == id)
        .map(|x| {
            use collomatique::gen::colloscope::{SlotStart, SlotWithDuration};
            use collomatique::gen::time::{Time, Weekday};
            use std::num::NonZeroU32;

            Result::<SlotWithDuration>::Ok(SlotWithDuration {
                start: SlotStart {
                    week: u32::try_from(x.week)?,
                    weekday: Weekday::try_from(usize::try_from(x.start_day)?)?,
                    start_time: Time::new(u32::try_from(x.start_time)?)
                        .ok_or(anyhow!("Invalid time"))?,
                },
                duration: NonZeroU32::new(u32::try_from(x.duration)?)
                    .ok_or(anyhow!("Invalid duration"))?,
            })
        });

    let mut slots = Vec::new();
    for record in records_iter {
        slots.push(record?);
    }

    Ok(collomatique::gen::colloscope::Incompatibility { slots })
}

#[derive(Clone, Debug)]
struct IncompatibilitiesData {
    list: collomatique::gen::colloscope::IncompatibilityList,
    id_map: std::collections::BTreeMap<i64, usize>,
}

async fn generate_incompatibilies(db_conn: &SqlitePool) -> Result<IncompatibilitiesData> {
    let ids = sqlx::query!("SELECT course_incompat_id AS id FROM course_incompats")
        .fetch_all(db_conn)
        .await?;

    let id_map = ids.iter().enumerate().map(|(i, x)| (x.id, i)).collect();

    let course_incompats_data = sqlx::query_as!(
        CourseIncompatRecord,
        "
SELECT course_incompat_id AS id, week, start_day, start_time, duration
FROM course_incompat_items NATURAL JOIN weeks
        "
    )
    .fetch_all(db_conn)
    .await?;

    use collomatique::gen::colloscope::IncompatibilityList;

    let mut list = IncompatibilityList::with_capacity(ids.len());
    for x in &ids {
        list.push(generate_incompatibility(x.id, &course_incompats_data)?);
    }

    Ok(IncompatibilitiesData { list, id_map })
}

#[derive(Clone, Debug)]
struct StudentRecord {
    student_id: i64,
    course_incompat_id: Option<i64>,
}

fn generate_student(
    student_id: i64,
    student_data: &Vec<StudentRecord>,
    course_incompat_id_map: &std::collections::BTreeMap<i64, usize>,
) -> Result<collomatique::gen::colloscope::Student> {
    use std::collections::BTreeSet;

    let incompatibilities: BTreeSet<_> = student_data
        .iter()
        .filter(|x| x.student_id == student_id)
        .filter_map(|x| {
            let course_incompat_id = x.course_incompat_id?;
            Some(
                *course_incompat_id_map
                    .get(&course_incompat_id)
                    .expect("Valid course_incompat_id"),
            )
        })
        .collect();

    Ok(collomatique::gen::colloscope::Student { incompatibilities })
}

#[derive(Clone, Debug)]
struct StudentsData {
    list: collomatique::gen::colloscope::StudentList,
    id_map: std::collections::BTreeMap<i64, usize>,
}

async fn generate_students(
    db_conn: &SqlitePool,
    course_incompat_id_map: &std::collections::BTreeMap<i64, usize>,
) -> Result<StudentsData> {
    let ids = sqlx::query!("SELECT student_id AS id FROM students")
        .fetch_all(db_conn)
        .await?;

    let id_map = ids.iter().enumerate().map(|(i, x)| (x.id, i)).collect();

    let students_data = sqlx::query_as!(
        StudentRecord,
        "
SELECT student_id, course_incompat_id FROM student_incompats
UNION
SELECT student_id, course_incompat_id FROM student_subjects NATURAL JOIN subjects
WHERE course_incompat_id IS NOT NULL
        "
    )
    .fetch_all(db_conn)
    .await?;

    use collomatique::gen::colloscope::StudentList;

    let mut list = StudentList::with_capacity(ids.len());
    for x in &ids {
        list.push(generate_student(
            x.id,
            &students_data,
            course_incompat_id_map,
        )?);
    }

    Ok(StudentsData { list, id_map })
}

struct SubjectsData {
    list: collomatique::gen::colloscope::SubjectList,
    slot_map: std::collections::BTreeMap<
        i64,
        std::collections::BTreeSet<collomatique::gen::colloscope::SlotRef>,
    >,
}

async fn generate_subjects(
    db_conn: &SqlitePool,
    student_id_map: &std::collections::BTreeMap<i64, usize>,
) -> Result<SubjectsData> {
    let subject_data = sqlx::query!("SELECT subject_id AS id, duration, min_students_per_slot, max_students_per_slot, period, period_is_strict FROM subjects")
        .fetch_all(db_conn)
        .await?;
    let slots_data_req = sqlx::query!(
        "
SELECT time_slot_id AS id, subject_id, teacher_id, start_day, start_time, week
FROM time_slots NATURAL JOIN weeks"
    )
    .fetch_all(db_conn);
    let teachers_data_req =
        sqlx::query!("SELECT teacher_id AS id FROM teachers").fetch_all(db_conn);
    let student_subjects_req =
        sqlx::query!("SELECT student_id, subject_id FROM student_subjects").fetch_all(db_conn);

    let id_map: BTreeMap<_, _> = subject_data
        .iter()
        .enumerate()
        .map(|(i, x)| (x.id, i))
        .collect();

    use collomatique::gen::colloscope::SubjectList;
    let mut list: SubjectList = subject_data
        .iter()
        .map(|x| {
            use collomatique::gen::colloscope::{GroupsDesc, Subject};
            use std::num::{NonZeroU32, NonZeroUsize};

            let min = usize::try_from(x.min_students_per_slot)
                .expect("Valid usize for minimum students per slot");
            let max = usize::try_from(x.max_students_per_slot)
                .expect("Valid usize for maximum students per slot");

            let non_zero_min = NonZeroUsize::new(min).expect("Non zero minimum students per slot");
            let non_zero_max = NonZeroUsize::new(max).expect("Non zero maximum students per slot");

            let students_per_slot = non_zero_min..=non_zero_max;

            Subject {
                students_per_slot,
                period: NonZeroU32::new(
                    u32::try_from(x.period).expect("Valid u32 for subject period"),
                )
                .expect("Valid non-zero period for subject"),
                period_is_strict: x.period_is_strict != 0,
                duration: NonZeroU32::new(
                    u32::try_from(x.duration).expect("Valid u32 for subject duration"),
                )
                .expect("Valid non-zero duration for subject"),
                slots: Vec::new(),
                groups: GroupsDesc {
                    prefilled_groups: Vec::new(),
                    not_assigned: BTreeSet::new(),
                },
            }
        })
        .collect();

    use std::collections::{BTreeMap, BTreeSet};
    let mut slot_map = BTreeMap::<_, BTreeSet<_>>::new();

    let slots_data = slots_data_req.await?;
    let teachers_data = teachers_data_req.await?;
    let teacher_id_map: BTreeMap<_, _> = teachers_data
        .iter()
        .enumerate()
        .map(|(i, x)| (x.id, i))
        .collect();

    for slot in &slots_data {
        use collomatique::gen::colloscope::{SlotRef, SlotStart, SlotWithTeacher};
        use collomatique::gen::time::{Time, Weekday};

        let subject_index = id_map[&slot.subject_id];

        match slot_map.get_mut(&slot.id) {
            Some(val) => {
                val.insert(SlotRef {
                    subject: subject_index,
                    slot: list[subject_index].slots.len(),
                });
            }
            None => {
                slot_map.insert(
                    slot.id,
                    BTreeSet::from([SlotRef {
                        subject: subject_index,
                        slot: list[subject_index].slots.len(),
                    }]),
                );
            }
        }

        list[subject_index].slots.push(SlotWithTeacher {
            teacher: teacher_id_map[&slot.teacher_id],
            start: SlotStart {
                week: u32::try_from(slot.week)?,
                weekday: Weekday::try_from(usize::try_from(slot.start_day)?)?,
                start_time: Time::new(u32::try_from(slot.start_time)?)
                    .ok_or(anyhow!("Invalid time"))?,
            },
        });
    }

    let student_subjects_data = student_subjects_req.await?;

    for x in &student_subjects_data {
        let subject_index = id_map[&x.subject_id];
        let student_index = student_id_map[&x.student_id];

        list[subject_index]
            .groups
            .not_assigned
            .insert(student_index);
    }

    for subject in list.iter_mut() {
        if subject.groups.not_assigned.len() < subject.students_per_slot.start().get() {
            return Err(anyhow!("Not enough students to assign into groups"));
        }
        let full_group_count =
            subject.groups.not_assigned.len() / subject.students_per_slot.end().get();
        let remaining_students =
            subject.groups.not_assigned.len() % subject.students_per_slot.end().get();
        let group_count = if remaining_students != 0 {
            if remaining_students < subject.students_per_slot.start().get() {
                let students_to_distribute =
                    remaining_students + subject.students_per_slot.end().get();
                let extra_groups_count =
                    students_to_distribute / subject.students_per_slot.start().get();
                full_group_count + extra_groups_count - 1
            } else {
                full_group_count + 1
            }
        } else {
            full_group_count
        };

        for _i in 0..group_count {
            use collomatique::gen::colloscope::GroupDesc;

            subject.groups.prefilled_groups.push(GroupDesc {
                students: BTreeSet::new(),
                can_be_extended: true,
            });
        }
    }

    Ok(SubjectsData { list, slot_map })
}

#[derive(Debug, Clone)]
struct SlotGroupingData {
    list: collomatique::gen::colloscope::SlotGroupingList,
    id_map: std::collections::BTreeMap<i64, usize>,
}

async fn generate_slot_groupings(
    db_conn: &SqlitePool,
    slot_map: &std::collections::BTreeMap<
        i64,
        std::collections::BTreeSet<collomatique::gen::colloscope::SlotRef>,
    >,
) -> Result<SlotGroupingData> {
    let ids = sqlx::query!("SELECT grouping_id AS id FROM groupings")
        .fetch_all(db_conn)
        .await?;
    use std::collections::BTreeMap;
    let id_map: BTreeMap<_, _> = ids.iter().enumerate().map(|(i, x)| (x.id, i)).collect();

    let slot_groupings_data = sqlx::query!("SELECT grouping_id, time_slot_id FROM grouping_items")
        .fetch_all(db_conn)
        .await?;

    use collomatique::gen::colloscope::SlotGrouping;
    use std::collections::BTreeSet;
    let mut list = vec![
        SlotGrouping {
            slots: BTreeSet::new(),
        };
        ids.len()
    ];

    for x in &slot_groupings_data {
        let slot_index = id_map[&x.grouping_id];
        let slot_ref = slot_map[&x.time_slot_id].clone();

        list[slot_index].slots.extend(slot_ref);
    }

    Ok(SlotGroupingData { list, id_map })
}

async fn generate_grouping_incompats(
    db_conn: &SqlitePool,
    id_map: &std::collections::BTreeMap<i64, usize>,
) -> Result<collomatique::gen::colloscope::SlotGroupingIncompatSet> {
    use collomatique::gen::colloscope::{SlotGroupingIncompat, SlotGroupingIncompatSet};

    let grouping_incompats_data = sqlx::query!("SELECT id1, id2 FROM grouping_incompats")
        .fetch_all(db_conn)
        .await?;

    let mut set = SlotGroupingIncompatSet::new();

    for record in &grouping_incompats_data {
        let incompat = SlotGroupingIncompat::new(id_map[&record.id1], id_map[&record.id2]);
        set.insert(incompat);
    }

    Ok(set)
}

async fn get_colloscope_id(db_conn: &SqlitePool, colloscope: Option<String>) -> Result<i64> {
    match colloscope {
        Some(name) => {
            let id = sqlx::query!("SELECT colloscope_id FROM colloscopes WHERE name = ?", name)
                .fetch_optional(db_conn)
                .await?;
            id.map(|x| x.colloscope_id)
                .ok_or(anyhow!("Colloscope {} does not exist", name))
        }
        None => {
            let id = sqlx::query!("SELECT colloscope_id FROM colloscopes")
                .fetch_optional(db_conn)
                .await?;
            id.map(|x| x.colloscope_id)
                .ok_or(anyhow!("No available colloscope to fill in"))
        }
    }
}

async fn generate_colloscope_data(
    db_conn: &SqlitePool,
    colloscope: Option<String>,
) -> Result<collomatique::gen::colloscope::ValidatedData> {
    use collomatique::gen::colloscope::*;

    let collo_id = get_colloscope_id(db_conn, colloscope).await?;

    let general = generate_general_data(db_conn);
    let incompatibilities = generate_incompatibilies(db_conn).await?;
    let students = generate_students(db_conn, &incompatibilities.id_map).await?;
    let subjects = generate_subjects(db_conn, &students.id_map).await?;
    let slot_groupings = generate_slot_groupings(db_conn, &subjects.slot_map).await?;
    let grouping_incompats = generate_grouping_incompats(db_conn, &slot_groupings.id_map);

    Ok(ValidatedData::new(
        general.await?,
        subjects.list,
        incompatibilities.list,
        students.list,
        slot_groupings.list,
        grouping_incompats.await?,
    )?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let db = connect_db(args.create, args.db.as_path()).await?;

    let data = generate_colloscope_data(&db, args.name).await?;

    let ilp_translator = data.ilp_translator();
    let problem = ilp_translator.problem();

    println!("{}", problem);

    let mut sa_optimizer = collomatique::ilp::optimizers::sa::Optimizer::new(&problem);

    let mut random_gen = collomatique::ilp::random::DefaultRndGen::new();

    sa_optimizer.set_init_config(problem.random_config(&mut random_gen));
    sa_optimizer.set_max_steps(Some(1000));

    use collomatique::ilp::solvers::backtracking::heuristics::Knuth2000;
    let solver = collomatique::ilp::solvers::backtracking::Solver::new(Knuth2000::default());
    let iterator = sa_optimizer.iterate(solver, &mut random_gen);

    for (i, (sol, cost)) in iterator.enumerate() {
        println!(
            "{}: {} - {:?}",
            i,
            cost,
            ilp_translator.read_solution(sol.as_ref())
        );
    }

    Ok(())
}
