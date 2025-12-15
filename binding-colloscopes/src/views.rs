use crate::tools;

use super::objects::{
    DayData, GroupId, InterrogationData, SubjectPeriodData, TimeSlotData, WeekId, WeekdayData,
};
use super::tools::*;
use collo_ml::{EvalObject, ViewBuilder, ViewObject};
use collomatique_state_colloscopes::{Data, GroupListId, PeriodId, StudentId, SubjectId};
use collomatique_time::{NonZeroMinutes, WholeMinuteTime};
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
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
    Weekday(WeekdayData),
    Day(DayData),
    TimeSlot(TimeSlotData),
    SubjectPeriod(SubjectPeriodData),
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Interrogation {
    subject: SubjectId,
    week: WeekId,
    group_list: GroupListId,
    students: Vec<StudentId>,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Week {
    num: i32,
    period: PeriodId,
    days: Vec<DayData>,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct GroupList {
    groups: Vec<GroupId>,
    complete_students: Vec<StudentId>,
    students: Vec<StudentId>,
    min_students_per_group: i32,
    max_students_per_group: i32,
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
    take_into_account: bool,
    has_interrogations: bool,
    duration: i32,
    periods_data: Vec<SubjectPeriodData>,
    min_students_per_group: i32,
    max_students_per_group: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct SubjectPeriod {
    subject: SubjectId,
    period: PeriodId,
    group_list: Option<GroupListId>,
    students: Vec<StudentId>,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
#[pretty("{firstname} {surname}")]
pub struct Student {
    hard_max_interrogations_per_day: bool,
    max_interrogations_per_day: i32,
    hard_max_interrogations_per_week: bool,
    max_interrogations_per_week: i32,
    hard_min_interrogations_per_week: bool,
    min_interrogations_per_week: i32,
    periods: Vec<PeriodId>,
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

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct TimeSlot {
    day: DayData,
    hour: i32,
    minute: i32,
    duration: i32,
    interrogations: Vec<InterrogationData>,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Weekday {
    num: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, ViewObject)]
#[eval_object(ObjectId)]
pub struct Day {
    weekday: WeekdayData,
    week: WeekId,
    time_slots: Vec<TimeSlotData>,
}

impl ViewBuilder<Env, InterrogationData> for ObjectId {
    type Object = Interrogation;

    fn enumerate(env: &Env) -> BTreeSet<InterrogationData> {
        let mut output = BTreeSet::new();
        for (subject_id, subject_slots) in &env.data.get_inner_data().params.slots.subject_map {
            let subject_desc = env
                .data
                .get_inner_data()
                .params
                .subjects
                .find_subject(*subject_id)
                .expect("Subject ID should be valid");
            for (slot_id, slot_desc) in &subject_slots.ordered_slots {
                let week_pattern = tools::extract_week_pattern(&env.data, slot_desc.week_pattern);
                for (week, status) in week_pattern.into_iter().enumerate() {
                    if !status {
                        continue;
                    }
                    let (period, _) =
                        tools::week_to_period_id(&env.data.get_inner_data().params, week)
                            .expect("Week should correspond to some period");
                    if subject_desc.excluded_periods.contains(&period) {
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

        let students = match env
            .data
            .get_inner_data()
            .params
            .assignments
            .period_map
            .get(&period_id)
            .expect("Period ID should be valid")
            .subject_map
            .get(&subject_id)
        {
            Some(students) => students.iter().cloned().collect(),
            None => vec![],
        };

        Some(Interrogation {
            subject: subject_id,
            week: WeekId(id.week),
            group_list: *group_list_id,
            students,
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
            days: collomatique_time::Weekday::iter()
                .map(|day| DayData { day, week: *id })
                .collect(),
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
            min_students_per_group: group_list_data.params.students_per_group.start().get() as i32,
            max_students_per_group: group_list_data.params.students_per_group.end().get() as i32,
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

        let periods_data = env
            .data
            .get_inner_data()
            .params
            .periods
            .ordered_period_list
            .iter()
            .filter_map(|(period_id, _period_desc)| {
                if subject_data.excluded_periods.contains(period_id) {
                    return None;
                }
                Some(SubjectPeriodData {
                    subject: *id,
                    period: *period_id,
                })
            })
            .collect();

        Some(match &subject_data.parameters.interrogation_parameters {
            Some(params) => Subject {
                max_group_per_interrogation: params.groups_per_interrogation.end().get() as i32,
                min_group_per_interrogation: params.groups_per_interrogation.start().get() as i32,
                take_into_account: params.take_duration_into_account,
                duration: params.duration.get().get() as i32,
                has_interrogations: true,
                periods_data,
                min_students_per_group: params.students_per_group.start().get() as i32,
                max_students_per_group: params.students_per_group.end().get() as i32,
            },
            None => Subject {
                max_group_per_interrogation: 0,
                min_group_per_interrogation: 0,
                min_students_per_group: 0,
                max_students_per_group: 0,
                take_into_account: false,
                duration: 60,
                has_interrogations: false,
                periods_data,
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

        let limits = env
            .data
            .get_inner_data()
            .params
            .settings
            .students
            .get(id)
            .unwrap_or(&env.data.get_inner_data().params.settings.global);

        let (hard_max_interrogations_per_day, max_interrogations_per_day) =
            match &limits.max_interrogations_per_day {
                Some(val) => (!val.soft, val.value.get() as i32),
                None => (false, -1),
            };
        let (hard_max_interrogations_per_week, max_interrogations_per_week) =
            match &limits.interrogations_per_week_max {
                Some(val) => (!val.soft, val.value as i32),
                None => (false, -1),
            };
        let (hard_min_interrogations_per_week, min_interrogations_per_week) =
            match &limits.interrogations_per_week_min {
                Some(val) => (!val.soft, val.value as i32),
                None => (false, -1),
            };

        Some(Student {
            firstname: student_data.desc.firstname.clone(),
            surname: student_data.desc.surname.clone(),
            hard_max_interrogations_per_day,
            max_interrogations_per_day,
            hard_max_interrogations_per_week,
            max_interrogations_per_week,
            hard_min_interrogations_per_week,
            min_interrogations_per_week,
            periods: env
                .data
                .get_inner_data()
                .params
                .periods
                .ordered_period_list
                .iter()
                .filter_map(|(period_id, _desc)| {
                    if student_data.excluded_periods.contains(period_id) {
                        return None;
                    }
                    Some(*period_id)
                })
                .collect(),
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

impl ViewBuilder<Env, WeekdayData> for ObjectId {
    type Object = Weekday;

    fn enumerate(_env: &Env) -> BTreeSet<WeekdayData> {
        let mut output = BTreeSet::new();
        for day in collomatique_time::Weekday::iter() {
            output.insert(WeekdayData { day });
        }
        output
    }

    fn build(_env: &Env, id: &WeekdayData) -> Option<Self::Object> {
        Some(Weekday {
            num: id.day.num_days_from_monday() as i32,
        })
    }
}

impl ViewBuilder<Env, DayData> for ObjectId {
    type Object = Day;

    fn enumerate(env: &Env) -> BTreeSet<DayData> {
        let mut output = BTreeSet::new();

        let mut current_first_week = 0usize;
        for (_period_id, period_desc) in
            &env.data.get_inner_data().params.periods.ordered_period_list
        {
            for (num, week_desc) in period_desc.iter().enumerate() {
                if !week_desc.interrogations {
                    continue;
                }
                for day in collomatique_time::Weekday::iter() {
                    output.insert(DayData {
                        week: WeekId(current_first_week + num),
                        day,
                    });
                }
            }
            current_first_week += period_desc.len();
        }

        output
    }

    fn build(env: &Env, id: &DayData) -> Option<Self::Object> {
        Some(Day {
            weekday: WeekdayData { day: id.day },
            week: id.week,
            time_slots: TimeSlot::generate_time_slots_for_a_single_day(env, id.day)
                .into_iter()
                .map(|slot| TimeSlotData {
                    slot,
                    week: id.week,
                })
                .collect(),
        })
    }
}

impl TimeSlot {
    fn generate_time_slots_for_a_single_day(
        env: &Env,
        day: collomatique_time::Weekday,
    ) -> Vec<collomatique_time::SlotWithDuration> {
        let duration = tools::compute_time_resolution(&env.data.get_inner_data().params);

        const MINUTES_PER_HOUR: u32 = 60;
        const HOUR_PER_DAY: u32 = 24;
        const MINUTES_PER_DAY: u32 = MINUTES_PER_HOUR * HOUR_PER_DAY;

        (0..MINUTES_PER_DAY)
            .step_by(duration as usize)
            .map(|start_minute| {
                let hour = start_minute / MINUTES_PER_HOUR;
                let min = start_minute % MINUTES_PER_HOUR;
                let start_time = WholeMinuteTime::new(
                    chrono::NaiveTime::from_hms_opt(hour, min, 0).expect("Time should be valid"),
                )
                .expect("Time should be with a whole minute");
                collomatique_time::SlotWithDuration::new(
                    collomatique_time::SlotStart {
                        weekday: day.clone().into(),
                        start_time,
                    },
                    NonZeroMinutes::new(duration).expect("duration should be non-zero"),
                )
                .expect("Slot should be valid and not cross the midnight boundary")
            })
            .collect()
    }
}

impl ViewBuilder<Env, TimeSlotData> for ObjectId {
    type Object = TimeSlot;

    fn enumerate(env: &Env) -> BTreeSet<TimeSlotData> {
        let mut output = BTreeSet::new();
        for day in collomatique_time::Weekday::iter() {
            for (week, status) in tools::extract_week_pattern(&env.data, None)
                .into_iter()
                .enumerate()
            {
                if !status {
                    continue;
                }
                output.extend(
                    TimeSlot::generate_time_slots_for_a_single_day(env, day)
                        .into_iter()
                        .map(|slot| TimeSlotData {
                            slot,
                            week: WeekId(week),
                        }),
                );
            }
        }
        output
    }

    fn build(env: &Env, id: &TimeSlotData) -> Option<Self::Object> {
        use chrono::Timelike;
        let mut interrogations = vec![];

        for (subject_id, subject_slots) in &env.data.get_inner_data().params.slots.subject_map {
            let subject_desc = env
                .data
                .get_inner_data()
                .params
                .subjects
                .find_subject(*subject_id)
                .expect("Subject ID should be valid");
            let (period, _) =
                tools::week_to_period_id(&env.data.get_inner_data().params, id.week.0)
                    .expect("Week should correspond to some period");
            if subject_desc.excluded_periods.contains(&period) {
                continue;
            }
            let subject_params = subject_desc
                .parameters
                .interrogation_parameters
                .as_ref()
                .expect("Subject with slots should have parameters");
            let duration = subject_params.duration.clone();
            for (slot_id, slot_desc) in &subject_slots.ordered_slots {
                let week_pattern = tools::extract_week_pattern(&env.data, slot_desc.week_pattern);
                let status = week_pattern
                    .get(id.week.0)
                    .expect("Week number should be valid");
                if !status {
                    continue;
                }
                let slot_with_duration = collomatique_time::SlotWithDuration::new(
                    slot_desc.start_time.clone(),
                    duration,
                )
                .expect("Slot should not cross the midnight boundary");
                if !slot_with_duration.overlaps_with(&id.slot) {
                    continue;
                }
                interrogations.push(InterrogationData {
                    slot: *slot_id,
                    week: id.week.0,
                })
            }
        }

        Some(TimeSlot {
            day: DayData {
                day: id.slot.start().weekday,
                week: id.week,
            },
            hour: id.slot.start().start_time.hour() as i32,
            minute: id.slot.start().start_time.minute() as i32,
            duration: id.slot.duration().get().get() as i32,
            interrogations,
        })
    }
}

impl ViewBuilder<Env, SubjectPeriodData> for ObjectId {
    type Object = SubjectPeriod;

    fn enumerate(env: &Env) -> BTreeSet<SubjectPeriodData> {
        let mut output = BTreeSet::new();

        for (subject_id, subject_data) in &env
            .data
            .get_inner_data()
            .params
            .subjects
            .ordered_subject_list
        {
            for (period_id, _period_desc) in
                &env.data.get_inner_data().params.periods.ordered_period_list
            {
                if subject_data.excluded_periods.contains(period_id) {
                    continue;
                }
                output.insert(SubjectPeriodData {
                    subject: *subject_id,
                    period: *period_id,
                });
            }
        }

        output
    }

    fn build(env: &Env, id: &SubjectPeriodData) -> Option<Self::Object> {
        let group_list_id = env
            .data
            .get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .get(&id.period)
            .expect("Period ID should be valid")
            .get(&id.subject);
        let subject_assignments = env
            .data
            .get_inner_data()
            .params
            .assignments
            .period_map
            .get(&id.period)
            .expect("Period ID should be valid")
            .subject_map
            .get(&id.subject);
        let students = match subject_assignments {
            Some(assignments) => assignments.iter().cloned().collect(),
            None => vec![],
        };
        Some(SubjectPeriod {
            subject: id.subject,
            period: id.period,
            group_list: group_list_id.cloned(),
            students,
        })
    }
}
