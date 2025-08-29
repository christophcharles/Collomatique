use std::ops::Deref;

use super::*;

fn get_next_id<T: OrdId, U: OrdId + From<u64>>(
    next_id: &mut u64,
    id_map: &mut BTreeMap<T, U>,
    id: T,
) -> U {
    match id_map.get(&id) {
        Some(new_id) => new_id.clone(),
        None => {
            let new_id = U::from(*next_id);
            *next_id += 1;
            id_map.insert(id, new_id.clone());
            new_id
        }
    }
}

fn translate_incompat<T: OrdId>(
    incompat: Incompat<T>,
    week_pattern_ids: &BTreeMap<T, WeekPatternId>,
) -> Option<Incompat<WeekPatternId>> {
    Some(Incompat {
        name: incompat.name,
        max_count: incompat.max_count,
        groups: incompat
            .groups
            .into_iter()
            .map(|x| {
                Some(IncompatGroup::<WeekPatternId> {
                    slots: x
                        .slots
                        .into_iter()
                        .map(|y| {
                            Some(IncompatSlot::<WeekPatternId> {
                                start: y.start,
                                duration: y.duration,
                                week_pattern_id: week_pattern_ids
                                    .get(&y.week_pattern_id)
                                    .cloned()?,
                            })
                        })
                        .collect::<Option<_>>()?,
                })
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_group_list<T: OrdId>(
    group_list: GroupList<T>,
    student_ids: &BTreeMap<T, StudentId>,
) -> Option<GroupList<StudentId>> {
    Some(GroupList {
        name: group_list.name,
        groups: group_list.groups,
        students_mapping: group_list
            .students_mapping
            .into_iter()
            .map(|(student_id, group)| {
                let new_student_id = student_ids.get(&student_id).cloned()?;

                Some((new_student_id, group))
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_subject<T: OrdId, U: OrdId, V: OrdId>(
    subject: Subject<T, U, V>,
    subject_group_ids: &BTreeMap<T, SubjectGroupId>,
    incompat_ids: &BTreeMap<U, IncompatId>,
    group_list_ids: &BTreeMap<V, GroupListId>,
) -> Option<Subject<SubjectGroupId, IncompatId, GroupListId>> {
    Some(Subject {
        name: subject.name,
        subject_group_id: subject_group_ids.get(&subject.subject_group_id).cloned()?,
        incompat_id: match subject.incompat_id {
            Some(old_id) => Some(incompat_ids.get(&old_id).cloned()?),
            None => None,
        },
        group_list_id: match subject.group_list_id {
            Some(old_id) => Some(group_list_ids.get(&old_id).cloned()?),
            None => None,
        },
        duration: subject.duration,
        students_per_group: subject.students_per_group,
        period: subject.period,
        period_is_strict: subject.period_is_strict,
        is_tutorial: subject.is_tutorial,
        max_groups_per_slot: subject.max_groups_per_slot,
        balancing_requirements: subject.balancing_requirements,
    })
}

fn translate_time_slot<T: OrdId, U: OrdId, V: OrdId>(
    time_slot: TimeSlot<T, U, V>,
    subject_ids: &BTreeMap<T, SubjectId>,
    teacher_ids: &BTreeMap<U, TeacherId>,
    week_pattern_ids: &BTreeMap<V, WeekPatternId>,
) -> Option<TimeSlot<SubjectId, TeacherId, WeekPatternId>> {
    Some(TimeSlot {
        subject_id: subject_ids.get(&time_slot.subject_id).cloned()?,
        teacher_id: teacher_ids.get(&time_slot.teacher_id).cloned()?,
        start: time_slot.start,
        week_pattern_id: week_pattern_ids.get(&time_slot.week_pattern_id).cloned()?,
        room: time_slot.room,
        cost: time_slot.cost,
    })
}

fn translate_grouping<T: OrdId>(
    grouping: Grouping<T>,
    time_slot_ids: &BTreeMap<T, TimeSlotId>,
) -> Option<Grouping<TimeSlotId>> {
    Some(Grouping {
        name: grouping.name,
        slots: grouping
            .slots
            .into_iter()
            .map(|time_slot_id| {
                let new_time_slot_id = time_slot_ids.get(&time_slot_id).cloned()?;

                Some(new_time_slot_id)
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_grouping_incompat<T: OrdId>(
    grouping_incompat: GroupingIncompat<T>,
    grouping_ids: &BTreeMap<T, GroupingId>,
) -> Option<GroupingIncompat<GroupingId>> {
    Some(GroupingIncompat {
        max_count: grouping_incompat.max_count,
        groupings: grouping_incompat
            .groupings
            .into_iter()
            .map(|grouping_id| {
                let new_grouping_id = grouping_ids.get(&grouping_id).cloned()?;

                Some(new_grouping_id)
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_colloscope_group_list<V: OrdId>(
    colloscope_group_list: ColloscopeGroupList<V>,
    student_ids: &BTreeMap<V, StudentId>,
) -> Option<ColloscopeGroupList<StudentId>> {
    Some(ColloscopeGroupList {
        name: colloscope_group_list.name,
        groups: colloscope_group_list.groups,
        students_mapping: colloscope_group_list
            .students_mapping
            .into_iter()
            .map(|(student_id, x)| {
                let new_student_id = student_ids.get(&student_id).cloned()?;

                Some((new_student_id, x))
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_colloscope_time_slot<T: OrdId>(
    colloscope_time_slot: ColloscopeTimeSlot<T>,
    teacher_ids: &BTreeMap<T, TeacherId>,
) -> Option<ColloscopeTimeSlot<TeacherId>> {
    Some(ColloscopeTimeSlot {
        teacher_id: teacher_ids.get(&colloscope_time_slot.teacher_id).cloned()?,
        start: colloscope_time_slot.start,
        room: colloscope_time_slot.room,
        group_assignments: colloscope_time_slot.group_assignments,
    })
}

fn translate_colloscope_subject<T: OrdId, V: OrdId>(
    colloscope_subject: ColloscopeSubject<T, V>,
    teacher_ids: &BTreeMap<T, TeacherId>,
    student_ids: &BTreeMap<V, StudentId>,
) -> Option<ColloscopeSubject<TeacherId, StudentId>> {
    Some(ColloscopeSubject {
        time_slots: colloscope_subject
            .time_slots
            .into_iter()
            .map(|x| translate_colloscope_time_slot(x, teacher_ids))
            .collect::<Option<_>>()?,
        group_list: translate_colloscope_group_list(colloscope_subject.group_list, student_ids)?,
    })
}

fn translate_colloscope<T: OrdId, U: OrdId, V: OrdId>(
    colloscope: Colloscope<T, U, V>,
    teacher_ids: &BTreeMap<T, TeacherId>,
    subject_ids: &BTreeMap<U, SubjectId>,
    student_ids: &BTreeMap<V, StudentId>,
) -> Option<Colloscope<TeacherId, SubjectId, StudentId>> {
    Some(Colloscope {
        name: colloscope.name,
        subjects: colloscope
            .subjects
            .into_iter()
            .map(|(subject_id, colloscope_subject)| {
                let new_subject_id = subject_ids.get(&subject_id).cloned()?;

                let new_colloscope_subject =
                    translate_colloscope_subject(colloscope_subject, teacher_ids, student_ids)?;

                Some((new_subject_id, new_colloscope_subject))
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_slot_selection<T: OrdId, U: OrdId>(
    slot_selection: SlotSelection<T, U>,
    subject_ids: &BTreeMap<T, SubjectId>,
    time_slot_ids: &BTreeMap<U, TimeSlotId>,
) -> Option<SlotSelection<SubjectId, TimeSlotId>> {
    Some(SlotSelection {
        subject_id: subject_ids.get(&slot_selection.subject_id).cloned()?,
        slot_groups: slot_selection
            .slot_groups
            .into_iter()
            .map(|slot_group| {
                Some(SlotGroup {
                    slots: slot_group
                        .slots
                        .into_iter()
                        .map(|slot_id| time_slot_ids.get(&slot_id).cloned())
                        .collect::<Option<_>>()?,
                    count: slot_group.count,
                })
            })
            .collect::<Option<_>>()?,
    })
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WeekPatternId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TeacherId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StudentId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SubjectGroupId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IncompatId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GroupListId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SubjectId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TimeSlotId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GroupingId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GroupingIncompatId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ColloscopeId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SlotSelectionId(u64);

macro_rules! impl_backup_id_from {
    ($id:ty) => {
        impl From<u64> for $id {
            fn from(value: u64) -> Self {
                Self(value)
            }
        }
    };
}
impl_backup_id_from!(WeekPatternId);
impl_backup_id_from!(TeacherId);
impl_backup_id_from!(StudentId);
impl_backup_id_from!(SubjectGroupId);
impl_backup_id_from!(IncompatId);
impl_backup_id_from!(GroupListId);
impl_backup_id_from!(SubjectId);
impl_backup_id_from!(TimeSlotId);
impl_backup_id_from!(GroupingId);
impl_backup_id_from!(GroupingIncompatId);
impl_backup_id_from!(ColloscopeId);
impl_backup_id_from!(SlotSelectionId);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JsonStudent {
    pub student: Student,
    pub subject_groups: BTreeMap<SubjectGroupId, SubjectId>,
    pub incompats: BTreeSet<IncompatId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct JsonData {
    general_data: GeneralData,
    week_patterns: BTreeMap<WeekPatternId, WeekPattern>,
    teachers: BTreeMap<TeacherId, Teacher>,
    students: BTreeMap<StudentId, JsonStudent>,
    subject_groups: BTreeMap<SubjectGroupId, SubjectGroup>,
    incompats: BTreeMap<IncompatId, Incompat<WeekPatternId>>,
    group_lists: BTreeMap<GroupListId, GroupList<StudentId>>,
    subjects: BTreeMap<SubjectId, Subject<SubjectGroupId, IncompatId, GroupListId>>,
    time_slots: BTreeMap<TimeSlotId, TimeSlot<SubjectId, TeacherId, WeekPatternId>>,
    groupings: BTreeMap<GroupingId, Grouping<TimeSlotId>>,
    grouping_incompats: BTreeMap<GroupingIncompatId, GroupingIncompat<GroupingId>>,
    colloscopes: BTreeMap<ColloscopeId, Colloscope<TeacherId, SubjectId, StudentId>>,
    slot_selections: BTreeMap<SlotSelectionId, SlotSelection<SubjectId, TimeSlotId>>,
}

impl JsonData {
    pub fn new() -> Self {
        JsonData::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedJson {
    validated: JsonData,
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Id {0} appears twice in data")]
    DuplicatedId(u64),
    #[error("Student {0:?} has invalid subject assigned for subject group {1:?}")]
    StudentWithBadSubjectGroupAssignment(StudentId, SubjectGroupId),
    #[error("Week pattern {0:?} has weeks outside of last week")]
    WeekPatternWithWeekTooBig(WeekPatternId),
    #[error("Group list {0:?} has an invalid group number")]
    GroupListWithInvalidGroupNumber(GroupListId),
    #[error("Week pattern id {0:?} is referenced but does not exist")]
    BadWeekPatternId(WeekPatternId),
    #[error("Subject group id {0:?} is referenced but does not exist")]
    BadSubjectGroupId(SubjectGroupId),
    #[error("Subject id {0:?} is referenced but does not exist")]
    BadSubjectId(SubjectId),
    #[error("Incompat id {0:?} is referenced but does not exist")]
    BadIncompatId(IncompatId),
    #[error("Student id {0:?} is referenced but does not exist")]
    BadStudentId(StudentId),
    #[error("Group list id {0:?} is referenced but does not exist")]
    BadGroupListId(GroupListId),
    #[error("Teacher id {0:?} is referenced but does not exist")]
    BadTeacherId(TeacherId),
    #[error("Time slot id {0:?} is referenced but does not exist")]
    BadTimeSlotId(TimeSlotId),
    #[error("Grouping id {0:?} is referenced but does not exist")]
    BadGroupingId(GroupingId),
}

pub type ValidationResult<T> = std::result::Result<T, ValidationError>;

impl JsonData {
    fn find_duplicated_id(&self) -> Option<u64> {
        let mut ids = BTreeSet::new();

        for (id, _) in &self.week_patterns {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.teachers {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.students {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.subject_groups {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.incompats {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.group_lists {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.subjects {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.time_slots {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.groupings {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.grouping_incompats {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.colloscopes {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }
        for (id, _) in &self.slot_selections {
            if !ids.insert(id.0) {
                return Some(id.0);
            }
        }

        None
    }

    fn validate_week_patterns(&self) -> ValidationResult<()> {
        for (week_pattern_id, week_pattern) in &self.week_patterns {
            if let Some(last_week) = week_pattern.weeks.last() {
                if last_week.0 >= self.general_data.week_count.get() {
                    return Err(ValidationError::WeekPatternWithWeekTooBig(*week_pattern_id));
                }
            }
        }
        Ok(())
    }

    fn validate_students(&self) -> ValidationResult<()> {
        for (student_id, student) in &self.students {
            for (subject_group_id, subject_id) in &student.subject_groups {
                if !self.subject_groups.contains_key(subject_group_id) {
                    return Err(ValidationError::BadSubjectGroupId(*subject_group_id));
                }
                match self.subjects.get(subject_id) {
                    Some(subject) => {
                        if subject.subject_group_id != *subject_group_id {
                            return Err(ValidationError::StudentWithBadSubjectGroupAssignment(
                                *student_id,
                                *subject_group_id,
                            ));
                        }
                    }
                    None => {
                        return Err(ValidationError::BadSubjectId(*subject_id));
                    }
                }
            }
            for incompat_id in &student.incompats {
                if !self.incompats.contains_key(incompat_id) {
                    return Err(ValidationError::BadIncompatId(*incompat_id));
                }
            }
        }
        Ok(())
    }

    fn validate_incompats(&self) -> ValidationResult<()> {
        for (_, incompat) in &self.incompats {
            for group in &incompat.groups {
                for slot in &group.slots {
                    if !self.week_patterns.contains_key(&slot.week_pattern_id) {
                        return Err(ValidationError::BadWeekPatternId(slot.week_pattern_id));
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_group_lists(&self) -> ValidationResult<()> {
        for (group_list_id, group_list) in &self.group_lists {
            for (student_id, group) in &group_list.students_mapping {
                if !self.students.contains_key(student_id) {
                    return Err(ValidationError::BadStudentId(*student_id));
                }
                if *group >= group_list.groups.len() {
                    return Err(ValidationError::GroupListWithInvalidGroupNumber(
                        *group_list_id,
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_subjects(&self) -> ValidationResult<()> {
        for (_, subject) in &self.subjects {
            if !self.subject_groups.contains_key(&subject.subject_group_id) {
                return Err(ValidationError::BadSubjectGroupId(subject.subject_group_id));
            }
            if let Some(incompat_id) = &subject.incompat_id {
                if !self.incompats.contains_key(incompat_id) {
                    return Err(ValidationError::BadIncompatId(*incompat_id));
                }
            }
            if let Some(group_list_id) = &subject.group_list_id {
                if !self.group_lists.contains_key(group_list_id) {
                    return Err(ValidationError::BadGroupListId(*group_list_id));
                }
            }
        }
        Ok(())
    }

    fn validate_time_slots(&self) -> ValidationResult<()> {
        for (_, time_slot) in &self.time_slots {
            if !self.subjects.contains_key(&time_slot.subject_id) {
                return Err(ValidationError::BadSubjectId(time_slot.subject_id));
            }
            if !self.teachers.contains_key(&time_slot.teacher_id) {
                return Err(ValidationError::BadTeacherId(time_slot.teacher_id));
            }
            if !self.week_patterns.contains_key(&time_slot.week_pattern_id) {
                return Err(ValidationError::BadWeekPatternId(time_slot.week_pattern_id));
            }
        }
        Ok(())
    }

    fn validate_groupings(&self) -> ValidationResult<()> {
        for (_, grouping) in &self.groupings {
            for time_slot_id in &grouping.slots {
                if !self.time_slots.contains_key(time_slot_id) {
                    return Err(ValidationError::BadTimeSlotId(*time_slot_id));
                }
            }
        }
        Ok(())
    }

    fn validate_grouping_incompats(&self) -> ValidationResult<()> {
        for (_, grouping_incompat) in &self.grouping_incompats {
            for grouping_id in &grouping_incompat.groupings {
                if !self.groupings.contains_key(grouping_id) {
                    return Err(ValidationError::BadGroupingId(*grouping_id));
                }
            }
        }
        Ok(())
    }

    fn validate_colloscopes(&self) -> ValidationResult<()> {
        for (_, colloscope) in &self.colloscopes {
            for (subject_id, subject) in &colloscope.subjects {
                if !self.subjects.contains_key(subject_id) {
                    return Err(ValidationError::BadSubjectId(*subject_id));
                }
                for time_slot in &subject.time_slots {
                    if !self.teachers.contains_key(&time_slot.teacher_id) {
                        return Err(ValidationError::BadTeacherId(time_slot.teacher_id));
                    }
                }
                for (student_id, _) in &subject.group_list.students_mapping {
                    if !self.students.contains_key(student_id) {
                        return Err(ValidationError::BadStudentId(*student_id));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_slot_selections(&self) -> ValidationResult<()> {
        for (_, slot_selection) in &self.slot_selections {
            if !self.subjects.contains_key(&slot_selection.subject_id) {
                return Err(ValidationError::BadSubjectId(slot_selection.subject_id));
            }
            for slot_group in &slot_selection.slot_groups {
                for time_slot_id in &slot_group.slots {
                    if !self.time_slots.contains_key(time_slot_id) {
                        return Err(ValidationError::BadTimeSlotId(*time_slot_id));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate(self) -> ValidationResult<ValidatedJson> {
        if let Some(id) = self.find_duplicated_id() {
            return Err(ValidationError::DuplicatedId(id));
        }

        self.validate_week_patterns()?;
        self.validate_students()?;
        self.validate_incompats()?;
        self.validate_group_lists()?;
        self.validate_subjects()?;
        self.validate_time_slots()?;
        self.validate_groupings()?;
        self.validate_grouping_incompats()?;
        self.validate_colloscopes()?;
        self.validate_slot_selections()?;

        Ok(ValidatedJson { validated: self })
    }
}

impl Default for ValidatedJson {
    fn default() -> Self {
        JsonData::default()
            .validate()
            .expect("Default JsonData should be valid")
    }
}

impl ValidatedJson {
    fn new() -> Self {
        ValidatedJson::default()
    }
}

#[derive(Debug, Error)]
pub enum FromLogicError<T: std::fmt::Debug + std::error::Error> {
    #[error("Error while retrieving data from backend")]
    InternalError(#[from] T),
    #[error("Data has inconstent ids")]
    InconsistentId,
    #[error("Data is not valid: {0}")]
    UnvalidData(ValidationError),
}

pub type FromLogicResult<T, E> = std::result::Result<T, FromLogicError<E>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonStore {
    next_id: u64,
    data: ValidatedJson,
}

impl JsonStore {
    pub fn from_logic<T: Storage>(
        logic: &Logic<T>,
    ) -> FromLogicResult<JsonStore, T::InternalError> {
        let mut next_id = 0;
        let general_data = logic.general_data_get()?;

        let mut week_pattern_ids = BTreeMap::<T::WeekPatternId, WeekPatternId>::new();
        let week_patterns: BTreeMap<_, _> = logic
            .week_patterns_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut week_pattern_ids, id);
                (new_id, x)
            })
            .collect();

        let mut teacher_ids = BTreeMap::<T::TeacherId, TeacherId>::new();
        let teachers: BTreeMap<_, _> = logic
            .teachers_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut teacher_ids, id);
                (new_id, x)
            })
            .collect();

        let mut student_ids = BTreeMap::<T::StudentId, StudentId>::new();
        let mut students: BTreeMap<_, _> = logic
            .students_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut student_ids, id);
                let json_student = JsonStudent {
                    student: x,
                    subject_groups: BTreeMap::new(),
                    incompats: BTreeSet::new(),
                };

                (new_id, json_student)
            })
            .collect();

        let mut subject_group_ids = BTreeMap::<T::SubjectGroupId, SubjectGroupId>::new();
        let subject_groups: BTreeMap<_, _> = logic
            .subject_groups_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut subject_group_ids, id);
                (new_id, x)
            })
            .collect();

        let mut incompat_ids = BTreeMap::<T::IncompatId, IncompatId>::new();
        let incompats: BTreeMap<_, _> = logic
            .incompats_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut incompat_ids, id);
                FromLogicResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_incompat(x, &week_pattern_ids)
                        .ok_or(FromLogicError::InconsistentId)?,
                ))
            })
            .collect::<FromLogicResult<_, _>>()?;

        let mut group_list_ids = BTreeMap::<T::GroupListId, GroupListId>::new();
        let group_lists: BTreeMap<_, _> = logic
            .group_lists_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut group_list_ids, id);
                FromLogicResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_group_list(x, &student_ids).ok_or(FromLogicError::InconsistentId)?,
                ))
            })
            .collect::<FromLogicResult<_, _>>()?;

        let mut subject_ids = BTreeMap::<T::SubjectId, SubjectId>::new();
        let subjects: BTreeMap<_, _> = logic
            .subjects_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut subject_ids, id);
                FromLogicResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_subject(x, &subject_group_ids, &incompat_ids, &group_list_ids)
                        .ok_or(FromLogicError::InconsistentId)?,
                ))
            })
            .collect::<FromLogicResult<_, _>>()?;

        let mut time_slot_ids = BTreeMap::<T::TimeSlotId, TimeSlotId>::new();
        let time_slots: BTreeMap<_, _> = logic
            .time_slots_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut time_slot_ids, id);
                FromLogicResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_time_slot(x, &subject_ids, &teacher_ids, &week_pattern_ids)
                        .ok_or(FromLogicError::InconsistentId)?,
                ))
            })
            .collect::<FromLogicResult<_, _>>()?;

        let mut grouping_ids = BTreeMap::<T::GroupingId, GroupingId>::new();
        let groupings: BTreeMap<_, _> = logic
            .groupings_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut grouping_ids, id);
                FromLogicResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_grouping(x, &time_slot_ids).ok_or(FromLogicError::InconsistentId)?,
                ))
            })
            .collect::<FromLogicResult<_, _>>()?;

        let mut grouping_incompat_ids =
            BTreeMap::<T::GroupingIncompatId, GroupingIncompatId>::new();
        let grouping_incompats: BTreeMap<_, _> = logic
            .grouping_incompats_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut grouping_incompat_ids, id);
                FromLogicResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_grouping_incompat(x, &grouping_ids)
                        .ok_or(FromLogicError::InconsistentId)?,
                ))
            })
            .collect::<FromLogicResult<_, _>>()?;

        let mut colloscope_ids = BTreeMap::<T::ColloscopeId, ColloscopeId>::new();
        let colloscopes: BTreeMap<_, _> = logic
            .colloscopes_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut colloscope_ids, id);
                FromLogicResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_colloscope(x, &teacher_ids, &subject_ids, &student_ids)
                        .ok_or(FromLogicError::InconsistentId)?,
                ))
            })
            .collect::<FromLogicResult<_, _>>()?;

        let mut slot_selection_ids = BTreeMap::<T::SlotSelectionId, SlotSelectionId>::new();
        let slot_selections: BTreeMap<_, _> = logic
            .slot_selections_get_all()?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_next_id(&mut next_id, &mut slot_selection_ids, id);
                FromLogicResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_slot_selection(x, &subject_ids, &time_slot_ids)
                        .ok_or(FromLogicError::InconsistentId)?,
                ))
            })
            .collect::<FromLogicResult<_, _>>()?;

        for (old_student_id, new_student_id) in &student_ids {
            let student_data = students
                .get_mut(new_student_id)
                .ok_or(FromLogicError::InconsistentId)?;

            for (old_incompat_id, new_incompat_id) in &incompat_ids {
                if logic
                    .incompat_for_student_get(*old_student_id, *old_incompat_id)
                    .map_err(|e| match e {
                        Id2Error::InternalError(int_err) => FromLogicError::InternalError(int_err),
                        Id2Error::InvalidId1(_) => FromLogicError::InconsistentId,
                        Id2Error::InvalidId2(_) => FromLogicError::InconsistentId,
                    })?
                {
                    student_data.incompats.insert(*new_incompat_id);
                }
            }

            for (old_subject_group_id, new_subject_group_id) in &subject_group_ids {
                let old_subject_id_opt = logic
                    .subject_group_for_student_get(*old_student_id, *old_subject_group_id)
                    .map_err(|e| match e {
                        Id2Error::InternalError(int_err) => FromLogicError::InternalError(int_err),
                        Id2Error::InvalidId1(_) => FromLogicError::InconsistentId,
                        Id2Error::InvalidId2(_) => FromLogicError::InconsistentId,
                    })?;

                if let Some(old_subject_id) = old_subject_id_opt {
                    let new_subject_id = subject_ids
                        .get(&old_subject_id)
                        .cloned()
                        .ok_or(FromLogicError::InconsistentId)?;
                    student_data
                        .subject_groups
                        .insert(*new_subject_group_id, new_subject_id);
                }
            }
        }

        Ok(JsonStore {
            next_id,
            data: JsonData {
                general_data,
                week_patterns,
                teachers,
                students,
                subject_groups,
                incompats,
                group_lists,
                subjects,
                time_slots,
                groupings,
                grouping_incompats,
                colloscopes,
                slot_selections,
            }
            .validate()
            .map_err(FromLogicError::UnvalidData)?,
        })
    }
}

impl Default for JsonStore {
    fn default() -> Self {
        JsonStore {
            next_id: 0,
            data: ValidatedJson::new(),
        }
    }
}

impl JsonStore {
    pub fn new() -> Self {
        JsonStore::default()
    }
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("Error while outputting json: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Error while writing file: {0}")]
    IO(#[from] std::io::Error),
}

pub type SaveResult<T> = std::result::Result<T, SaveError>;

impl JsonStore {
    pub fn to_json(&self) -> serde_json::Result<String> {
        Ok(serde_json::to_string_pretty(&self.data.validated)?)
    }

    pub fn to_json_file(&self, path: &std::path::Path) -> SaveResult<()> {
        let content = self.to_json()?;
        let mut file = std::fs::File::create(path)?;

        use std::io::Write;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum FromJsonError {
    #[error("Error while reading json: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Possible malicious (or corrupted) file: last id exceeds 2^63")]
    EndOfTheUniverseReached,
    #[error("Data is not valid: {0}")]
    UnvalidData(#[from] ValidationError),
}

pub type FromJsonResult<T> = std::result::Result<T, FromJsonError>;

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("Error while decoding data: {0}")]
    FromJsonError(#[from] FromJsonError),
    #[error("Error while reading file: {0}")]
    IO(#[from] std::io::Error),
}

pub type OpenResult<T> = std::result::Result<T, OpenError>;

impl JsonStore {
    pub fn from_json_file(path: &std::path::Path) -> OpenResult<Self> {
        let mut file = std::fs::File::open(path)?;
        let mut content = String::new();

        use std::io::Read;
        file.read_to_string(&mut content)?;
        Ok(Self::from_json(&content)?)
    }

    pub fn from_json(content: &str) -> FromJsonResult<Self> {
        let data = serde_json::from_str::<JsonData>(content)?.validate()?;

        let next_id = match data.find_last_id() {
            Some(last) => {
                if last >= (1u64 << 63) {
                    return Err(FromJsonError::EndOfTheUniverseReached);
                } else {
                    last + 1
                }
            }
            None => 0,
        };

        Ok(JsonStore { next_id, data })
    }
}

impl JsonData {
    fn find_last_id(&self) -> Option<u64> {
        let mut ids = BTreeSet::new();
        for (id, _) in &self.week_patterns {
            ids.insert(id.0);
        }
        for (id, _) in &self.teachers {
            ids.insert(id.0);
        }
        for (id, _) in &self.students {
            ids.insert(id.0);
        }
        for (id, _) in &self.subject_groups {
            ids.insert(id.0);
        }
        for (id, _) in &self.incompats {
            ids.insert(id.0);
        }
        for (id, _) in &self.group_lists {
            ids.insert(id.0);
        }
        for (id, _) in &self.subjects {
            ids.insert(id.0);
        }
        for (id, _) in &self.time_slots {
            ids.insert(id.0);
        }
        for (id, _) in &self.groupings {
            ids.insert(id.0);
        }
        for (id, _) in &self.grouping_incompats {
            ids.insert(id.0);
        }
        for (id, _) in &self.colloscopes {
            ids.insert(id.0);
        }
        for (id, _) in &self.slot_selections {
            ids.insert(id.0);
        }
        ids.last().copied()
    }
}

impl Deref for ValidatedJson {
    type Target = JsonData;

    fn deref(&self) -> &Self::Target {
        &self.validated
    }
}

#[derive(Debug, Error)]
pub enum InternalError {
    #[error("Data was corrupted and we now have inconsistent ids")]
    CorruptedData,
    #[error("Invalid ID")]
    InvalidId,
}

impl JsonStore {
    fn get_id(&mut self) -> u64 {
        let result = self.next_id;
        self.next_id += 1;
        result
    }
}

impl Storage for JsonStore {
    type WeekPatternId = WeekPatternId;
    type TeacherId = TeacherId;
    type StudentId = StudentId;
    type SubjectGroupId = SubjectGroupId;
    type IncompatId = IncompatId;
    type GroupListId = GroupListId;
    type SubjectId = SubjectId;
    type TimeSlotId = TimeSlotId;
    type GroupingId = GroupingId;
    type GroupingIncompatId = GroupingIncompatId;
    type ColloscopeId = ColloscopeId;
    type SlotSelectionId = SlotSelectionId;

    type InternalError = InternalError;

    unsafe fn general_data_set_unchecked(
        &mut self,
        general_data: &GeneralData,
    ) -> std::result::Result<(), Self::InternalError> {
        self.data.validated.general_data = general_data.clone();
        Ok(())
    }
    fn general_data_get(&self) -> std::result::Result<GeneralData, Self::InternalError> {
        Ok(self.data.validated.general_data.clone())
    }

    fn week_patterns_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::WeekPatternId, WeekPattern>, Self::InternalError> {
        Ok(self.data.validated.week_patterns.clone())
    }
    fn week_patterns_get(
        &self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<WeekPattern, IdError<Self::InternalError, Self::WeekPatternId>> {
        match self.data.validated.week_patterns.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn week_patterns_add_unchecked(
        &mut self,
        pattern: &WeekPattern,
    ) -> std::result::Result<Self::WeekPatternId, Self::InternalError> {
        let id = WeekPatternId(self.get_id());
        if self
            .data
            .validated
            .week_patterns
            .insert(id, pattern.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn week_patterns_remove_unchecked(
        &mut self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.week_patterns.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn week_patterns_update_unchecked(
        &mut self,
        index: Self::WeekPatternId,
        pattern: &WeekPattern,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.week_patterns.get_mut(&index) {
            Some(x) => {
                *x = pattern.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    fn teachers_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::TeacherId, Teacher>, Self::InternalError> {
        Ok(self.data.validated.teachers.clone())
    }
    fn teachers_get(
        &self,
        index: Self::TeacherId,
    ) -> std::result::Result<Teacher, IdError<Self::InternalError, Self::TeacherId>> {
        match self.data.validated.teachers.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    fn teachers_add(
        &mut self,
        teacher: &Teacher,
    ) -> std::result::Result<Self::TeacherId, Self::InternalError> {
        let id = TeacherId(self.get_id());
        if self
            .data
            .validated
            .teachers
            .insert(id, teacher.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn teachers_remove_unchecked(
        &mut self,
        index: Self::TeacherId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.teachers.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    fn teachers_update(
        &mut self,
        index: Self::TeacherId,
        teacher: &Teacher,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>> {
        match self.data.validated.teachers.get_mut(&index) {
            Some(x) => {
                *x = teacher.clone();
                Ok(())
            }
            None => Err(IdError::InvalidId(index)),
        }
    }

    fn students_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::StudentId, Student>, Self::InternalError> {
        Ok(self
            .data
            .validated
            .students
            .iter()
            .map(|(id, x)| (*id, x.student.clone()))
            .collect())
    }
    fn students_get(
        &self,
        index: Self::StudentId,
    ) -> std::result::Result<Student, IdError<Self::InternalError, Self::StudentId>> {
        match self.data.validated.students.get(&index) {
            Some(x) => Ok(x.student.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    fn students_add(
        &mut self,
        student: &Student,
    ) -> std::result::Result<Self::StudentId, Self::InternalError> {
        let id = StudentId(self.get_id());
        let new_entry = JsonStudent {
            student: student.clone(),
            subject_groups: BTreeMap::new(),
            incompats: BTreeSet::new(),
        };
        if self.data.validated.students.insert(id, new_entry).is_some() {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn students_remove_unchecked(
        &mut self,
        index: Self::StudentId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.students.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    fn students_update(
        &mut self,
        index: Self::StudentId,
        student: &Student,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::StudentId>> {
        match self.data.validated.students.get_mut(&index) {
            Some(x) => {
                x.student = student.clone();
                Ok(())
            }
            None => Err(IdError::InvalidId(index)),
        }
    }

    fn subject_groups_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::SubjectGroupId, SubjectGroup>, Self::InternalError>
    {
        Ok(self.data.validated.subject_groups.clone())
    }
    fn subject_groups_get(
        &self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<SubjectGroup, IdError<Self::InternalError, Self::SubjectGroupId>> {
        match self.data.validated.subject_groups.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    fn subject_groups_add(
        &mut self,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<Self::SubjectGroupId, Self::InternalError> {
        let id = SubjectGroupId(self.get_id());
        if self
            .data
            .validated
            .subject_groups
            .insert(id, subject_group.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn subject_groups_remove_unchecked(
        &mut self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.subject_groups.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    fn subject_groups_update(
        &mut self,
        index: Self::SubjectGroupId,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>> {
        match self.data.validated.subject_groups.get_mut(&index) {
            Some(x) => {
                *x = subject_group.clone();
                Ok(())
            }
            None => Err(IdError::InvalidId(index)),
        }
    }

    fn incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::IncompatId, Incompat<Self::WeekPatternId>>,
        Self::InternalError,
    > {
        Ok(self.data.validated.incompats.clone())
    }
    fn incompats_get(
        &self,
        index: Self::IncompatId,
    ) -> std::result::Result<
        Incompat<Self::WeekPatternId>,
        IdError<Self::InternalError, Self::IncompatId>,
    > {
        match self.data.validated.incompats.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn incompats_add_unchecked(
        &mut self,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<Self::IncompatId, Self::InternalError> {
        let id = IncompatId(self.get_id());
        if self
            .data
            .validated
            .incompats
            .insert(id, incompat.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn incompats_remove_unchecked(
        &mut self,
        index: Self::IncompatId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.incompats.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn incompats_update_unchecked(
        &mut self,
        index: Self::IncompatId,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.incompats.get_mut(&index) {
            Some(x) => {
                *x = incompat.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    fn group_lists_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupListId, GroupList<Self::StudentId>>,
        Self::InternalError,
    > {
        Ok(self.data.validated.group_lists.clone())
    }
    fn group_lists_get(
        &self,
        index: Self::GroupListId,
    ) -> std::result::Result<
        GroupList<Self::StudentId>,
        IdError<Self::InternalError, Self::GroupListId>,
    > {
        match self.data.validated.group_lists.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn group_lists_add_unchecked(
        &mut self,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<Self::GroupListId, Self::InternalError> {
        let id = GroupListId(self.get_id());
        if self
            .data
            .validated
            .group_lists
            .insert(id, group_list.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn group_lists_remove_unchecked(
        &mut self,
        index: Self::GroupListId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.group_lists.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn group_lists_update_unchecked(
        &mut self,
        index: Self::GroupListId,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.group_lists.get_mut(&index) {
            Some(x) => {
                *x = group_list.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    fn subjects_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<
            Self::SubjectId,
            Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        >,
        Self::InternalError,
    > {
        Ok(self.data.validated.subjects.clone())
    }
    fn subjects_get(
        &self,
        index: Self::SubjectId,
    ) -> std::result::Result<
        Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        IdError<Self::InternalError, Self::SubjectId>,
    > {
        match self.data.validated.subjects.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn subjects_add_unchecked(
        &mut self,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<Self::SubjectId, Self::InternalError> {
        let id = SubjectId(self.get_id());
        if self
            .data
            .validated
            .subjects
            .insert(id, subject.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn subjects_remove_unchecked(
        &mut self,
        index: Self::SubjectId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.subjects.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn subjects_update_unchecked(
        &mut self,
        index: Self::SubjectId,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.subjects.get_mut(&index) {
            Some(x) => {
                *x = subject.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    fn time_slots_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::TimeSlotId, TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>>,
        Self::InternalError,
    > {
        Ok(self.data.validated.time_slots.clone())
    }
    fn time_slots_get(
        &self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<
        TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
        IdError<Self::InternalError, Self::TimeSlotId>,
    > {
        match self.data.validated.time_slots.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn time_slots_add_unchecked(
        &mut self,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<Self::TimeSlotId, Self::InternalError> {
        let id = TimeSlotId(self.get_id());
        if self
            .data
            .validated
            .time_slots
            .insert(id, time_slot.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn time_slots_remove_unchecked(
        &mut self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.time_slots.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn time_slots_update_unchecked(
        &mut self,
        index: Self::TimeSlotId,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.time_slots.get_mut(&index) {
            Some(x) => {
                *x = time_slot.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    fn groupings_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupingId, Grouping<Self::TimeSlotId>>,
        Self::InternalError,
    > {
        Ok(self.data.validated.groupings.clone())
    }
    fn groupings_get(
        &self,
        index: Self::GroupingId,
    ) -> std::result::Result<
        Grouping<Self::TimeSlotId>,
        IdError<Self::InternalError, Self::GroupingId>,
    > {
        match self.data.validated.groupings.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn groupings_add_unchecked(
        &mut self,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<Self::GroupingId, Self::InternalError> {
        let id = GroupingId(self.get_id());
        if self
            .data
            .validated
            .groupings
            .insert(id, grouping.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn groupings_remove_unchecked(
        &mut self,
        index: Self::GroupingId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.groupings.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn groupings_update_unchecked(
        &mut self,
        index: Self::GroupingId,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.groupings.get_mut(&index) {
            Some(x) => {
                *x = grouping.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    fn grouping_incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupingIncompatId, GroupingIncompat<Self::GroupingId>>,
        Self::InternalError,
    > {
        Ok(self.data.validated.grouping_incompats.clone())
    }
    fn grouping_incompats_get(
        &self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<
        GroupingIncompat<Self::GroupingId>,
        IdError<Self::InternalError, Self::GroupingIncompatId>,
    > {
        match self.data.validated.grouping_incompats.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn grouping_incompats_add_unchecked(
        &mut self,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<Self::GroupingIncompatId, Self::InternalError> {
        let id = GroupingIncompatId(self.get_id());
        if self
            .data
            .validated
            .grouping_incompats
            .insert(id, grouping_incompat.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn grouping_incompats_remove_unchecked(
        &mut self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self
            .data
            .validated
            .grouping_incompats
            .remove(&index)
            .is_none()
        {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn grouping_incompats_update_unchecked(
        &mut self,
        index: Self::GroupingIncompatId,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.grouping_incompats.get_mut(&index) {
            Some(x) => {
                *x = grouping_incompat.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    fn colloscopes_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::ColloscopeId, Colloscope<Self::TeacherId, Self::SubjectId, Self::StudentId>>,
        Self::InternalError,
    > {
        Ok(self.data.validated.colloscopes.clone())
    }
    fn colloscopes_get(
        &self,
        index: Self::ColloscopeId,
    ) -> std::result::Result<
        Colloscope<Self::TeacherId, Self::SubjectId, Self::StudentId>,
        IdError<Self::InternalError, Self::ColloscopeId>,
    > {
        match self.data.validated.colloscopes.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn colloscopes_add_unchecked(
        &mut self,
        colloscope: &Colloscope<Self::TeacherId, Self::SubjectId, Self::StudentId>,
    ) -> std::result::Result<Self::ColloscopeId, Self::InternalError> {
        let id = ColloscopeId(self.get_id());
        if self
            .data
            .validated
            .colloscopes
            .insert(id, colloscope.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn colloscopes_remove_unchecked(
        &mut self,
        index: Self::ColloscopeId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.colloscopes.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn colloscopes_update_unchecked(
        &mut self,
        index: Self::ColloscopeId,
        colloscope: &Colloscope<Self::TeacherId, Self::SubjectId, Self::StudentId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.colloscopes.get_mut(&index) {
            Some(x) => {
                *x = colloscope.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    fn slot_selections_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::SlotSelectionId, SlotSelection<Self::SubjectId, Self::TimeSlotId>>,
        Self::InternalError,
    > {
        Ok(self.data.validated.slot_selections.clone())
    }
    fn slot_selections_get(
        &self,
        index: Self::SlotSelectionId,
    ) -> std::result::Result<
        SlotSelection<Self::SubjectId, Self::TimeSlotId>,
        IdError<Self::InternalError, Self::SlotSelectionId>,
    > {
        match self.data.validated.slot_selections.get(&index) {
            Some(x) => Ok(x.clone()),
            None => Err(IdError::InvalidId(index)),
        }
    }
    unsafe fn slot_selections_add_unchecked(
        &mut self,
        slot_selection: &SlotSelection<Self::SubjectId, Self::TimeSlotId>,
    ) -> std::result::Result<Self::SlotSelectionId, Self::InternalError> {
        let id = SlotSelectionId(self.get_id());
        if self
            .data
            .validated
            .slot_selections
            .insert(id, slot_selection.clone())
            .is_some()
        {
            return Err(InternalError::CorruptedData);
        }
        Ok(id)
    }
    unsafe fn slot_selections_remove_unchecked(
        &mut self,
        index: Self::SlotSelectionId,
    ) -> std::result::Result<(), Self::InternalError> {
        if self.data.validated.slot_selections.remove(&index).is_none() {
            return Err(InternalError::InvalidId);
        }
        Ok(())
    }
    unsafe fn slot_selections_update_unchecked(
        &mut self,
        index: Self::SlotSelectionId,
        slot_selection: &SlotSelection<Self::SubjectId, Self::TimeSlotId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.slot_selections.get_mut(&index) {
            Some(x) => {
                *x = slot_selection.clone();
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }

    unsafe fn subject_group_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
        subject_id: Option<Self::SubjectId>,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.students.get_mut(&student_id) {
            Some(json_student) => {
                match subject_id {
                    Some(id) => {
                        json_student.subject_groups.insert(subject_group_id, id);
                    }
                    None => {
                        json_student.subject_groups.remove(&subject_group_id);
                    }
                }
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }
    fn subject_group_for_student_get(
        &self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
    ) -> std::result::Result<
        Option<Self::SubjectId>,
        Id2Error<Self::InternalError, Self::StudentId, Self::SubjectGroupId>,
    > {
        match self.data.validated.students.get(&student_id) {
            Some(json_student) => Ok(json_student.subject_groups.get(&subject_group_id).copied()),
            None => Err(Id2Error::InvalidId1(student_id)),
        }
    }

    unsafe fn incompat_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        incompat_id: Self::IncompatId,
        enabled: bool,
    ) -> std::result::Result<(), Self::InternalError> {
        match self.data.validated.students.get_mut(&student_id) {
            Some(json_student) => {
                if enabled {
                    json_student.incompats.insert(incompat_id);
                } else {
                    json_student.incompats.remove(&incompat_id);
                }
                Ok(())
            }
            None => Err(InternalError::InvalidId),
        }
    }
    fn incompat_for_student_get(
        &self,
        student_id: Self::StudentId,
        incompat_id: Self::IncompatId,
    ) -> std::result::Result<bool, Id2Error<Self::InternalError, Self::StudentId, Self::IncompatId>>
    {
        match self.data.validated.students.get(&student_id) {
            Some(json_student) => Ok(json_student.incompats.contains(&incompat_id)),
            None => Err(Id2Error::InvalidId1(student_id)),
        }
    }
}
