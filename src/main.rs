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
}

use sqlx::sqlite::SqlitePool;

use collomatique::backend::sqlite;

use serde::{Deserialize, Serialize};
use std::num::{NonZeroU32, NonZeroUsize};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GeneralDataDb {
    interrogations_per_week: Option<std::ops::Range<u32>>,
    max_interrogations_per_day: Option<NonZeroU32>,
}

async fn connect_db(create: bool, path: &std::path::Path) -> Result<sqlite::Store> {
    if create {
        Ok(sqlite::Store::new_db(path).await?)
    } else {
        Ok(sqlite::Store::open_db(path).await?)
    }
}

async fn generate_general_data(
    db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::GeneralData> {
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
    let max_interrogations_per_day = general_data_db.max_interrogations_per_day;

    Ok(collomatique::gen::colloscope::GeneralData {
        teacher_count,
        week_count,
        interrogations_per_week,
        max_interrogations_per_day,
    })
}

#[derive(Clone, Debug)]
struct IncompatGroupRecord {
    id: i64,
    week: i64,
    start_day: i64,
    start_time: i64,
    duration: i64,
}

#[derive(Clone, Debug)]
struct IncompatGroupListData {
    incompat_group_list: collomatique::gen::colloscope::IncompatibilityGroupList,
    id_map: std::collections::BTreeMap<i64, std::collections::BTreeMap<u32, usize>>,
}

async fn generate_incompat_group_list(db_conn: &SqlitePool) -> Result<IncompatGroupListData> {
    let incompat_group_items_data = sqlx::query_as!(
        IncompatGroupRecord,
        "
SELECT incompat_group_id AS id, week, start_day, start_time, duration
FROM incompat_group_items NATURAL JOIN weeks
        "
    )
    .fetch_all(db_conn)
    .await?;

    use collomatique::gen::colloscope::IncompatibilityGroupList;
    use std::collections::{BTreeMap, BTreeSet};

    let mut incompat_group_list = IncompatibilityGroupList::new();
    let mut id_map = BTreeMap::<_, BTreeMap<_, _>>::new();

    for x in &incompat_group_items_data {
        use collomatique::gen::colloscope::{IncompatibilityGroup, SlotStart, SlotWithDuration};
        use collomatique::gen::time::{Time, Weekday};

        let slot = SlotWithDuration {
            start: SlotStart {
                week: u32::try_from(x.week)?,
                weekday: Weekday::try_from(usize::try_from(x.start_day)?)?,
                start_time: Time::new(u32::try_from(x.start_time)?)
                    .ok_or(anyhow!("Invalid time"))?,
            },
            duration: NonZeroU32::new(u32::try_from(x.duration)?)
                .ok_or(anyhow!("Invalid duration"))?,
        };

        match id_map.get_mut(&x.id) {
            Some(week_map) => match week_map.get(&slot.start.week) {
                Some(&i) => {
                    let group: &mut IncompatibilityGroup = &mut incompat_group_list[i];
                    group.slots.insert(slot);
                }
                None => {
                    week_map.insert(slot.start.week, incompat_group_list.len());
                    incompat_group_list.push(IncompatibilityGroup {
                        slots: BTreeSet::from([slot]),
                    })
                }
            },
            None => {
                id_map.insert(
                    x.id,
                    BTreeMap::from([(slot.start.week, incompat_group_list.len())]),
                );
                incompat_group_list.push(IncompatibilityGroup {
                    slots: BTreeSet::from([slot]),
                })
            }
        }
    }

    Ok(IncompatGroupListData {
        incompat_group_list,
        id_map,
    })
}

#[derive(Clone, Debug)]
struct IncompatibilitiesData {
    incompat_list: collomatique::gen::colloscope::IncompatibilityList,
    incompat_group_list: collomatique::gen::colloscope::IncompatibilityGroupList,
    id_map: std::collections::BTreeMap<i64, std::collections::BTreeSet<usize>>,
}

async fn generate_incompatibilies(db_conn: &SqlitePool) -> Result<IncompatibilitiesData> {
    let incompat_group_list_data = generate_incompat_group_list(db_conn).await?;

    let incompat_group_list = incompat_group_list_data.incompat_group_list;
    let incompat_group_id_map = incompat_group_list_data.id_map;

    let incompats_data = sqlx::query!("SELECT incompat_id, max_count FROM incompats")
        .fetch_all(db_conn)
        .await?;

    use std::collections::{BTreeMap, BTreeSet};
    let max_count_map: BTreeMap<_, _> = incompats_data
        .into_iter()
        .map(|x| (x.incompat_id, x.max_count))
        .collect();

    let incompat_groups_data =
        sqlx::query!("SELECT incompat_id, incompat_group_id FROM incompat_groups")
            .fetch_all(db_conn)
            .await?;

    use collomatique::gen::colloscope::{Incompatibility, IncompatibilityList};

    let mut incompat_list = IncompatibilityList::new();
    let mut incompat_id_map = BTreeMap::<_, BTreeMap<_, _>>::new();

    for x in &incompat_groups_data {
        match incompat_id_map.get_mut(&x.incompat_id) {
            Some(week_map) => {
                let empty_map = BTreeMap::new();
                let group_week_map = incompat_group_id_map
                    .get(&x.incompat_group_id)
                    .unwrap_or(&empty_map);
                for (&week, &incompat_group_index) in group_week_map {
                    match week_map.get(&week) {
                        Some(&i) => {
                            let incompat: &mut Incompatibility = &mut incompat_list[i];
                            incompat.groups.insert(incompat_group_index);
                        }
                        None => {
                            week_map.insert(week, incompat_list.len());
                            let max_count_i64 =
                                max_count_map.get(&x.incompat_id).copied().ok_or(anyhow!(
                                    "Inconsistent database: non existant incompat_id referenced"
                                ))?;
                            let max_count = usize::try_from(max_count_i64)
                                .map_err(|_| anyhow!("max_count should fit in usize"))?;
                            incompat_list.push(Incompatibility {
                                groups: BTreeSet::from([incompat_group_index]),
                                max_count,
                            });
                        }
                    }
                }
            }
            None => {
                let mut week_map = BTreeMap::new();
                let empty_map = BTreeMap::new();
                let group_week_map = incompat_group_id_map
                    .get(&x.incompat_group_id)
                    .unwrap_or(&empty_map);
                for (&week, &incompat_group_index) in group_week_map {
                    week_map.insert(week, incompat_list.len());
                    let max_count_i64 = max_count_map.get(&x.incompat_id).copied().ok_or(
                        anyhow!("Inconsistent database: non existant incompat_id referenced"),
                    )?;
                    let max_count = usize::try_from(max_count_i64)
                        .map_err(|_| anyhow!("max_count should fit in usize"))?;
                    incompat_list.push(Incompatibility {
                        groups: BTreeSet::from([incompat_group_index]),
                        max_count,
                    });
                }
                incompat_id_map.insert(x.incompat_id, week_map);
            }
        }
    }

    let id_map = incompat_id_map
        .into_iter()
        .map(|(id, week_map)| {
            (
                id,
                week_map.into_iter().map(|(_week, index)| index).collect(),
            )
        })
        .collect();

    Ok(IncompatibilitiesData {
        incompat_list,
        incompat_group_list,
        id_map,
    })
}

#[derive(Clone, Debug)]
struct StudentRecord {
    student_id: i64,
    incompat_id: Option<i64>,
}

fn generate_student(
    student_id: i64,
    student_data: &Vec<StudentRecord>,
    incompat_id_map: &std::collections::BTreeMap<i64, std::collections::BTreeSet<usize>>,
) -> Result<collomatique::gen::colloscope::Student> {
    use std::collections::BTreeSet;

    let incompatibilities: BTreeSet<_> = student_data
        .iter()
        .filter(|x| x.student_id == student_id)
        .filter_map(|x| {
            let incompat_id = x.incompat_id?;
            Some(
                incompat_id_map
                    .get(&incompat_id)
                    .cloned()
                    .expect("Valid incompat_id"),
            )
        })
        .flatten()
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
    incompat_id_map: &std::collections::BTreeMap<i64, std::collections::BTreeSet<usize>>,
) -> Result<StudentsData> {
    let ids = sqlx::query!("SELECT student_id AS id FROM students")
        .fetch_all(db_conn)
        .await?;

    let id_map = ids.iter().enumerate().map(|(i, x)| (x.id, i)).collect();

    let students_data = sqlx::query_as!(
        StudentRecord,
        "
SELECT student_id, incompat_id FROM student_incompats
UNION
SELECT student_id, incompat_id FROM student_subjects NATURAL JOIN subjects
WHERE incompat_id IS NOT NULL
        "
    )
    .fetch_all(db_conn)
    .await?;

    use collomatique::gen::colloscope::StudentList;

    let mut list = StudentList::with_capacity(ids.len());
    for x in &ids {
        list.push(generate_student(x.id, &students_data, incompat_id_map)?);
    }

    Ok(StudentsData { list, id_map })
}

#[derive(Clone, Debug)]
struct SubjectRecord {
    id: i64,
    duration: i64,
    min_students_per_group: i64,
    max_students_per_group: i64,
    period: i64,
    period_is_strict: i64,
    is_tutorial: i64,
    max_groups_per_slot: i64,
    balance_teachers: i64,
    balance_timeslots: i64,
}

fn generate_bare_subjects(
    subject_data: &[SubjectRecord],
) -> (
    collomatique::gen::colloscope::SubjectList,
    std::collections::BTreeMap<i64, usize>,
) {
    use std::collections::BTreeMap;
    let id_map: BTreeMap<_, _> = subject_data
        .iter()
        .enumerate()
        .map(|(i, x)| (x.id, i))
        .collect();

    use std::collections::BTreeSet;
    let subjects = subject_data
        .iter()
        .map(|x| {
            use collomatique::gen::colloscope::{GroupsDesc, Subject};

            let min = usize::try_from(x.min_students_per_group)
                .expect("Valid usize for minimum students per group");
            let max = usize::try_from(x.max_students_per_group)
                .expect("Valid usize for maximum students per group");

            let non_zero_min = NonZeroUsize::new(min).expect("Non zero minimum students per group");
            let non_zero_max = NonZeroUsize::new(max).expect("Non zero maximum students per group");

            let students_per_group = non_zero_min..=non_zero_max;

            let max_groups_per_slot_usize = usize::try_from(x.max_groups_per_slot)
                .expect("Valid usize for maximum groups per slot");
            let max_groups_per_slot = NonZeroUsize::new(max_groups_per_slot_usize)
                .expect("Non zero maximum groups per slot");

            Subject {
                students_per_group,
                max_groups_per_slot,
                period: NonZeroU32::new(
                    u32::try_from(x.period).expect("Valid u32 for subject period"),
                )
                .expect("Valid non-zero period for subject"),
                period_is_strict: x.period_is_strict != 0,
                is_tutorial: x.is_tutorial != 0,
                balancing_requirements: collomatique::gen::colloscope::BalancingRequirements {
                    teachers: x.balance_teachers != 0,
                    timeslots: x.balance_timeslots != 0,
                },
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

    (subjects, id_map)
}

#[derive(Clone, Debug)]
struct SlotRecord {
    id: i64,
    subject_id: i64,
    teacher_id: i64,
    start_day: i64,
    start_time: i64,
    week: i64,
}

#[derive(Clone, Debug)]
struct TeacherRecord {
    id: i64,
}

fn add_slots_to_subjects(
    subjects: &mut collomatique::gen::colloscope::SubjectList,
    slots_data: &[SlotRecord],
    teachers_data: &[TeacherRecord],
    subject_id_map: &std::collections::BTreeMap<i64, usize>,
) -> Result<
    std::collections::BTreeMap<
        i64,
        std::collections::BTreeMap<u32, collomatique::gen::colloscope::SlotRef>,
    >,
> {
    use std::collections::BTreeMap;
    let mut slot_map = BTreeMap::<_, BTreeMap<_, _>>::new();

    let teacher_id_map: BTreeMap<_, _> = teachers_data
        .iter()
        .enumerate()
        .map(|(i, x)| (x.id, i))
        .collect();

    for slot in slots_data {
        use collomatique::gen::colloscope::{SlotRef, SlotStart, SlotWithTeacher};
        use collomatique::gen::time::{Time, Weekday};

        let subject_index = subject_id_map[&slot.subject_id];

        let week = u32::try_from(slot.week)?;

        match slot_map.get_mut(&slot.id) {
            Some(val) => {
                val.insert(
                    week,
                    SlotRef {
                        subject: subject_index,
                        slot: subjects[subject_index].slots.len(),
                    },
                );
            }
            None => {
                slot_map.insert(
                    slot.id,
                    BTreeMap::from([(
                        week,
                        SlotRef {
                            subject: subject_index,
                            slot: subjects[subject_index].slots.len(),
                        },
                    )]),
                );
            }
        }

        subjects[subject_index].slots.push(SlotWithTeacher {
            teacher: teacher_id_map[&slot.teacher_id],
            start: SlotStart {
                week,
                weekday: Weekday::try_from(usize::try_from(slot.start_day)?)?,
                start_time: Time::new(u32::try_from(slot.start_time)?)
                    .ok_or(anyhow!("Invalid time"))?,
            },
        });
    }

    Ok(slot_map)
}

#[derive(Clone, Debug)]
struct StudentSubjectRecord {
    student_id: i64,
    subject_id: i64,
}

#[derive(Clone, Debug)]
struct GroupInfo {
    students: std::collections::BTreeSet<usize>,
    is_extendable: bool,
}

fn add_students_to_subjects(
    subjects: &mut collomatique::gen::colloscope::SubjectList,
    student_subjects_data: &[StudentSubjectRecord],
    subject_id_map: &std::collections::BTreeMap<i64, usize>,
    student_id_map: &std::collections::BTreeMap<i64, usize>,
    group_infos: &std::collections::BTreeMap<i64, Vec<GroupInfo>>,
) -> Result<()> {
    for x in student_subjects_data {
        let subject_index = subject_id_map[&x.subject_id];
        let student_index = student_id_map[&x.student_id];

        subjects[subject_index]
            .groups
            .not_assigned
            .insert(student_index);
    }

    for (subject_id, group_info) in group_infos {
        let subject_index = student_id_map[&subject_id];

        for x in group_info {
            use std::collections::BTreeSet;
            let mut group = collomatique::gen::colloscope::GroupDesc {
                students: BTreeSet::new(),
                can_be_extended: x.is_extendable,
            };

            for student in &x.students {
                // Ignore students that are not assigned to the subject
                if !subjects[subject_index]
                    .groups
                    .not_assigned
                    .contains(student)
                {
                    continue;
                }

                subjects[subject_index].groups.not_assigned.remove(student);
                group.students.insert(*student);
            }

            // The new group might be empty after ignoring students
            // If not extensible, we should remove it entirely
            if group.students.is_empty() && !group.can_be_extended {
                continue;
            }
            subjects[subject_index].groups.prefilled_groups.push(group);
        }
    }

    /*for subject in subjects.iter_mut() {
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
            use std::collections::BTreeSet;

            subject.groups.prefilled_groups.push(GroupDesc {
                students: BTreeSet::new(),
                can_be_extended: true,
            });
        }
    }*/

    Ok(())
}

#[derive(Clone, Debug)]
struct StudentInGroupRecord {
    subject_id: i64,
    group_id: i64,
    student_id: i64,
}

#[derive(Clone, Debug)]
struct GroupRecord {
    subject_id: Option<i64>,
    group_id: i64,
    extendable: i64,
}

fn construct_group_info(
    students_in_group_data: &[StudentInGroupRecord],
    groups_data: &[GroupRecord],
    student_id_map: &std::collections::BTreeMap<i64, usize>,
) -> Result<std::collections::BTreeMap<i64, Vec<GroupInfo>>> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut temp_group_infos: BTreeMap<i64, BTreeMap<i64, _>> = BTreeMap::new();

    for group_record in groups_data {
        match temp_group_infos.get_mut(
            &group_record.subject_id.expect(
                "Why this is an option is beyond me, but if this fails, you have the reason",
            ),
        ) {
            Some(map) => {
                map.insert(
                    group_record.group_id,
                    GroupInfo {
                        students: BTreeSet::new(),
                        is_extendable: group_record.extendable == 1,
                    },
                );
            }
            None => {
                temp_group_infos.insert(
                    group_record.subject_id.expect("Why this is an option is beyond me, but if this fails, you have the reason"),
                    BTreeMap::from([
                        (
                            group_record.group_id,
                            GroupInfo {
                                students: BTreeSet::new(),
                                is_extendable: group_record.extendable == 1,
                            }
                        )
                    ])
                );
            }
        }
    }

    for student_in_group_record in students_in_group_data {
        let subject_info = temp_group_infos
            .get_mut(&student_in_group_record.subject_id)
            .ok_or(anyhow!("Non existent subject referenced for student"))?;

        let group_info = subject_info
            .get_mut(&student_in_group_record.group_id)
            .ok_or(anyhow!("Non existent group referenced for student"))?;

        group_info
            .students
            .insert(student_id_map[&student_in_group_record.student_id]);
    }

    Ok(temp_group_infos
        .into_iter()
        .map(|(subject, subject_info)| {
            (
                subject,
                subject_info
                    .into_iter()
                    .map(|(_group_id, group_info)| group_info)
                    .collect(),
            )
        })
        .collect())
}

#[derive(Clone, Debug)]
struct SubjectsData {
    list: collomatique::gen::colloscope::SubjectList,
    slot_map: std::collections::BTreeMap<
        i64,
        std::collections::BTreeMap<u32, collomatique::gen::colloscope::SlotRef>,
    >,
}

async fn generate_subjects(
    db_conn: &SqlitePool,
    student_id_map: &std::collections::BTreeMap<i64, usize>,
) -> Result<SubjectsData> {
    let subject_data = sqlx::query_as!(SubjectRecord, "
SELECT subject_id AS id, duration, min_students_per_group, max_students_per_group, period, period_is_strict, is_tutorial, max_groups_per_slot, balance_teachers, balance_timeslots
FROM subjects
        ")
        .fetch_all(db_conn)
        .await?;
    let slots_data_req = sqlx::query_as!(
        SlotRecord,
        "
SELECT time_slot_id AS id, subject_id, teacher_id, start_day, start_time, week
FROM time_slots NATURAL JOIN weeks"
    )
    .fetch_all(db_conn);
    let teachers_data_req =
        sqlx::query_as!(TeacherRecord, "SELECT teacher_id AS id FROM teachers").fetch_all(db_conn);
    let student_subjects_req = sqlx::query_as!(
        StudentSubjectRecord,
        "SELECT student_id, subject_id FROM student_subjects"
    )
    .fetch_all(db_conn);
    let students_in_group_req = sqlx::query_as!(
        StudentInGroupRecord,
        "
SELECT subject_id, group_id, student_id
FROM group_list_subjects 
JOIN group_lists ON group_list_subjects.group_list_id = group_lists.group_list_id
JOIN group_items ON group_lists.group_list_id = group_items.group_list_id;
        "
    )
    .fetch_all(db_conn);
    let groups_data_req = sqlx::query_as!(
        GroupRecord,
        "
SELECT subject_id, group_list_items.group_id, extendable
FROM group_list_subjects
JOIN group_list_items ON group_list_subjects.group_list_id = group_list_items.group_list_id
JOIN groups ON group_list_items.group_id = groups.group_id
JOIN group_lists On group_lists.group_list_id = group_list_subjects.group_list_id;
        "
    )
    .fetch_all(db_conn);

    let (mut list, subject_id_map) = generate_bare_subjects(&subject_data[..]);

    let slots_data = slots_data_req.await?;
    let teachers_data = teachers_data_req.await?;
    let slot_map = add_slots_to_subjects(
        &mut list,
        &slots_data[..],
        &teachers_data[..],
        &subject_id_map,
    )?;

    let students_in_group_data = students_in_group_req.await?;
    let groups_data = groups_data_req.await?;
    let group_infos = construct_group_info(
        &students_in_group_data[..],
        &groups_data[..],
        student_id_map,
    )?;

    let student_subjects_data = student_subjects_req.await?;
    add_students_to_subjects(
        &mut list,
        &student_subjects_data[..],
        &subject_id_map,
        student_id_map,
        &group_infos,
    )?;

    Ok(SubjectsData { list, slot_map })
}

#[derive(Debug, Clone)]
struct SlotGroupingData {
    list: collomatique::gen::colloscope::SlotGroupingList,
    id_map: std::collections::BTreeMap<i64, std::collections::BTreeMap<u32, usize>>,
}

async fn generate_slot_groupings(
    db_conn: &SqlitePool,
    slot_map: &std::collections::BTreeMap<
        i64,
        std::collections::BTreeMap<u32, collomatique::gen::colloscope::SlotRef>,
    >,
) -> Result<SlotGroupingData> {
    let slot_groupings_data = sqlx::query!("SELECT grouping_id, time_slot_id FROM grouping_items")
        .fetch_all(db_conn)
        .await?;

    use collomatique::gen::colloscope::SlotGrouping;
    use std::collections::{BTreeMap, BTreeSet};
    let mut grouping_map: BTreeMap<i64, BTreeMap<u32, SlotGrouping>> = BTreeMap::new();

    for x in &slot_groupings_data {
        let slot_refs = &slot_map[&x.time_slot_id];

        match grouping_map.get_mut(&x.grouping_id) {
            Some(week_map) => {
                for (week, slot_ref) in slot_refs {
                    match week_map.get_mut(week) {
                        Some(slot_set) => {
                            slot_set.slots.insert(slot_ref.clone());
                        }
                        None => {
                            week_map.insert(
                                *week,
                                SlotGrouping {
                                    slots: BTreeSet::from([slot_ref.clone()]),
                                },
                            );
                        }
                    }
                }
            }
            None => {
                let week_grouping: BTreeMap<u32, SlotGrouping> = slot_refs
                    .iter()
                    .map(|(week, slot_ref)| {
                        (
                            *week,
                            SlotGrouping {
                                slots: BTreeSet::from([slot_ref.clone()]),
                            },
                        )
                    })
                    .collect();

                grouping_map.insert(x.grouping_id, week_grouping);
            }
        }
    }

    let mut id_map = BTreeMap::new();
    let mut list = vec![];
    for (grouping, week_map) in grouping_map {
        let mut week_id_map = BTreeMap::new();

        for (week, slots) in week_map {
            week_id_map.insert(week, list.len());
            list.push(slots);
        }

        id_map.insert(grouping, week_id_map);
    }

    Ok(SlotGroupingData { list, id_map })
}

async fn generate_grouping_incompats(
    db_conn: &SqlitePool,
    id_map: &std::collections::BTreeMap<i64, std::collections::BTreeMap<u32, usize>>,
) -> Result<collomatique::gen::colloscope::SlotGroupingIncompatSet> {
    use collomatique::gen::colloscope::{SlotGroupingIncompat, SlotGroupingIncompatSet};

    let grouping_incompats_data =
        sqlx::query!("SELECT grouping_incompat_id, max_count FROM grouping_incompats")
            .fetch_all(db_conn)
            .await?;
    let grouping_incompat_items_data =
        sqlx::query!("SELECT grouping_incompat_id, grouping_id FROM grouping_incompat_items")
            .fetch_all(db_conn)
            .await?;

    let mut set = SlotGroupingIncompatSet::new();

    for record in &grouping_incompats_data {
        let mut week_maps = Vec::new();
        use std::collections::BTreeSet;
        let mut weeks = BTreeSet::new();
        for x in &grouping_incompat_items_data {
            if x.grouping_incompat_id != record.grouping_incompat_id {
                continue;
            }
            let week_map = id_map
                .get(&x.grouping_id)
                .ok_or(anyhow!("Invalid grouping ID"))?;
            weeks.extend(week_map.keys().copied());
            week_maps.push(week_map);
        }

        let max_count_maybe_zero = usize::try_from(record.max_count)
            .map_err(|_| anyhow!("Invalid (non usize) max count"))?;
        let max_count =
            NonZeroUsize::new(max_count_maybe_zero).ok_or(anyhow!("Invalid (zero) max count"))?;

        for week in weeks {
            let groupings: BTreeSet<_> = week_maps
                .iter()
                .filter_map(|&week_map| week_map.get(&week).copied())
                .collect();

            if groupings.len() <= max_count.get() {
                continue;
            }

            let incompat = SlotGroupingIncompat {
                groupings,
                max_count,
            };
            set.insert(incompat);
        }
    }

    Ok(set)
}

/*async fn get_colloscope_id(db_conn: &SqlitePool, colloscope: Option<String>) -> Result<i64> {
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
}*/

async fn generate_colloscope_data(
    db_conn: &SqlitePool,
) -> Result<collomatique::gen::colloscope::ValidatedData> {
    use collomatique::gen::colloscope::*;

    let general = generate_general_data(db_conn);
    let incompatibilities = generate_incompatibilies(db_conn).await?;
    let students = generate_students(db_conn, &incompatibilities.id_map).await?;
    let subjects = generate_subjects(db_conn, &students.id_map).await?;
    let slot_groupings = generate_slot_groupings(db_conn, &subjects.slot_map).await?;
    let grouping_incompats = generate_grouping_incompats(db_conn, &slot_groupings.id_map);

    Ok(ValidatedData::new(
        general.await?,
        subjects.list,
        incompatibilities.incompat_group_list,
        incompatibilities.incompat_list,
        students.list,
        slot_groupings.list,
        grouping_incompats.await?,
    )?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Opening database...");
    let storage = connect_db(args.create, args.db.as_path()).await?;

    use collomatique::backend::Storage;
    println!("{:?}", storage.week_pattern_get_all().await?);

    /*let pool = storage.get_pool();

    let data = generate_colloscope_data(pool).await?;

    let ilp_translator = data.ilp_translator();

    println!("Generating ILP problem...");
    let problem = ilp_translator
        .problem_builder()
        .eval_fn(collomatique::debuggable!(|x| {
            if !x
                .get(&collomatique::gen::colloscope::Variable::GroupInSlot {
                    subject: 0,
                    slot: 0,
                    group: 0,
                })
                .unwrap()
            {
                100.
            } else {
                0.
            }
        }))
        .build();

    println!("{}", problem);*/

    /*let genetic_optimizer = collomatique::ilp::optimizers::genetic::Optimizer::new(&problem);

    let general_initializer = collomatique::ilp::initializers::Random::with_p(
        collomatique::ilp::random::DefaultRndGen::new(),
        0.01,
    )
    .unwrap();
    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let max_steps = None;
    let retries = 20;
    let initializer =
        ilp_translator.incremental_initializer(general_initializer, solver, max_steps, retries);

    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let random_gen = collomatique::ilp::random::DefaultRndGen::new();
    let iterator = genetic_optimizer.iterate(
        initializer,
        solver,
        random_gen.clone(),
        collomatique::ilp::optimizers::genetic::RandomCrossingPolicy::new(random_gen.clone()),
        collomatique::ilp::optimizers::RandomMutationPolicy::new(random_gen.clone(), 0.01),
    )?;

    for population in iterator {
        let m = population.last().unwrap();
        eprintln!(
            "solution: {:?}\nscore: {}",
            ilp_translator.read_solution(&m.config),
            m.cost
        );
    }*/

    /*let general_initializer = collomatique::ilp::initializers::Random::with_p(
        collomatique::ilp::random::DefaultRndGen::new(),
        0.01,
    )
    .unwrap();
    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let max_steps = None;
    let retries = 20;
    let incremental_initializer =
        ilp_translator.incremental_initializer(general_initializer, solver, max_steps, retries);
    let random_gen = collomatique::ilp::random::DefaultRndGen::new();

    let variable_count = problem.get_variables().len();
    let p = 2. / (variable_count as f64);

    use collomatique::ilp::initializers::ConfigInitializer;
    let init_config = incremental_initializer.build_init_config(&problem);
    let sa_optimizer = collomatique::ilp::optimizers::sa::Optimizer::new(init_config);

    let solver = collomatique::ilp::solvers::coin_cbc::Solver::new();
    let mutation_policy =
        collomatique::ilp::optimizers::RandomMutationPolicy::new(random_gen.clone(), p);
    let iterator = sa_optimizer.iterate(solver, random_gen.clone(), mutation_policy);

    for (i, (sol, cost)) in iterator.enumerate() {
        eprintln!(
            "{}: {} - {:?}",
            i,
            cost,
            ilp_translator.read_solution(sol.as_ref())
        );
    }*/

    Ok(())
}
