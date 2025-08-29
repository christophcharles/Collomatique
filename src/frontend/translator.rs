use crate::backend::*;
use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU32, NonZeroUsize};
use thiserror::Error;

#[derive(Clone, Debug)]
struct GenColloCacheTimeSlot<TeacherId: OrdId> {
    teacher_id: TeacherId,
    start: SlotStart,
    room: String,
    week_map: BTreeMap<Week, usize>,
}

#[derive(Clone, Debug)]
struct GenColloCacheSubject<SubjectId: OrdId, TeacherId: OrdId> {
    id: SubjectId,
    group_list_name: String,
    groups: Vec<String>,
    slots: Vec<GenColloCacheTimeSlot<TeacherId>>,
}

#[derive(Clone, Debug)]
struct GenColloscopeCache<StudentId: OrdId, SubjectId: OrdId, TeacherId: OrdId> {
    student_ids: Vec<StudentId>,
    subjects: Vec<GenColloCacheSubject<SubjectId, TeacherId>>,
    validated_data: crate::gen::colloscope::ValidatedData,
}

use super::state::{
    update::Manager, GroupListHandle, GroupingHandle, GroupingIncompatHandle, IncompatHandle,
    StudentHandle, SubjectGroupHandle, SubjectHandle, TeacherHandle, TimeSlotHandle,
    WeekPatternHandle,
};

#[derive(Debug)]
pub struct GenColloscopeTranslator {
    data_cache: GenColloscopeCache<StudentHandle, SubjectHandle, TeacherHandle>,
}

impl GenColloscopeTranslator {
    pub async fn new<T: Manager>(
        manager: &mut T,
    ) -> GenColloscopeResult<GenColloscopeTranslator, T> {
        Ok(GenColloscopeTranslator {
            data_cache: GenColloscopeTranslator::build_data_cache(manager).await?,
        })
    }
}

#[derive(Debug, Error)]
pub enum GenColloscopeError<StorageError: std::fmt::Debug + std::error::Error> {
    #[error("Error in the storage backend: {0:?}")]
    StorageError(#[from] StorageError),
    #[error("Error while validating data: {0:?}")]
    ValidationError(crate::gen::colloscope::Error),
    #[error("Inconsistent data: bad subject id ({0:?})")]
    BadSubjectId(SubjectHandle),
    #[error("Inconsistent data: bad teacher id ({0:?})")]
    BadTeacherId(TeacherHandle),
    #[error("Inconsistent data: bad week pattern id ({0:?})")]
    BadWeekPatternId(WeekPatternHandle),
    #[error("Inconsistent data: bad group list id ({0:?})")]
    BadGroupListId(GroupListHandle),
    #[error("Inconsistent data: bad student id ({0:?})")]
    BadStudentId(StudentHandle),
    #[error("Inconsistent data: bad group index ({0})")]
    BadGroupIndex(usize),
    #[error(
        "Group size constraints are too strict (nb student = {0}, allowed group sizes in {1:?})"
    )]
    InconsistentGroupSizeConstraints(usize, std::ops::RangeInclusive<NonZeroUsize>),
    #[error("Inconsistent data: bad time slot id ({0:?})")]
    BadTimeSlotId(TimeSlotHandle),
    #[error("Inconsistent data: bad grouping id ({0:?})")]
    BadGroupingId(GroupingHandle),
}

#[derive(Debug, Error)]
pub enum TranslateColloscopeError {
    #[error("Colloscope does not fit validated data. It might not be built for this set of constraints or database")]
    BadColloscope,
}

impl<StorageError: std::fmt::Debug + std::error::Error> GenColloscopeError<StorageError> {
    fn from_validation(validation_error: crate::gen::colloscope::Error) -> Self {
        GenColloscopeError::ValidationError(validation_error)
    }
}

type GenColloscopeResult<R, T> = Result<
    R,
    GenColloscopeError<<<T as Manager>::InternalStorage as crate::backend::Storage>::InternalError>,
>;

struct GenColloscopeData {
    general_data: GeneralData,
    week_patterns: BTreeMap<WeekPatternHandle, WeekPattern>,
    teachers: BTreeMap<TeacherHandle, Teacher>,
    incompats: BTreeMap<IncompatHandle, Incompat<WeekPatternHandle>>,
    students: BTreeMap<StudentHandle, Student>,
    incompat_for_student_data: BTreeSet<(StudentHandle, IncompatHandle)>,
    subjects: BTreeMap<SubjectHandle, Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>>,
    subject_for_student_data: BTreeSet<(StudentHandle, SubjectHandle)>,
    time_slots: BTreeMap<TimeSlotHandle, TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>>,
    group_lists: BTreeMap<GroupListHandle, GroupList<StudentHandle>>,
    groupings: BTreeMap<GroupingHandle, Grouping<TimeSlotHandle>>,
    grouping_incompats: BTreeMap<GroupingIncompatHandle, GroupingIncompat<GroupingHandle>>,
}

impl GenColloscopeTranslator {
    async fn extract_data<T: Manager>(
        manager: &mut T,
    ) -> GenColloscopeResult<GenColloscopeData, T> {
        let incompats = manager.incompats_get_all().await?;
        let students = manager.students_get_all().await?;

        let mut incompat_for_student_data = BTreeSet::new();
        for (&student_id, _student) in &students {
            for (&incompat_id, _incompat) in &incompats {
                if manager
                    .incompat_for_student_get(student_id, incompat_id)
                    .await
                    .map_err(|e| match e {
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid", id1),
                        Id2Error::InvalidId2(id2) => {
                            panic!("Incompat id {:?} should be valid", id2)
                        }
                        Id2Error::InternalError(int_err) => int_err,
                    })?
                {
                    incompat_for_student_data.insert((student_id, incompat_id));
                }
            }
        }

        let subjects = manager.subjects_get_all().await?;
        let subject_groups = manager.subject_groups_get_all().await?;

        let mut subject_for_student_data = BTreeSet::new();
        for (&student_id, _student) in &students {
            for (&subject_group_id, _subject_group) in &subject_groups {
                let subject_opt = manager
                    .subject_group_for_student_get(student_id, subject_group_id)
                    .await
                    .map_err(|e| match e {
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid", id1),
                        Id2Error::InvalidId2(id2) => {
                            panic!("Subject group id {:?} should be valid", id2)
                        }
                        Id2Error::InternalError(int_err) => int_err,
                    })?;

                if let Some(subject_id) = subject_opt {
                    subject_for_student_data.insert((student_id, subject_id));
                }
            }
        }

        Ok(GenColloscopeData {
            general_data: manager.general_data_get().await?,
            week_patterns: manager.week_patterns_get_all().await?,
            teachers: manager.teachers_get_all().await?,
            incompats,
            students,
            incompat_for_student_data,
            subjects,
            subject_for_student_data,
            time_slots: manager.time_slots_get_all().await?,
            group_lists: manager.group_lists_get_all().await?,
            groupings: manager.groupings_get_all().await?,
            grouping_incompats: manager.grouping_incompats_get_all().await?,
        })
    }

    fn is_week_in_week_pattern(
        data: &GenColloscopeData,
        week_pattern_id: WeekPatternHandle,
        week: Week,
    ) -> bool {
        match data.week_patterns.get(&week_pattern_id) {
            None => false,
            Some(week_pattern) => week_pattern.weeks.contains(&week),
        }
    }

    fn build_general_data(data: &GenColloscopeData) -> crate::gen::colloscope::GeneralData {
        crate::gen::colloscope::GeneralData {
            teacher_count: data.teachers.len(),
            week_count: data.general_data.week_count,
            interrogations_per_week: data.general_data.interrogations_per_week.clone(),
            max_interrogations_per_day: data.general_data.max_interrogations_per_day,
            periodicity_cuts: data.general_data.periodicity_cuts.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct IncompatibilitiesData {
    incompat_list: crate::gen::colloscope::IncompatibilityList,
    incompat_group_list: crate::gen::colloscope::IncompatibilityGroupList,
    id_map: BTreeMap<IncompatHandle, BTreeSet<usize>>,
}

impl GenColloscopeTranslator {
    fn is_week_in_incompat_group(
        data: &GenColloscopeData,
        group: &IncompatGroup<WeekPatternHandle>,
        week: Week,
    ) -> bool {
        for slot in &group.slots {
            if Self::is_week_in_week_pattern(data, slot.week_pattern_id, week) {
                return true;
            }
        }
        false
    }

    fn build_incompatibility_data(
        data: &GenColloscopeData,
        week_count: NonZeroU32,
    ) -> IncompatibilitiesData {
        use crate::gen::colloscope::{Incompatibility, IncompatibilityGroup, SlotWithDuration};

        let mut output = IncompatibilitiesData {
            incompat_list: vec![],
            incompat_group_list: vec![],
            id_map: BTreeMap::new(),
        };

        for (&incompat_id, incompat) in &data.incompats {
            let mut ids = BTreeSet::new();

            for i in 0..week_count.get() {
                let week = Week::new(i);

                let mut new_incompat = Incompatibility {
                    max_count: incompat.max_count,
                    groups: BTreeSet::new(),
                };

                for group in &incompat.groups {
                    if !Self::is_week_in_incompat_group(data, group, week) {
                        continue;
                    }

                    let slots = group
                        .slots
                        .iter()
                        .map(|s| SlotWithDuration {
                            start: crate::gen::colloscope::SlotStart {
                                week: week.get(),
                                weekday: s.start.day,
                                start_time: s.start.time.clone(),
                            },
                            duration: s.duration,
                        })
                        .collect();
                    let new_group = IncompatibilityGroup { slots };

                    new_incompat.groups.insert(output.incompat_group_list.len());
                    output.incompat_group_list.push(new_group);
                }

                if !new_incompat.groups.is_empty() {
                    ids.insert(output.incompat_list.len());
                    output.incompat_list.push(new_incompat);
                }
            }

            output.id_map.insert(incompat_id, ids);
        }

        output
    }
}

#[derive(Clone, Debug)]
struct StudentData {
    student_list: crate::gen::colloscope::StudentList,
    id_map: BTreeMap<StudentHandle, usize>,
}

impl GenColloscopeTranslator {
    fn build_student_data(
        data: &GenColloscopeData,
        incompat_id_map: &BTreeMap<IncompatHandle, BTreeSet<usize>>,
    ) -> StudentData {
        use crate::gen::colloscope::Student;

        let mut output = StudentData {
            student_list: vec![],
            id_map: BTreeMap::new(),
        };

        for (&student_id, _student) in &data.students {
            let mut new_student = Student {
                incompatibilities: BTreeSet::new(),
            };

            for (&incompat_id, _incompat) in &data.incompats {
                if data
                    .incompat_for_student_data
                    .contains(&(student_id, incompat_id))
                {
                    new_student.incompatibilities.extend(
                        incompat_id_map
                            .get(&incompat_id)
                            .expect("Incompat id should be valid in map")
                            .iter()
                            .cloned(),
                    )
                }
            }

            for (&subject_id, subject) in &data.subjects {
                if let Some(incompat_id) = subject.incompat_id {
                    if data
                        .subject_for_student_data
                        .contains(&(student_id, subject_id))
                    {
                        // Because we are using BTreeSet, we don't care if the incompat was already added
                        new_student.incompatibilities.extend(
                            incompat_id_map
                                .get(&incompat_id)
                                .expect("Incompat id should be valid in map")
                                .iter()
                                .cloned(),
                        )
                    }
                }
            }

            output.id_map.insert(student_id, output.student_list.len());
            output.student_list.push(new_student);
        }

        output
    }
}

#[derive(Clone, Debug)]
struct BareSubjectData {
    subject_list: crate::gen::colloscope::SubjectList,
    id_map: BTreeMap<SubjectHandle, usize>,
}

#[derive(Clone, Debug)]
struct SubjectData {
    subject_list: crate::gen::colloscope::SubjectList,
    slot_id_map: BTreeMap<TimeSlotHandle, BTreeMap<Week, crate::gen::colloscope::SlotRef>>,
    subject_reverse_data: Vec<GenColloCacheSubject<SubjectHandle, TeacherHandle>>,
}

impl GenColloscopeTranslator {
    fn build_bare_subjects(data: &GenColloscopeData) -> BareSubjectData {
        use crate::gen::colloscope::{BalancingRequirements, GroupsDesc, Subject};

        let mut output = BareSubjectData {
            subject_list: vec![],
            id_map: BTreeMap::new(),
        };

        for (&subject_id, subject) in &data.subjects {
            let new_subject = Subject {
                students_per_group: subject.students_per_group.clone(),
                max_groups_per_slot: subject.max_groups_per_slot,
                period: subject.period,
                period_is_strict: subject.period_is_strict,
                is_tutorial: subject.is_tutorial,
                balancing_requirements: BalancingRequirements {
                    teachers: subject.balancing_requirements.teachers,
                    timeslots: subject.balancing_requirements.timeslots,
                },
                duration: subject.duration,
                slots: vec![],
                groups: GroupsDesc::default(),
            };

            output.id_map.insert(subject_id, output.subject_list.len());
            output.subject_list.push(new_subject);
        }

        output
    }

    fn add_slots_to_subjects_and_build_slot_id_map<T: Manager>(
        data: &GenColloscopeData,
        subjects: &mut crate::gen::colloscope::SubjectList,
        subject_reverse_data: &mut Vec<GenColloCacheSubject<SubjectHandle, TeacherHandle>>,
        subject_id_map: &BTreeMap<SubjectHandle, usize>,
    ) -> GenColloscopeResult<
        BTreeMap<TimeSlotHandle, BTreeMap<Week, crate::gen::colloscope::SlotRef>>,
        T,
    > {
        use crate::gen::colloscope::{SlotRef, SlotStart, SlotWithTeacher};

        let mut slot_id_map = BTreeMap::new();

        let teacher_id_map: BTreeMap<_, _> = data
            .teachers
            .iter()
            .enumerate()
            .map(|(i, (&teacher_id, _teacher))| (teacher_id, i))
            .collect();

        for (&time_slot_id, time_slot) in &data.time_slots {
            let subject_index = *subject_id_map
                .get(&time_slot.subject_id)
                .ok_or(GenColloscopeError::BadSubjectId(time_slot.subject_id))?;
            let subject = subjects.get_mut(subject_index).expect(&format!(
                "Subject index {} was built from id_map with id {:?} and should be valid",
                subject_index, time_slot.subject_id
            ));

            let teacher = *teacher_id_map
                .get(&time_slot.teacher_id)
                .ok_or(GenColloscopeError::BadTeacherId(time_slot.teacher_id))?;

            let week_pattern = data.week_patterns.get(&time_slot.week_pattern_id).ok_or(
                GenColloscopeError::BadWeekPatternId(time_slot.week_pattern_id),
            )?;

            let mut ids = BTreeMap::new();
            for &week in &week_pattern.weeks {
                let new_slot = SlotWithTeacher {
                    teacher,
                    start: SlotStart {
                        week: week.get(),
                        weekday: time_slot.start.day,
                        start_time: time_slot.start.time.clone(),
                    },
                    cost: time_slot.cost,
                };

                ids.insert(
                    week,
                    SlotRef {
                        subject: subject_index,
                        slot: subject.slots.len(),
                    },
                );
                subject.slots.push(new_slot);
            }
            let rev_time_slot = GenColloCacheTimeSlot {
                teacher_id: time_slot.teacher_id,
                start: time_slot.start.clone(),
                room: time_slot.room.clone(),
                week_map: ids
                    .iter()
                    .map(|(week, slot_ref)| (*week, slot_ref.slot))
                    .collect(),
            };

            let sub_rev = subject_reverse_data
                .get_mut(subject_index)
                .expect("Subject number should be valid");
            sub_rev.slots.push(rev_time_slot);

            slot_id_map.insert(time_slot_id, ids);
        }

        Ok(slot_id_map)
    }

    fn default_empty_groups_count(subject: &crate::gen::colloscope::Subject) -> Option<usize> {
        let min_group_size = subject.students_per_group.start().get();
        let max_group_size = subject.students_per_group.end().get();

        let student_count = subject.groups.not_assigned.len();

        let minimum_group_count = (student_count + (max_group_size - 1)) / max_group_size;
        let maximum_group_count = student_count / min_group_size;

        if minimum_group_count > maximum_group_count {
            None
        } else {
            Some(minimum_group_count)
        }
    }

    fn build_default_empty_groups<T: Manager>(
        subject: &mut crate::gen::colloscope::Subject,
    ) -> GenColloscopeResult<(), T> {
        use crate::gen::colloscope::GroupDesc;

        let Some(group_count) = Self::default_empty_groups_count(subject) else {
            let student_count = subject.groups.not_assigned.len();

            return Err(GenColloscopeError::InconsistentGroupSizeConstraints(
                student_count,
                subject.students_per_group.clone(),
            ));
        };

        subject.groups.prefilled_groups = vec![
            GroupDesc {
                students: BTreeSet::new(),
                can_be_extended: true,
            };
            group_count
        ];

        Ok(())
    }

    fn migrate_students_to_groups<T: Manager>(
        subject: &mut crate::gen::colloscope::Subject,
        rev_groups: &mut Vec<String>,
        group_list: &GroupList<StudentHandle>,
        student_id_map: &BTreeMap<StudentHandle, usize>,
    ) -> GenColloscopeResult<(), T> {
        use crate::gen::colloscope::GroupDesc;
        let mut prefilled_groups: Vec<_> = group_list
            .groups
            .iter()
            .map(|group| GroupDesc {
                students: BTreeSet::new(),
                can_be_extended: group.extendable,
            })
            .collect();

        for (&student_id, &group_index) in &group_list.students_mapping {
            let student_index = *student_id_map
                .get(&student_id)
                .ok_or(GenColloscopeError::BadStudentId(student_id))?;

            if subject.groups.not_assigned.contains(&student_index) {
                subject.groups.not_assigned.remove(&student_index);
                let group = prefilled_groups
                    .get_mut(group_index)
                    .ok_or(GenColloscopeError::BadGroupIndex(group_index))?;
                group.students.insert(student_index);
            }
        }

        // Remove groups that are empty and not extendable
        subject.groups.prefilled_groups = prefilled_groups
            .into_iter()
            .enumerate()
            .filter_map(|(i, group)| {
                if !(group.students.is_empty() && !group.can_be_extended) {
                    rev_groups.push(group_list.groups[i].name.clone());
                    Some(group)
                } else {
                    None
                }
            })
            .collect();

        Ok(())
    }

    fn add_groups_to_subjects<T: Manager>(
        data: &GenColloscopeData,
        subjects: &mut crate::gen::colloscope::SubjectList,
        subject_reverse_data: &mut Vec<GenColloCacheSubject<SubjectHandle, TeacherHandle>>,
        subject_id_map: &BTreeMap<SubjectHandle, usize>,
        student_id_map: &BTreeMap<StudentHandle, usize>,
    ) -> GenColloscopeResult<(), T> {
        for (&subject_id, &subject_index) in subject_id_map {
            let subject = subjects
                .get_mut(subject_index)
                .expect(&format!("Subject index {} should be valid", subject_index));

            // Put all students that are registered as not_assigned at first
            for (&student_id, _student) in &data.students {
                let student_index = *student_id_map
                    .get(&student_id)
                    .expect(&format!("Student id {:?} should be valid", student_id));
                if data
                    .subject_for_student_data
                    .contains(&(student_id, subject_id))
                {
                    subject.groups.not_assigned.insert(student_index);
                }
            }

            // If the subject has a group_list, we use it.
            // If not, we build a default group list with empty extendable groups
            let og_subject = data
                .subjects
                .get(&subject_id)
                .expect("Subject id should be valid");
            let rev_subject = subject_reverse_data
                .get_mut(subject_index)
                .expect("Subject index should be valid");
            match og_subject.group_list_id {
                Some(group_list_id) => {
                    let group_list = data
                        .group_lists
                        .get(&group_list_id)
                        .ok_or(GenColloscopeError::BadGroupListId(group_list_id))?;

                    rev_subject.group_list_name = group_list.name.clone();

                    Self::migrate_students_to_groups::<T>(
                        subject,
                        &mut rev_subject.groups,
                        group_list,
                        student_id_map,
                    )?;
                }
                None => {
                    Self::build_default_empty_groups::<T>(subject)?;

                    rev_subject.group_list_name = data
                        .subjects
                        .get(&subject_id)
                        .expect("Subject id should be valid")
                        .name
                        .clone();
                    rev_subject.groups = subject
                        .groups
                        .prefilled_groups
                        .iter()
                        .enumerate()
                        .map(|(i, _)| (i + 1).to_string())
                        .collect();
                }
            }
        }

        Ok(())
    }

    fn build_subject_data<T: Manager>(
        data: &GenColloscopeData,
        student_id_map: &BTreeMap<StudentHandle, usize>,
    ) -> GenColloscopeResult<SubjectData, T> {
        let mut bare_subject_data = Self::build_bare_subjects(data);

        let subject_id_reverse_map: BTreeMap<_, _> = bare_subject_data
            .id_map
            .iter()
            .map(|(id, num)| (*num, *id))
            .collect();

        let mut subject_reverse_data = bare_subject_data
            .subject_list
            .iter()
            .enumerate()
            .map(|(subject_num, _bare_subject)| {
                let subject_id = subject_id_reverse_map
                    .get(&subject_num)
                    .expect("There should be an id for every subject");
                let orig_subject = data
                    .subjects
                    .get(subject_id)
                    .expect("Subject id should be valid");

                let group_list_name = match orig_subject.group_list_id {
                    Some(group_list_id) => {
                        let group_list = data
                            .group_lists
                            .get(&group_list_id)
                            .ok_or(GenColloscopeError::BadGroupListId(group_list_id))?;
                        group_list.name.clone()
                    }
                    None => orig_subject.name.clone(),
                };

                Ok(GenColloCacheSubject {
                    id: *subject_id,
                    group_list_name,
                    groups: vec![],
                    slots: vec![],
                })
            })
            .collect::<GenColloscopeResult<_, T>>()?;

        let slot_id_map = Self::add_slots_to_subjects_and_build_slot_id_map::<T>(
            data,
            &mut bare_subject_data.subject_list,
            &mut subject_reverse_data,
            &bare_subject_data.id_map,
        )?;

        Self::add_groups_to_subjects::<T>(
            data,
            &mut bare_subject_data.subject_list,
            &mut subject_reverse_data,
            &bare_subject_data.id_map,
            student_id_map,
        )?;

        Ok(SubjectData {
            subject_list: bare_subject_data.subject_list,
            slot_id_map,
            subject_reverse_data,
        })
    }
}

#[derive(Clone, Debug)]
struct SlotGroupingData {
    slot_grouping_list: crate::gen::colloscope::SlotGroupingList,
    id_map: BTreeMap<GroupingHandle, BTreeMap<Week, usize>>,
}

impl GenColloscopeTranslator {
    fn build_slot_grouping_data<T: Manager>(
        data: &GenColloscopeData,
        week_count: NonZeroU32,
        slot_id_map: &BTreeMap<TimeSlotHandle, BTreeMap<Week, crate::gen::colloscope::SlotRef>>,
    ) -> GenColloscopeResult<SlotGroupingData, T> {
        use crate::gen::colloscope::SlotGrouping;

        let mut output = SlotGroupingData {
            slot_grouping_list: vec![],
            id_map: BTreeMap::new(),
        };

        for (&grouping_id, grouping) in &data.groupings {
            let mut ids = BTreeMap::new();

            for i in 0..week_count.get() {
                let week = Week::new(i);

                let mut slots = BTreeSet::new();

                for &time_slot_id in &grouping.slots {
                    let week_map = slot_id_map
                        .get(&time_slot_id)
                        .ok_or(GenColloscopeError::BadTimeSlotId(time_slot_id))?;

                    let slot_ref_opt = week_map.get(&week);
                    if let Some(slot_ref) = slot_ref_opt {
                        slots.insert(slot_ref.clone());
                    }
                }

                if !slots.is_empty() {
                    ids.insert(week, output.slot_grouping_list.len());
                    output.slot_grouping_list.push(SlotGrouping { slots });
                };
            }

            output.id_map.insert(grouping_id, ids);
        }

        Ok(output)
    }
}

impl GenColloscopeTranslator {
    fn build_grouping_incompats<T: Manager>(
        data: &GenColloscopeData,
        week_count: NonZeroU32,
        grouping_id_map: &BTreeMap<GroupingHandle, BTreeMap<Week, usize>>,
    ) -> GenColloscopeResult<crate::gen::colloscope::SlotGroupingIncompatSet, T> {
        use crate::gen::colloscope::SlotGroupingIncompat;

        let mut output = BTreeSet::new();

        for (&_grouping_incompat_id, grouping_incompat) in &data.grouping_incompats {
            for i in 0..week_count.get() {
                let week = Week::new(i);

                let mut groupings = BTreeSet::new();

                for &grouping_id in &grouping_incompat.groupings {
                    let week_map = grouping_id_map
                        .get(&grouping_id)
                        .ok_or(GenColloscopeError::BadGroupingId(grouping_id))?;

                    let grouping_index_opt = week_map.get(&week);
                    if let Some(&grouping_index) = grouping_index_opt {
                        groupings.insert(grouping_index);
                    }
                }

                let max_count = grouping_incompat.max_count;
                if groupings.len() > max_count.get() {
                    output.insert(SlotGroupingIncompat {
                        groupings,
                        max_count,
                    });
                };
            }
        }

        Ok(output)
    }
}

impl GenColloscopeTranslator {
    async fn build_data_cache<T: Manager>(
        manager: &mut T,
    ) -> GenColloscopeResult<GenColloscopeCache<StudentHandle, SubjectHandle, TeacherHandle>, T>
    {
        let data = Self::extract_data(manager).await?;

        let general = Self::build_general_data(&data);
        let incompatibility_data = Self::build_incompatibility_data(&data, general.week_count);
        let student_data = Self::build_student_data(&data, &incompatibility_data.id_map);
        let subject_data = Self::build_subject_data::<T>(&data, &student_data.id_map)?;
        let slot_grouping_data = Self::build_slot_grouping_data::<T>(
            &data,
            general.week_count,
            &subject_data.slot_id_map,
        )?;
        let grouping_incompats = Self::build_grouping_incompats::<T>(
            &data,
            general.week_count,
            &slot_grouping_data.id_map,
        )?;

        let validated_data = crate::gen::colloscope::ValidatedData::new(
            general,
            subject_data.subject_list,
            incompatibility_data.incompat_group_list,
            incompatibility_data.incompat_list,
            student_data.student_list,
            slot_grouping_data.slot_grouping_list,
            grouping_incompats,
        )
        .map_err(GenColloscopeError::from_validation)?;

        let reverse_students_map: BTreeMap<_, _> = student_data
            .id_map
            .into_iter()
            .map(|(id, num)| (num, id))
            .collect();
        let student_ids = reverse_students_map.iter().enumerate().map(
            |(i,(num,id))| {
                if *num != i {
                    panic!("Missing student in reverse_students_map. This should not happen and student numbers should be consecutive");
                }

                *id
            }
        ).collect();

        let subjects = subject_data.subject_reverse_data;

        Ok(GenColloscopeCache {
            student_ids,
            subjects,
            validated_data,
        })
    }
}

impl GenColloscopeTranslator {
    pub fn get_validated_data(&self) -> crate::gen::colloscope::ValidatedData {
        self.data_cache.validated_data.clone()
    }

    pub fn translate_colloscope(
        &self,
        colloscope: &crate::gen::colloscope::Colloscope,
        name: &str,
    ) -> Result<Colloscope<TeacherHandle, SubjectHandle, StudentHandle>, TranslateColloscopeError>
    {
        let subjects = colloscope
            .subjects
            .iter()
            .enumerate()
            .map(|(i, subject)| {
                let subject_data = self
                    .data_cache
                    .subjects
                    .get(i)
                    .ok_or(TranslateColloscopeError::BadColloscope)?;

                let group_list = ColloscopeGroupList {
                    name: subject_data.group_list_name.clone(),
                    groups: subject_data.groups.clone(),
                    students_mapping: subject
                        .groups
                        .iter()
                        .enumerate()
                        .flat_map(|(j, group)| {
                            group
                                .iter()
                                .copied()
                                .map(|student| {
                                    let student_id = *self
                                        .data_cache
                                        .student_ids
                                        .get(student)
                                        .ok_or(TranslateColloscopeError::BadColloscope)?;

                                    Ok((student_id, j))
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Result<_, TranslateColloscopeError>>()?,
                };

                let time_slots = subject_data
                    .slots
                    .iter()
                    .map(|time_slot_data| {
                        Ok(ColloscopeTimeSlot {
                            teacher_id: time_slot_data.teacher_id,
                            start: time_slot_data.start.clone(),
                            room: time_slot_data.room.clone(),
                            group_assignments: time_slot_data
                                .week_map
                                .iter()
                                .map(|(&week, &slot_num)| {
                                    let groups: &BTreeSet<usize> = subject
                                        .slots
                                        .get(slot_num)
                                        .ok_or(TranslateColloscopeError::BadColloscope)?;

                                    Ok((week, groups.clone()))
                                })
                                .collect::<Result<_, TranslateColloscopeError>>()?,
                        })
                    })
                    .collect::<Result<_, TranslateColloscopeError>>()?;

                Ok((
                    subject_data.id,
                    ColloscopeSubject {
                        time_slots,
                        group_list,
                    },
                ))
            })
            .collect::<Result<_, TranslateColloscopeError>>()?;

        let output = Colloscope {
            name: String::from(name),
            subjects,
        };

        Ok(output)
    }
}
