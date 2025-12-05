use crate::tools;

use super::objects::{GroupId, InterrogationData, WeekId};
use super::tools::*;
use collo_ml::{EvalObject, ViewBuilder, ViewObject};
use collomatique_state_colloscopes::{Data, GroupListId, PeriodId, StudentId, SubjectId};
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct Env {
    pub data: Data,
    pub ignore_prefill_for_group_lists: BTreeSet<GroupListId>,
}

impl From<Data> for Env {
    fn from(value: Data) -> Self {
        Env {
            data: value,
            ignore_prefill_for_group_lists: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(Env)]
#[cached]
pub enum ObjectId {
    Interrogation(InterrogationData),
    Week(WeekId),
    GroupList(GroupListId),
    Subject(SubjectId),
    Student(StudentId),
    Period(PeriodId),
    Group(GroupId),
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Interrogation {
    subject: SubjectId,
    week: WeekId,
    group_list: GroupListId,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Week {
    num: i32,
    period: PeriodId,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct GroupList {
    groups: Vec<GroupId>,
    complete_students: Vec<StudentId>,
    students: Vec<StudentId>,
    min_student_per_group: i32,
    max_student_per_group: i32,
    max_group_count: i32,
    min_group_count: i32,
    already_counted_students: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Group {
    group_list: GroupListId,
    num: i32,
    next: GroupId,
    prefilled_students: Vec<StudentId>,
    sealed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Subject {
    max_group_per_interrogation: i32,
    min_group_per_interrogation: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
#[pretty("{firstname} {surname}")]
pub struct Student {
    #[hidden]
    firstname: String,
    #[hidden]
    surname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Period {
    weeks: Vec<WeekId>,
}

impl ViewBuilder<Env, InterrogationData> for ObjectId {
    type Object = Interrogation;

    fn enumerate(env: &Env) -> BTreeSet<InterrogationData> {
        let mut output = BTreeSet::new();
        for (_subject_id, subject_slots) in &env.data.get_inner_data().params.slots.subject_map {
            for (slot_id, slot_desc) in &subject_slots.ordered_slots {
                let week_pattern = tools::extract_week_pattern(&env.data, slot_desc.week_pattern);
                for (week, status) in week_pattern.into_iter().enumerate() {
                    if !status {
                        continue;
                    }
                    output.insert(InterrogationData {
                        slot: *slot_id,
                        week,
                    });
                }
            }
        }
        output
    }

    fn build(env: &Env, id: &InterrogationData) -> Option<Self::Object> {
        let (subject_id, _pos) = env
            .data
            .get_inner_data()
            .params
            .slots
            .find_slot_subject_and_position(id.slot)?;
        let (period_id, _) = week_to_period_id(&env.data.get_inner_data().params, id.week)?;
        let period_associations = env
            .data
            .get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .get(&period_id)?;
        let group_list_id = period_associations.get(&subject_id)?;

        Some(Interrogation {
            subject: subject_id,
            week: WeekId(id.week),
            group_list: *group_list_id,
        })
    }
}

impl ViewBuilder<Env, WeekId> for ObjectId {
    type Object = Week;

    fn enumerate(env: &Env) -> BTreeSet<WeekId> {
        let mut output = BTreeSet::new();

        let mut current_first_week = 0usize;
        for (_period_id, period_desc) in
            &env.data.get_inner_data().params.periods.ordered_period_list
        {
            for (num, week_desc) in period_desc.iter().enumerate() {
                if !week_desc.interrogations {
                    continue;
                }
                output.insert(WeekId(current_first_week + num));
            }
            current_first_week += period_desc.len();
        }

        output
    }

    fn build(env: &Env, id: &WeekId) -> Option<Self::Object> {
        let (period, _) = tools::week_to_period_id(&env.data.get_inner_data().params, id.0)?;

        Some(Week {
            num: id.0 as i32,
            period,
        })
    }
}

impl ViewBuilder<Env, GroupListId> for ObjectId {
    type Object = GroupList;

    fn enumerate(env: &Env) -> BTreeSet<GroupListId> {
        env.data
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .keys()
            .map(|x| *x)
            .collect()
    }

    fn build(env: &Env, id: &GroupListId) -> Option<Self::Object> {
        let group_list_data = env
            .data
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(id)?;

        let complete_students: Vec<_> = env
            .data
            .get_inner_data()
            .params
            .students
            .student_map
            .keys()
            .copied()
            .filter(|x| !group_list_data.params.excluded_students.contains(x))
            .collect();
        let students = if env.ignore_prefill_for_group_lists.contains(id) {
            complete_students.clone()
        } else {
            complete_students
                .iter()
                .copied()
                .filter(|x| !group_list_data.prefilled_groups.contains_student(*x))
                .collect()
        };
        Some(GroupList {
            groups: GroupList::enumerate_groups(env, id)?,
            already_counted_students: (complete_students.len() - students.len()) as i32,
            complete_students,
            students,
            min_student_per_group: group_list_data.params.students_per_group.start().get() as i32,
            max_student_per_group: group_list_data.params.students_per_group.end().get() as i32,
            min_group_count: *group_list_data.params.group_count.start() as i32,
            max_group_count: *group_list_data.params.group_count.end() as i32,
        })
    }
}

impl GroupList {
    fn enumerate_groups(env: &Env, group_list_id: &GroupListId) -> Option<Vec<GroupId>> {
        let group_list_data = env
            .data
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(group_list_id)?;

        Some(
            (0..(*group_list_data.params.group_count.end() as i32))
                .into_iter()
                .map(|num| GroupId {
                    group_list: *group_list_id,
                    num,
                })
                .collect(),
        )
    }
}

impl ViewBuilder<Env, GroupId> for ObjectId {
    type Object = Group;

    fn enumerate(env: &Env) -> BTreeSet<GroupId> {
        env.data
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .keys()
            .flat_map(|group_list_id| {
                GroupList::enumerate_groups(env, group_list_id)
                    .expect("Group list ID should be valid")
            })
            .collect()
    }

    fn build(env: &Env, id: &GroupId) -> Option<Self::Object> {
        let (prefilled_students, sealed) =
            if env.ignore_prefill_for_group_lists.contains(&id.group_list) {
                (vec![], false)
            } else {
                let group_list_data = env
                    .data
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .get(&id.group_list)?;
                match group_list_data.prefilled_groups.groups.get(id.num as usize) {
                    Some(prefilled_group) => (
                        prefilled_group.students.iter().copied().collect(),
                        prefilled_group.sealed,
                    ),
                    None => (vec![], false),
                }
            };

        Some(Group {
            group_list: id.group_list,
            num: id.num,
            next: GroupId {
                group_list: id.group_list,
                num: id.num + 1,
            },
            prefilled_students,
            sealed,
        })
    }
}

impl ViewBuilder<Env, SubjectId> for ObjectId {
    type Object = Subject;

    fn enumerate(env: &Env) -> BTreeSet<SubjectId> {
        env.data
            .get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }

    fn build(env: &Env, id: &SubjectId) -> Option<Self::Object> {
        let subject_data = env
            .data
            .get_inner_data()
            .params
            .subjects
            .find_subject(*id)?;

        Some(match &subject_data.parameters.interrogation_parameters {
            Some(params) => Subject {
                max_group_per_interrogation: params.groups_per_interrogation.end().get() as i32,
                min_group_per_interrogation: params.groups_per_interrogation.start().get() as i32,
            },
            None => Subject {
                max_group_per_interrogation: 0,
                min_group_per_interrogation: 0,
            },
        })
    }
}

impl ViewBuilder<Env, StudentId> for ObjectId {
    type Object = Student;

    fn enumerate(env: &Env) -> BTreeSet<StudentId> {
        env.data
            .get_inner_data()
            .params
            .students
            .student_map
            .keys()
            .copied()
            .collect()
    }

    fn build(env: &Env, id: &StudentId) -> Option<Self::Object> {
        let student_data = env
            .data
            .get_inner_data()
            .params
            .students
            .student_map
            .get(id)?;

        Some(Student {
            firstname: student_data.desc.firstname.clone(),
            surname: student_data.desc.surname.clone(),
        })
    }
}

impl ViewBuilder<Env, PeriodId> for ObjectId {
    type Object = Period;

    fn enumerate(env: &Env) -> BTreeSet<PeriodId> {
        env.data
            .get_inner_data()
            .params
            .periods
            .ordered_period_list
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }

    fn build(env: &Env, id: &PeriodId) -> Option<Self::Object> {
        let (pos, first_week) = env
            .data
            .get_inner_data()
            .params
            .periods
            .find_period_position_and_first_week(*id)?;
        let period_data = &env.data.get_inner_data().params.periods.ordered_period_list[pos].1;

        Some(Period {
            weeks: period_data
                .iter()
                .enumerate()
                .filter_map(|(num, desc)| {
                    if desc.interrogations {
                        Some(WeekId(first_week + num))
                    } else {
                        None
                    }
                })
                .collect(),
        })
    }
}
