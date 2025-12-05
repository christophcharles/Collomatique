use crate::tools;

use super::objects::{InterrogationData, WeekId};
use super::tools::*;
use collo_ml::{EvalObject, ViewBuilder, ViewObject};
use collomatique_state_colloscopes::{Data, GroupListId, PeriodId, StudentId, SubjectId};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(Data)]
#[cached]
pub enum ObjectId {
    Interrogation(InterrogationData),
    Week(WeekId),
    GroupList(GroupListId),
    Subject(SubjectId),
    Student(StudentId),
    Period(PeriodId),
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
    groups: Vec<i32>,
    students: Vec<StudentId>,
    min_student_per_group: i32,
    max_student_per_group: i32,
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

impl ViewBuilder<Data, InterrogationData> for ObjectId {
    type Object = Interrogation;

    fn enumerate(env: &Data) -> BTreeSet<InterrogationData> {
        let mut output = BTreeSet::new();
        for (_subject_id, subject_slots) in &env.get_inner_data().params.slots.subject_map {
            for (slot_id, slot_desc) in &subject_slots.ordered_slots {
                let week_pattern = tools::extract_week_pattern(env, slot_desc.week_pattern);
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

    fn build(env: &Data, id: &InterrogationData) -> Option<Self::Object> {
        let (subject_id, _pos) = env
            .get_inner_data()
            .params
            .slots
            .find_slot_subject_and_position(id.slot)?;
        let (period_id, _) = week_to_period_id(&env.get_inner_data().params, id.week)?;
        let period_associations = env
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

impl ViewBuilder<Data, WeekId> for ObjectId {
    type Object = Week;

    fn enumerate(env: &Data) -> BTreeSet<WeekId> {
        let mut output = BTreeSet::new();

        let mut current_first_week = 0usize;
        for (_period_id, period_desc) in &env.get_inner_data().params.periods.ordered_period_list {
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

    fn build(env: &Data, id: &WeekId) -> Option<Self::Object> {
        let (period, _) = tools::week_to_period_id(&env.get_inner_data().params, id.0)?;

        Some(Week {
            num: id.0 as i32,
            period,
        })
    }
}

impl ViewBuilder<Data, GroupListId> for ObjectId {
    type Object = GroupList;

    fn enumerate(env: &Data) -> BTreeSet<GroupListId> {
        env.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .keys()
            .map(|x| *x)
            .collect()
    }

    fn build(env: &Data, id: &GroupListId) -> Option<Self::Object> {
        let group_list_data = env
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(id)?;

        Some(GroupList {
            groups: (0..(*group_list_data.params.group_count.end() as i32))
                .into_iter()
                .collect(),
            students: env
                .get_inner_data()
                .params
                .students
                .student_map
                .keys()
                .copied()
                .filter(|x| !group_list_data.params.excluded_students.contains(x))
                .collect(),
            min_student_per_group: group_list_data.params.students_per_group.start().get() as i32,
            max_student_per_group: group_list_data.params.students_per_group.end().get() as i32,
        })
    }
}

impl ViewBuilder<Data, SubjectId> for ObjectId {
    type Object = Subject;

    fn enumerate(env: &Data) -> BTreeSet<SubjectId> {
        env.get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }

    fn build(env: &Data, id: &SubjectId) -> Option<Self::Object> {
        let subject_data = env.get_inner_data().params.subjects.find_subject(*id)?;

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

impl ViewBuilder<Data, StudentId> for ObjectId {
    type Object = Student;

    fn enumerate(env: &Data) -> BTreeSet<StudentId> {
        env.get_inner_data()
            .params
            .students
            .student_map
            .keys()
            .copied()
            .collect()
    }

    fn build(env: &Data, id: &StudentId) -> Option<Self::Object> {
        let student_data = env.get_inner_data().params.students.student_map.get(id)?;

        Some(Student {
            firstname: student_data.desc.firstname.clone(),
            surname: student_data.desc.surname.clone(),
        })
    }
}

impl ViewBuilder<Data, PeriodId> for ObjectId {
    type Object = Period;

    fn enumerate(env: &Data) -> BTreeSet<PeriodId> {
        env.get_inner_data()
            .params
            .periods
            .ordered_period_list
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }

    fn build(env: &Data, id: &PeriodId) -> Option<Self::Object> {
        let (pos, first_week) = env
            .get_inner_data()
            .params
            .periods
            .find_period_position_and_first_week(*id)?;
        let period_data = &env.get_inner_data().params.periods.ordered_period_list[pos].1;

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
