use super::*;

#[derive(Debug, Error)]
pub enum BackupError<T: std::fmt::Debug + std::error::Error> {
    #[error("Error while retrieving data from backend")]
    InternalError(#[from] T),
    #[error("Data has inconstent ids")]
    InconsistentId,
}

pub type BackupResult<T, E> = std::result::Result<T, BackupError<E>>;

fn get_backup_id<T: OrdId, U: OrdId + From<u64>>(
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
    week_pattern_ids: &BTreeMap<T, BackupWeekPatternId>,
) -> Option<Incompat<BackupWeekPatternId>> {
    Some(Incompat {
        name: incompat.name,
        max_count: incompat.max_count,
        groups: incompat
            .groups
            .into_iter()
            .map(|x| {
                Some(IncompatGroup::<BackupWeekPatternId> {
                    slots: x
                        .slots
                        .into_iter()
                        .map(|y| {
                            Some(IncompatSlot::<BackupWeekPatternId> {
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
    student_ids: &BTreeMap<T, BackupStudentId>,
) -> Option<GroupList<BackupStudentId>> {
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
    subject_group_ids: &BTreeMap<T, BackupSubjectGroupId>,
    incompat_ids: &BTreeMap<U, BackupIncompatId>,
    group_list_ids: &BTreeMap<V, BackupGroupListId>,
) -> Option<Subject<BackupSubjectGroupId, BackupIncompatId, BackupGroupListId>> {
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
    subject_ids: &BTreeMap<T, BackupSubjectId>,
    teacher_ids: &BTreeMap<U, BackupTeacherId>,
    week_pattern_ids: &BTreeMap<V, BackupWeekPatternId>,
) -> Option<TimeSlot<BackupSubjectId, BackupTeacherId, BackupWeekPatternId>> {
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
    time_slot_ids: &BTreeMap<T, BackupTimeSlotId>,
) -> Option<Grouping<BackupTimeSlotId>> {
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
    grouping_ids: &BTreeMap<T, BackupGroupingId>,
) -> Option<GroupingIncompat<BackupGroupingId>> {
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
    student_ids: &BTreeMap<V, BackupStudentId>,
) -> Option<ColloscopeGroupList<BackupStudentId>> {
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
    teacher_ids: &BTreeMap<T, BackupTeacherId>,
) -> Option<ColloscopeTimeSlot<BackupTeacherId>> {
    Some(ColloscopeTimeSlot {
        teacher_id: teacher_ids.get(&colloscope_time_slot.teacher_id).cloned()?,
        start: colloscope_time_slot.start,
        room: colloscope_time_slot.room,
        group_assignments: colloscope_time_slot.group_assignments,
    })
}

fn translate_colloscope_subject<T: OrdId, V: OrdId>(
    colloscope_subject: ColloscopeSubject<T, V>,
    teacher_ids: &BTreeMap<T, BackupTeacherId>,
    student_ids: &BTreeMap<V, BackupStudentId>,
) -> Option<ColloscopeSubject<BackupTeacherId, BackupStudentId>> {
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
    teacher_ids: &BTreeMap<T, BackupTeacherId>,
    subject_ids: &BTreeMap<U, BackupSubjectId>,
    student_ids: &BTreeMap<V, BackupStudentId>,
) -> Option<Colloscope<BackupTeacherId, BackupSubjectId, BackupStudentId>> {
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
    subject_ids: &BTreeMap<T, BackupSubjectId>,
    time_slot_ids: &BTreeMap<U, BackupTimeSlotId>,
) -> Option<SlotSelection<BackupSubjectId, BackupTimeSlotId>> {
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
pub struct BackupWeekPatternId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupTeacherId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupStudentId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupSubjectGroupId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupIncompatId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupGroupListId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupSubjectId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupTimeSlotId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupGroupingId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupGroupingIncompatId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupColloscopeId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BackupSlotSelectionId(u64);

macro_rules! impl_backup_id_from {
    ($id:ty) => {
        impl From<u64> for $id {
            fn from(value: u64) -> Self {
                Self(value)
            }
        }
    };
}
impl_backup_id_from!(BackupWeekPatternId);
impl_backup_id_from!(BackupTeacherId);
impl_backup_id_from!(BackupStudentId);
impl_backup_id_from!(BackupSubjectGroupId);
impl_backup_id_from!(BackupIncompatId);
impl_backup_id_from!(BackupGroupListId);
impl_backup_id_from!(BackupSubjectId);
impl_backup_id_from!(BackupTimeSlotId);
impl_backup_id_from!(BackupGroupingId);
impl_backup_id_from!(BackupGroupingIncompatId);
impl_backup_id_from!(BackupColloscopeId);
impl_backup_id_from!(BackupSlotSelectionId);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupAssignments {
    pub subject_groups: BTreeMap<BackupSubjectGroupId, BackupSubjectId>,
    pub incompats: BTreeSet<BackupIncompatId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupData {
    pub next_id: u64,
    pub general_data: GeneralData,
    pub week_patterns: BTreeMap<BackupWeekPatternId, WeekPattern>,
    pub teachers: BTreeMap<BackupTeacherId, Teacher>,
    pub students: BTreeMap<BackupStudentId, Student>,
    pub subject_groups: BTreeMap<BackupSubjectGroupId, SubjectGroup>,
    pub incompats: BTreeMap<BackupIncompatId, Incompat<BackupWeekPatternId>>,
    pub group_lists: BTreeMap<BackupGroupListId, GroupList<BackupStudentId>>,
    pub subjects: BTreeMap<
        BackupSubjectId,
        Subject<BackupSubjectGroupId, BackupIncompatId, BackupGroupListId>,
    >,
    pub time_slots:
        BTreeMap<BackupTimeSlotId, TimeSlot<BackupSubjectId, BackupTeacherId, BackupWeekPatternId>>,
    pub groupings: BTreeMap<BackupGroupingId, Grouping<BackupTimeSlotId>>,
    pub grouping_incompats: BTreeMap<BackupGroupingIncompatId, GroupingIncompat<BackupGroupingId>>,
    pub colloscopes:
        BTreeMap<BackupColloscopeId, Colloscope<BackupTeacherId, BackupSubjectId, BackupStudentId>>,
    pub slot_selections:
        BTreeMap<BackupSlotSelectionId, SlotSelection<BackupSubjectId, BackupTimeSlotId>>,
    pub student_assignments: BTreeMap<BackupStudentId, BackupAssignments>,
}

impl BackupData {
    pub async fn from_logic<T: Storage>(
        logic: &Logic<T>,
    ) -> BackupResult<BackupData, T::InternalError> {
        let mut next_id = 0;
        let general_data = logic.general_data_get().await?;

        let mut week_pattern_ids = BTreeMap::<T::WeekPatternId, BackupWeekPatternId>::new();
        let week_patterns: BTreeMap<_, _> = logic
            .week_patterns_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut week_pattern_ids, id);
                (new_id, x)
            })
            .collect();

        let mut teacher_ids = BTreeMap::<T::TeacherId, BackupTeacherId>::new();
        let teachers: BTreeMap<_, _> = logic
            .teachers_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut teacher_ids, id);
                (new_id, x)
            })
            .collect();

        let mut student_ids = BTreeMap::<T::StudentId, BackupStudentId>::new();
        let students: BTreeMap<_, _> = logic
            .students_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut student_ids, id);
                (new_id, x)
            })
            .collect();

        let mut subject_group_ids = BTreeMap::<T::SubjectGroupId, BackupSubjectGroupId>::new();
        let subject_groups: BTreeMap<_, _> = logic
            .subject_groups_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut subject_group_ids, id);
                (new_id, x)
            })
            .collect();

        let mut incompat_ids = BTreeMap::<T::IncompatId, BackupIncompatId>::new();
        let incompats: BTreeMap<_, _> = logic
            .incompats_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut incompat_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_incompat(x, &week_pattern_ids).ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut group_list_ids = BTreeMap::<T::GroupListId, BackupGroupListId>::new();
        let group_lists: BTreeMap<_, _> = logic
            .group_lists_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut group_list_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_group_list(x, &student_ids).ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut subject_ids = BTreeMap::<T::SubjectId, BackupSubjectId>::new();
        let subjects: BTreeMap<_, _> = logic
            .subjects_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut subject_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_subject(x, &subject_group_ids, &incompat_ids, &group_list_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut time_slot_ids = BTreeMap::<T::TimeSlotId, BackupTimeSlotId>::new();
        let time_slots: BTreeMap<_, _> = logic
            .time_slots_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut time_slot_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_time_slot(x, &subject_ids, &teacher_ids, &week_pattern_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut grouping_ids = BTreeMap::<T::GroupingId, BackupGroupingId>::new();
        let groupings: BTreeMap<_, _> = logic
            .groupings_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut grouping_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_grouping(x, &time_slot_ids).ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut grouping_incompat_ids =
            BTreeMap::<T::GroupingIncompatId, BackupGroupingIncompatId>::new();
        let grouping_incompats: BTreeMap<_, _> = logic
            .grouping_incompats_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut grouping_incompat_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_grouping_incompat(x, &grouping_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut colloscope_ids = BTreeMap::<T::ColloscopeId, BackupColloscopeId>::new();
        let colloscopes: BTreeMap<_, _> = logic
            .colloscopes_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut colloscope_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_colloscope(x, &teacher_ids, &subject_ids, &student_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut slot_selection_ids = BTreeMap::<T::SlotSelectionId, BackupSlotSelectionId>::new();
        let slot_selections: BTreeMap<_, _> = logic
            .slot_selections_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut slot_selection_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_slot_selection(x, &subject_ids, &time_slot_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut student_assignments = BTreeMap::<_, _>::new();
        for (old_student_id, new_student_id) in &student_ids {
            let mut incompats = BTreeSet::<BackupIncompatId>::new();
            for (old_incompat_id, new_incompat_id) in &incompat_ids {
                if logic
                    .incompat_for_student_get(*old_student_id, *old_incompat_id)
                    .await
                    .map_err(|e| match e {
                        Id2Error::InternalError(int_err) => BackupError::InternalError(int_err),
                        Id2Error::InvalidId1(_) => BackupError::InconsistentId,
                        Id2Error::InvalidId2(_) => BackupError::InconsistentId,
                    })?
                {
                    incompats.insert(*new_incompat_id);
                }
            }

            let mut subject_groups = BTreeMap::<BackupSubjectGroupId, BackupSubjectId>::new();
            for (old_subject_group_id, new_subject_group_id) in &subject_group_ids {
                let old_subject_id_opt = logic
                    .subject_group_for_student_get(*old_student_id, *old_subject_group_id)
                    .await
                    .map_err(|e| match e {
                        Id2Error::InternalError(int_err) => BackupError::InternalError(int_err),
                        Id2Error::InvalidId1(_) => BackupError::InconsistentId,
                        Id2Error::InvalidId2(_) => BackupError::InconsistentId,
                    })?;

                if let Some(old_subject_id) = old_subject_id_opt {
                    let new_subject_id = subject_ids
                        .get(&old_subject_id)
                        .cloned()
                        .ok_or(BackupError::InconsistentId)?;
                    subject_groups.insert(*new_subject_group_id, new_subject_id);
                }
            }

            student_assignments.insert(
                *new_student_id,
                BackupAssignments {
                    subject_groups,
                    incompats,
                },
            );
        }

        Ok(BackupData {
            next_id,
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
            student_assignments,
        })
    }
}
