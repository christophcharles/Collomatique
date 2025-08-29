pub mod sqlite;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdError<T, Id>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CheckedIdError<T, Id, CheckData>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CheckData: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Check failed. Data provided is: {0:?}")]
    CheckFailed(CheckData),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

impl<T, Id, CheckData> CheckedIdError<T, Id, CheckData>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CheckData: std::fmt::Debug,
{
    fn from_id_error(id_error: IdError<T, Id>) -> Self {
        match id_error {
            IdError::InvalidId(id) => CheckedIdError::InvalidId(id),
            IdError::InternalError(int_err) => CheckedIdError::InternalError(int_err),
        }
    }
}

#[derive(Error, Debug)]
pub enum Id2Error<T, Id1, Id2>
where
    T: std::fmt::Debug + std::error::Error,
    Id1: std::fmt::Debug,
    Id2: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId1(Id1),
    #[error("Id {0:?} is invalid")]
    InvalidId2(Id2),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossId2Error<T, Id1, Id2, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Id1: std::fmt::Debug,
    Id2: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId1(Id1),
    #[error("Id {0:?} is invalid")]
    InvalidId2(Id2),
    #[error("Id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossError<T, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossIdError<T, Id, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross3Error<T, CrossId1, CrossId2, CrossId3>
where
    T: std::fmt::Debug + std::error::Error,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId3(CrossId3),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross3IdError<T, Id, CrossId1, CrossId2, CrossId3>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId3(CrossId3),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum InvalidCrossError<T, Data, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Data: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Data to be stored is invalid: {0:?}")]
    InvalidData(Data),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum InvalidCrossIdError<T, Data, Id, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Data: std::fmt::Debug,
    Id: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Data to be stored is invalid: {0:?}")]
    InvalidData(Data),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

pub trait OrdId:
    std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord + Send + Sync + Copy
{
}
impl<T: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord + Send + Sync + Copy> OrdId
    for T
{
}

#[derive(Error, Debug)]
pub enum WeekPatternDependancy<IncompatId: OrdId, TimeSlotId: OrdId> {
    Incompat(IncompatId),
    TimeSlot(TimeSlotId),
}

#[derive(Error, Debug)]
pub enum SubjectGroupDependancy<SubjectId: OrdId, StudentId: OrdId> {
    Subject(SubjectId),
    Student(StudentId),
}

#[derive(Error, Debug)]
pub enum IncompatDependancy<SubjectId: OrdId, StudentId: OrdId> {
    Subject(SubjectId),
    Student(StudentId),
}

#[derive(Error, Debug)]
pub enum SubjectDependancy<TimeSlotId: OrdId, StudentId: OrdId> {
    TimeSlot(TimeSlotId),
    Student(StudentId),
}

#[derive(Error, Debug)]
pub enum DataStatusWithId<Id: OrdId> {
    Ok,
    BadCrossId(Id),
}

#[derive(Error, Debug)]
pub enum DataStatusWithId3<Id1: OrdId, Id2: OrdId, Id3: OrdId> {
    Ok,
    BadCrossId1(Id1),
    BadCrossId2(Id2),
    BadCrossId3(Id3),
}

#[derive(Error, Debug)]
pub enum DataStatusWithIdAndInvalidState<Id: OrdId> {
    Ok,
    InvalidData,
    BadCrossId(Id),
}

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::RangeInclusive;

#[trait_variant::make(Send)]
pub trait Storage: Send + Sync {
    type WeekPatternId: OrdId;
    type TeacherId: OrdId;
    type StudentId: OrdId;
    type SubjectGroupId: OrdId;
    type IncompatId: OrdId;
    type GroupListId: OrdId;
    type SubjectId: OrdId;
    type TimeSlotId: OrdId;
    type GroupingId: OrdId;
    type GroupingIncompatId: OrdId;

    type InternalError: std::fmt::Debug + std::error::Error;

    async fn general_data_set(
        &mut self,
        general_data: &GeneralData,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn general_data_get(&self) -> std::result::Result<GeneralData, Self::InternalError>;

    async fn week_patterns_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::WeekPatternId, WeekPattern>, Self::InternalError>;
    async fn week_patterns_get(
        &self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<WeekPattern, IdError<Self::InternalError, Self::WeekPatternId>>;
    async fn week_patterns_add(
        &mut self,
        pattern: &WeekPattern,
    ) -> std::result::Result<Self::WeekPatternId, Self::InternalError>;
    async unsafe fn week_patterns_remove_unchecked(
        &mut self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn week_patterns_update(
        &mut self,
        index: Self::WeekPatternId,
        pattern: &WeekPattern,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::WeekPatternId>>;
    async fn week_patterns_check_can_remove(
        &mut self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<
        Vec<WeekPatternDependancy<Self::IncompatId, Self::TimeSlotId>>,
        IdError<Self::InternalError, Self::WeekPatternId>,
    > {
        async move {
            let week_patterns = self.week_patterns_get_all().await?;

            if !week_patterns.contains_key(&index) {
                return Err(IdError::InvalidId(index));
            }

            let mut dependancies = Vec::new();

            let incompats = self.incompats_get_all().await?;
            for (incompat_id, incompat) in incompats {
                if incompat.references_week_pattern(index) {
                    dependancies.push(WeekPatternDependancy::Incompat(incompat_id));
                }
            }

            let time_slots = self.time_slots_get_all().await?;
            for (time_slot_id, time_slot) in time_slots {
                if time_slot.week_pattern_id == index {
                    dependancies.push(WeekPatternDependancy::TimeSlot(time_slot_id));
                }
            }

            Ok(dependancies)
        }
    }
    async fn week_patterns_remove(
        &mut self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            Self::InternalError,
            Self::WeekPatternId,
            Vec<WeekPatternDependancy<Self::IncompatId, Self::TimeSlotId>>,
        >,
    > {
        async move {
            let dependancies = self
                .week_patterns_check_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.week_patterns_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn teachers_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::TeacherId, Teacher>, Self::InternalError>;
    async fn teachers_get(
        &self,
        index: Self::TeacherId,
    ) -> std::result::Result<Teacher, IdError<Self::InternalError, Self::TeacherId>>;
    async fn teachers_add(
        &mut self,
        teacher: &Teacher,
    ) -> std::result::Result<Self::TeacherId, Self::InternalError>;
    async unsafe fn teachers_remove_unchecked(
        &mut self,
        index: Self::TeacherId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn teachers_update(
        &mut self,
        index: Self::TeacherId,
        teacher: &Teacher,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>>;
    async fn teachers_check_can_remove(
        &mut self,
        index: Self::TeacherId,
    ) -> std::result::Result<Vec<Self::TimeSlotId>, IdError<Self::InternalError, Self::TeacherId>>
    {
        async move {
            let teachers = self.teachers_get_all().await?;

            if !teachers.contains_key(&index) {
                return Err(IdError::InvalidId(index));
            }

            let mut dependancies = Vec::new();

            let time_slots = self.time_slots_get_all().await?;
            for (time_slot_id, time_slot) in time_slots {
                if time_slot.teacher_id == index {
                    dependancies.push(time_slot_id);
                }
            }

            Ok(dependancies)
        }
    }
    async fn teachers_patterns_remove(
        &mut self,
        index: Self::TeacherId,
    ) -> std::result::Result<
        (),
        CheckedIdError<Self::InternalError, Self::TeacherId, Vec<Self::TimeSlotId>>,
    > {
        async move {
            let dependancies = self
                .teachers_check_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.teachers_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn students_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::StudentId, Student>, Self::InternalError>;
    async fn students_get(
        &self,
        index: Self::StudentId,
    ) -> std::result::Result<Student, IdError<Self::InternalError, Self::StudentId>>;
    async fn students_add(
        &mut self,
        student: &Student,
    ) -> std::result::Result<Self::StudentId, Self::InternalError>;
    async unsafe fn students_remove_unchecked(
        &mut self,
        index: Self::StudentId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn students_update(
        &mut self,
        index: Self::StudentId,
        student: &Student,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::StudentId>>;
    async fn students_check_can_remove(
        &mut self,
        index: Self::StudentId,
    ) -> std::result::Result<Vec<Self::GroupListId>, IdError<Self::InternalError, Self::StudentId>>
    {
        async move {
            let students = self.students_get_all().await?;

            if !students.contains_key(&index) {
                return Err(IdError::InvalidId(index));
            }

            let mut dependancies = Vec::new();

            let group_lists = self.group_lists_get_all().await?;
            for (group_list_id, group_list) in group_lists {
                if group_list.references_student(index) {
                    dependancies.push(group_list_id)
                }
            }

            Ok(dependancies)
        }
    }
    async fn students_remove(
        &mut self,
        index: Self::StudentId,
    ) -> std::result::Result<
        (),
        CheckedIdError<Self::InternalError, Self::StudentId, Vec<Self::GroupListId>>,
    > {
        async move {
            let dependancies = self
                .students_check_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.students_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn subject_groups_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::SubjectGroupId, SubjectGroup>, Self::InternalError>;
    async fn subject_groups_get(
        &self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<SubjectGroup, IdError<Self::InternalError, Self::SubjectGroupId>>;
    async fn subject_groups_add(
        &mut self,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<Self::SubjectGroupId, Self::InternalError>;
    async unsafe fn subject_groups_remove_unchecked(
        &mut self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn subject_groups_update(
        &mut self,
        index: Self::SubjectGroupId,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>;
    async fn subject_groups_can_remove(
        &mut self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<
        Vec<SubjectGroupDependancy<Self::SubjectId, Self::StudentId>>,
        IdError<Self::InternalError, Self::SubjectGroupId>,
    > {
        async move {
            let subject_groups = self.subject_groups_get_all().await?;

            if !subject_groups.contains_key(&index) {
                return Err(IdError::InvalidId(index));
            }

            let mut dependancies = Vec::new();

            let subjects = self.subjects_get_all().await?;
            for (subject_id, subject) in subjects {
                if subject.subject_group_id == index {
                    dependancies.push(SubjectGroupDependancy::Subject(subject_id));
                }
            }

            let students = self.students_get_all().await?;
            for (student_id, _student) in students {
                let subject_for_student = self.subject_group_for_student_get(student_id, index)
                    .await
                    .map_err(
                        |e| match e {
                            Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                            Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                            Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was tested valid a few instructions ago", id2),
                        }
                    )?;
                if subject_for_student.is_some() {
                    dependancies.push(SubjectGroupDependancy::Student(student_id));
                }
            }

            Ok(dependancies)
        }
    }
    async fn subject_groups_remove(
        &mut self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            Self::InternalError,
            Self::SubjectGroupId,
            Vec<SubjectGroupDependancy<Self::SubjectId, Self::StudentId>>,
        >,
    > {
        async move {
            let dependancies = self
                .subject_groups_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.subject_groups_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::IncompatId, Incompat<Self::WeekPatternId>>,
        Self::InternalError,
    >;
    async fn incompats_get(
        &self,
        index: Self::IncompatId,
    ) -> std::result::Result<
        Incompat<Self::WeekPatternId>,
        IdError<Self::InternalError, Self::IncompatId>,
    >;
    async unsafe fn incompats_add_unchecked(
        &mut self,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<Self::IncompatId, Self::InternalError>;
    async unsafe fn incompats_remove_unchecked(
        &mut self,
        index: Self::IncompatId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn incompats_update_unchecked(
        &mut self,
        index: Self::IncompatId,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn incompats_check_id(
        &self,
        index: Self::IncompatId,
    ) -> std::result::Result<bool, Self::InternalError> {
        async move {
            let incompats = self.incompats_get_all().await?;

            Ok(incompats.contains_key(&index))
        }
    }
    async fn incompats_check_data(
        &self,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<DataStatusWithId<Self::WeekPatternId>, Self::InternalError> {
        async move {
            let week_patterns = self.week_patterns_get_all().await?;

            for incompat_group in &incompat.groups {
                for incompat_slot in &incompat_group.slots {
                    if !week_patterns.contains_key(&incompat_slot.week_pattern_id) {
                        return Ok(DataStatusWithId::BadCrossId(incompat_slot.week_pattern_id));
                    }
                }
            }

            Ok(DataStatusWithId::Ok)
        }
    }
    async fn incompats_add(
        &mut self,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<Self::IncompatId, CrossError<Self::InternalError, Self::WeekPatternId>>
    {
        async move {
            let data_status = self.incompats_check_data(incompat).await?;
            match data_status {
                DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
                DataStatusWithId::Ok => {
                    let id = unsafe { self.incompats_add_unchecked(incompat) }.await?;
                    Ok(id)
                }
            }
        }
    }
    async fn incompats_update(
        &mut self,
        index: Self::IncompatId,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<
        (),
        CrossIdError<Self::InternalError, Self::IncompatId, Self::WeekPatternId>,
    > {
        async move {
            if !self.incompats_check_id(index).await? {
                return Err(CrossIdError::InvalidId(index));
            }

            let data_status = self.incompats_check_data(incompat).await?;
            match data_status {
                DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
                DataStatusWithId::Ok => {
                    unsafe { self.incompats_update_unchecked(index, incompat) }.await?;
                    Ok(())
                }
            }
        }
    }
    async fn incompats_can_remove(
        &mut self,
        index: Self::IncompatId,
    ) -> std::result::Result<
        Vec<IncompatDependancy<Self::SubjectId, Self::StudentId>>,
        IdError<Self::InternalError, Self::IncompatId>,
    > {
        async move {
            if !self.incompats_check_id(index).await? {
                return Err(IdError::InvalidId(index));
            }

            let mut dependancies = Vec::new();

            let subjects = self.subjects_get_all().await?;
            for (subject_id, subject) in subjects {
                if subject.incompat_id == Some(index) {
                    dependancies.push(IncompatDependancy::Subject(subject_id));
                }
            }

            let students = self.students_get_all().await?;
            for (student_id, _student) in students {
                let incompat_for_student = self.incompat_for_student_get(student_id, index)
                    .await
                    .map_err(
                        |e| match e {
                            Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                            Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                            Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was tested valid a few instructions ago", id2),
                        }
                    )?;
                if incompat_for_student {
                    dependancies.push(IncompatDependancy::Student(student_id));
                }
            }

            Ok(dependancies)
        }
    }
    async fn incompats_remove(
        &mut self,
        index: Self::IncompatId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            Self::InternalError,
            Self::IncompatId,
            Vec<IncompatDependancy<Self::SubjectId, Self::StudentId>>,
        >,
    > {
        async move {
            let dependancies = self
                .incompats_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.incompats_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn group_lists_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupListId, GroupList<Self::StudentId>>,
        Self::InternalError,
    >;
    async fn group_lists_get(
        &self,
        index: Self::GroupListId,
    ) -> std::result::Result<
        GroupList<Self::StudentId>,
        IdError<Self::InternalError, Self::GroupListId>,
    >;
    async unsafe fn group_lists_add_unchecked(
        &mut self,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<Self::GroupListId, Self::InternalError>;
    async unsafe fn group_lists_remove_unchecked(
        &mut self,
        index: Self::GroupListId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn group_lists_update_unchecked(
        &mut self,
        index: Self::GroupListId,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn group_lists_check_id(
        &self,
        index: Self::GroupListId,
    ) -> std::result::Result<bool, Self::InternalError> {
        async move {
            let group_lists = self.group_lists_get_all().await?;

            Ok(group_lists.contains_key(&index))
        }
    }
    async fn group_lists_check_data(
        &self,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<DataStatusWithIdAndInvalidState<Self::StudentId>, Self::InternalError>
    {
        async move {
            let students = self.students_get_all().await?;

            for (&student_id, &group) in &group_list.students_mapping {
                if !students.contains_key(&student_id) {
                    return Ok(DataStatusWithIdAndInvalidState::BadCrossId(student_id));
                }

                if group >= group_list.groups.len() {
                    return Ok(DataStatusWithIdAndInvalidState::InvalidData);
                }
            }

            Ok(DataStatusWithIdAndInvalidState::Ok)
        }
    }
    async fn group_lists_add(
        &mut self,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<
        Self::GroupListId,
        InvalidCrossError<Self::InternalError, GroupList<Self::StudentId>, Self::StudentId>,
    > {
        async move {
            let data_status = self.group_lists_check_data(group_list).await?;
            match data_status {
                DataStatusWithIdAndInvalidState::BadCrossId(id) => {
                    Err(InvalidCrossError::InvalidCrossId(id))
                }
                DataStatusWithIdAndInvalidState::InvalidData => {
                    Err(InvalidCrossError::InvalidData(group_list.clone()))
                }
                DataStatusWithIdAndInvalidState::Ok => {
                    let id = unsafe { self.group_lists_add_unchecked(group_list) }.await?;
                    Ok(id)
                }
            }
        }
    }
    async fn group_lists_update(
        &mut self,
        index: Self::GroupListId,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<
        (),
        InvalidCrossIdError<
            Self::InternalError,
            GroupList<Self::StudentId>,
            Self::GroupListId,
            Self::StudentId,
        >,
    > {
        async move {
            if !self.group_lists_check_id(index).await? {
                return Err(InvalidCrossIdError::InvalidId(index));
            }

            let data_status = self.group_lists_check_data(group_list).await?;
            match data_status {
                DataStatusWithIdAndInvalidState::BadCrossId(id) => {
                    Err(InvalidCrossIdError::InvalidCrossId(id))
                }
                DataStatusWithIdAndInvalidState::InvalidData => {
                    Err(InvalidCrossIdError::InvalidData(group_list.clone()))
                }
                DataStatusWithIdAndInvalidState::Ok => {
                    unsafe { self.group_lists_update_unchecked(index, group_list) }.await?;
                    Ok(())
                }
            }
        }
    }
    async fn group_lists_can_remove(
        &mut self,
        index: Self::GroupListId,
    ) -> std::result::Result<Vec<Self::SubjectId>, IdError<Self::InternalError, Self::GroupListId>>
    {
        async move {
            if !self.group_lists_check_id(index).await? {
                return Err(IdError::InvalidId(index));
            }

            let mut dependancies = Vec::new();

            let subjects = self.subjects_get_all().await?;
            for (subject_id, subject) in subjects {
                if subject.group_list_id == Some(index) {
                    dependancies.push(subject_id);
                }
            }

            Ok(dependancies)
        }
    }
    async fn group_lists_remove(
        &mut self,
        index: Self::GroupListId,
    ) -> std::result::Result<
        (),
        CheckedIdError<Self::InternalError, Self::GroupListId, Vec<Self::SubjectId>>,
    > {
        async move {
            let dependancies = self
                .group_lists_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.group_lists_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn subjects_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<
            Self::SubjectId,
            Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        >,
        Self::InternalError,
    >;
    async fn subjects_get(
        &self,
        index: Self::SubjectId,
    ) -> std::result::Result<
        Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        IdError<Self::InternalError, Self::SubjectId>,
    >;
    async unsafe fn subjects_add_unchecked(
        &mut self,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<Self::SubjectId, Self::InternalError>;
    async unsafe fn subjects_remove_unchecked(
        &mut self,
        index: Self::SubjectId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn subjects_update_unchecked(
        &mut self,
        index: Self::SubjectId,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn subjects_check_id(
        &self,
        index: Self::SubjectId,
    ) -> std::result::Result<bool, Self::InternalError> {
        async move {
            let subjects = self.subjects_get_all().await?;

            Ok(subjects.contains_key(&index))
        }
    }
    async fn subjects_check_data(
        &self,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<
        DataStatusWithId3<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        Self::InternalError,
    > {
        async move {
            let subject_groups = self.subject_groups_get_all().await?;
            if !subject_groups.contains_key(&subject.subject_group_id) {
                return Ok(DataStatusWithId3::BadCrossId1(subject.subject_group_id));
            }

            if let Some(incompat_id) = subject.incompat_id {
                let incompats = self.incompats_get_all().await?;
                if !incompats.contains_key(&incompat_id) {
                    return Ok(DataStatusWithId3::BadCrossId2(incompat_id));
                }
            }

            if let Some(group_list_id) = subject.group_list_id {
                let group_lists = self.group_lists_get_all().await?;
                if !group_lists.contains_key(&group_list_id) {
                    return Ok(DataStatusWithId3::BadCrossId3(group_list_id));
                }
            }

            Ok(DataStatusWithId3::Ok)
        }
    }
    async fn subjects_add(
        &mut self,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<
        Self::SubjectId,
        Cross3Error<Self::InternalError, Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    > {
        async move {
            let data_status = self.subjects_check_data(subject).await?;
            match data_status {
                DataStatusWithId3::BadCrossId1(id1) => Err(Cross3Error::InvalidCrossId1(id1)),
                DataStatusWithId3::BadCrossId2(id2) => Err(Cross3Error::InvalidCrossId2(id2)),
                DataStatusWithId3::BadCrossId3(id3) => Err(Cross3Error::InvalidCrossId3(id3)),
                DataStatusWithId3::Ok => {
                    let id = unsafe { self.subjects_add_unchecked(subject) }.await?;
                    Ok(id)
                }
            }
        }
    }
    async fn subjects_update(
        &mut self,
        index: Self::SubjectId,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<
        (),
        Cross3IdError<
            Self::InternalError,
            Self::SubjectId,
            Self::SubjectGroupId,
            Self::IncompatId,
            Self::GroupListId,
        >,
    > {
        async move {
            if !self.subjects_check_id(index).await? {
                return Err(Cross3IdError::InvalidId(index));
            }

            let data_status = self.subjects_check_data(subject).await?;
            match data_status {
                DataStatusWithId3::BadCrossId1(id1) => Err(Cross3IdError::InvalidCrossId1(id1)),
                DataStatusWithId3::BadCrossId2(id2) => Err(Cross3IdError::InvalidCrossId2(id2)),
                DataStatusWithId3::BadCrossId3(id3) => Err(Cross3IdError::InvalidCrossId3(id3)),
                DataStatusWithId3::Ok => {
                    unsafe { self.subjects_update_unchecked(index, subject) }.await?;
                    Ok(())
                }
            }
        }
    }
    async fn subjects_can_remove(
        &mut self,
        index: Self::SubjectId,
    ) -> std::result::Result<
        Vec<SubjectDependancy<Self::TimeSlotId, Self::StudentId>>,
        IdError<Self::InternalError, Self::SubjectId>,
    > {
        async move {
            let subject = self.subjects_get(index).await?;

            let mut dependancies = Vec::new();

            let time_slots = self.time_slots_get_all().await?;
            for (time_slot_id, time_slot) in time_slots {
                if time_slot.subject_id == index {
                    dependancies.push(SubjectDependancy::TimeSlot(time_slot_id));
                }
            }

            let students = self.students_get_all().await?;
            for (student_id, _student) in students {
                let subject_group_id = subject.subject_group_id;
                let subject_group_for_student = self.subject_group_for_student_get(student_id, subject_group_id)
                    .await
                    .map_err(
                        |e| match e {
                            Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                            Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                            Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was retrieved from the database", id2),
                        }
                    )?;
                if subject_group_for_student == Some(index) {
                    dependancies.push(SubjectDependancy::Student(student_id));
                }
            }

            Ok(dependancies)
        }
    }
    async fn subjects_remove(
        &mut self,
        index: Self::SubjectId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            Self::InternalError,
            Self::SubjectId,
            Vec<SubjectDependancy<Self::TimeSlotId, Self::StudentId>>,
        >,
    > {
        async move {
            let dependancies = self
                .subjects_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.subjects_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn time_slots_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::TimeSlotId, TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>>,
        Self::InternalError,
    >;
    async fn time_slots_get(
        &self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<
        TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
        IdError<Self::InternalError, Self::TimeSlotId>,
    >;
    async unsafe fn time_slots_add_unchecked(
        &mut self,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<Self::TimeSlotId, Self::InternalError>;
    async unsafe fn time_slots_remove_unchecked(
        &mut self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn time_slots_update_unchecked(
        &mut self,
        index: Self::TimeSlotId,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn time_slots_check_id(
        &self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<bool, Self::InternalError> {
        async move {
            let time_slots = self.time_slots_get_all().await?;

            Ok(time_slots.contains_key(&index))
        }
    }
    async fn time_slots_check_data(
        &self,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<
        DataStatusWithId3<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
        Self::InternalError,
    > {
        async move {
            let subjects = self.subjects_get_all().await?;
            if !subjects.contains_key(&time_slot.subject_id) {
                return Ok(DataStatusWithId3::BadCrossId1(time_slot.subject_id));
            }

            let teachers = self.teachers_get_all().await?;
            if !teachers.contains_key(&time_slot.teacher_id) {
                return Ok(DataStatusWithId3::BadCrossId2(time_slot.teacher_id));
            }

            let week_patterns = self.week_patterns_get_all().await?;
            if !week_patterns.contains_key(&time_slot.week_pattern_id) {
                return Ok(DataStatusWithId3::BadCrossId3(time_slot.week_pattern_id));
            }

            Ok(DataStatusWithId3::Ok)
        }
    }
    async fn time_slots_add(
        &mut self,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<
        Self::TimeSlotId,
        Cross3Error<Self::InternalError, Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    > {
        async move {
            let data_status = self.time_slots_check_data(time_slot).await?;
            match data_status {
                DataStatusWithId3::BadCrossId1(id1) => Err(Cross3Error::InvalidCrossId1(id1)),
                DataStatusWithId3::BadCrossId2(id2) => Err(Cross3Error::InvalidCrossId2(id2)),
                DataStatusWithId3::BadCrossId3(id3) => Err(Cross3Error::InvalidCrossId3(id3)),
                DataStatusWithId3::Ok => {
                    let id = unsafe { self.time_slots_add_unchecked(time_slot) }.await?;
                    Ok(id)
                }
            }
        }
    }
    async fn time_slots_update(
        &mut self,
        index: Self::TimeSlotId,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<
        (),
        Cross3IdError<
            Self::InternalError,
            Self::TimeSlotId,
            Self::SubjectId,
            Self::TeacherId,
            Self::WeekPatternId,
        >,
    > {
        async move {
            if !self.time_slots_check_id(index).await? {
                return Err(Cross3IdError::InvalidId(index));
            }

            let data_status = self.time_slots_check_data(time_slot).await?;
            match data_status {
                DataStatusWithId3::BadCrossId1(id1) => Err(Cross3IdError::InvalidCrossId1(id1)),
                DataStatusWithId3::BadCrossId2(id2) => Err(Cross3IdError::InvalidCrossId2(id2)),
                DataStatusWithId3::BadCrossId3(id3) => Err(Cross3IdError::InvalidCrossId3(id3)),
                DataStatusWithId3::Ok => {
                    unsafe { self.time_slots_update_unchecked(index, time_slot) }.await?;
                    Ok(())
                }
            }
        }
    }
    async fn time_slots_can_remove(
        &mut self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<Vec<Self::GroupingId>, IdError<Self::InternalError, Self::TimeSlotId>>
    {
        async move {
            if !self.time_slots_check_id(index).await? {
                return Err(IdError::InvalidId(index));
            }

            let mut dependancies = Vec::new();

            let groupings = self.groupings_get_all().await?;
            for (grouping_id, grouping) in groupings {
                if grouping.references_time_slot(index) {
                    dependancies.push(grouping_id);
                }
            }

            Ok(dependancies)
        }
    }
    async fn time_slots_remove(
        &mut self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<
        (),
        CheckedIdError<Self::InternalError, Self::TimeSlotId, Vec<Self::GroupingId>>,
    > {
        async move {
            let dependancies = self
                .time_slots_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.time_slots_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn groupings_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupingId, Grouping<Self::TimeSlotId>>,
        Self::InternalError,
    >;
    async fn groupings_get(
        &self,
        index: Self::GroupingId,
    ) -> std::result::Result<
        Grouping<Self::TimeSlotId>,
        IdError<Self::InternalError, Self::GroupingId>,
    >;
    async unsafe fn groupings_add_unchecked(
        &mut self,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<Self::GroupingId, Self::InternalError>;
    async unsafe fn groupings_remove_unchecked(
        &mut self,
        index: Self::GroupingId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn groupings_update_unchecked(
        &mut self,
        index: Self::GroupingId,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn groupings_check_id(
        &self,
        index: Self::GroupingId,
    ) -> std::result::Result<bool, Self::InternalError> {
        async move {
            let groupings = self.groupings_get_all().await?;

            Ok(groupings.contains_key(&index))
        }
    }
    async fn groupings_check_data(
        &self,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<DataStatusWithId<Self::TimeSlotId>, Self::InternalError> {
        async move {
            let time_slots = self.time_slots_get_all().await?;
            for &slot_id in &grouping.slots {
                if !time_slots.contains_key(&slot_id) {
                    return Ok(DataStatusWithId::BadCrossId(slot_id));
                }
            }

            Ok(DataStatusWithId::Ok)
        }
    }
    async fn groupings_add(
        &mut self,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<Self::GroupingId, CrossError<Self::InternalError, Self::TimeSlotId>>
    {
        async move {
            let data_status = self.groupings_check_data(grouping).await?;
            match data_status {
                DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
                DataStatusWithId::Ok => {
                    let id = unsafe { self.groupings_add_unchecked(grouping) }.await?;
                    Ok(id)
                }
            }
        }
    }
    async fn groupings_update(
        &mut self,
        index: Self::GroupingId,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<
        (),
        CrossIdError<Self::InternalError, Self::GroupingId, Self::TimeSlotId>,
    > {
        async move {
            if !self.groupings_check_id(index).await? {
                return Err(CrossIdError::InvalidId(index));
            }

            let data_status = self.groupings_check_data(grouping).await?;
            match data_status {
                DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
                DataStatusWithId::Ok => {
                    unsafe { self.groupings_update_unchecked(index, grouping) }.await?;
                    Ok(())
                }
            }
        }
    }
    async fn groupings_can_remove(
        &mut self,
        index: Self::GroupingId,
    ) -> std::result::Result<
        Vec<Self::GroupingIncompatId>,
        IdError<Self::InternalError, Self::GroupingId>,
    > {
        async move {
            if !self.groupings_check_id(index).await? {
                return Err(IdError::InvalidId(index));
            }

            let mut dependancies = Vec::new();

            let grouping_incompats = self.grouping_incompats_get_all().await?;
            for (grouping_incompat_id, grouping_incompat) in grouping_incompats {
                if grouping_incompat.references_grouping(index) {
                    dependancies.push(grouping_incompat_id);
                }
            }

            Ok(dependancies)
        }
    }
    async fn groupings_remove(
        &mut self,
        index: Self::GroupingId,
    ) -> std::result::Result<
        (),
        CheckedIdError<Self::InternalError, Self::GroupingId, Vec<Self::GroupingIncompatId>>,
    > {
        async move {
            let dependancies = self
                .groupings_can_remove(index)
                .await
                .map_err(CheckedIdError::from_id_error)?;
            if dependancies.len() != 0 {
                return Err(CheckedIdError::CheckFailed(dependancies));
            }
            unsafe { self.groupings_remove_unchecked(index) }.await?;
            Ok(())
        }
    }

    async fn grouping_incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupingIncompatId, GroupingIncompat<Self::GroupingId>>,
        Self::InternalError,
    >;
    async fn grouping_incompats_get(
        &self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<
        GroupingIncompat<Self::GroupingId>,
        IdError<Self::InternalError, Self::GroupingIncompatId>,
    >;
    async unsafe fn grouping_incompats_add_unchecked(
        &mut self,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<Self::GroupingIncompatId, Self::InternalError>;
    async fn grouping_incompats_remove(
        &mut self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::GroupingIncompatId>>;
    async unsafe fn grouping_incompats_update_unchecked(
        &mut self,
        index: Self::GroupingIncompatId,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn grouping_incompats_check_id(
        &self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<bool, Self::InternalError> {
        async move {
            let grouping_incompats = self.grouping_incompats_get_all().await?;

            Ok(grouping_incompats.contains_key(&index))
        }
    }
    async fn grouping_incompats_check_data(
        &self,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<DataStatusWithId<Self::GroupingId>, Self::InternalError> {
        async move {
            let groupings = self.groupings_get_all().await?;

            for &grouping_id in &grouping_incompat.groupings {
                if !groupings.contains_key(&grouping_id) {
                    return Ok(DataStatusWithId::BadCrossId(grouping_id));
                }
            }

            Ok(DataStatusWithId::Ok)
        }
    }
    async fn grouping_incompats_add(
        &mut self,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<
        Self::GroupingIncompatId,
        CrossError<Self::InternalError, Self::GroupingId>,
    > {
        async move {
            let data_status = self
                .grouping_incompats_check_data(grouping_incompat)
                .await?;
            match data_status {
                DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
                DataStatusWithId::Ok => {
                    let id =
                        unsafe { self.grouping_incompats_add_unchecked(grouping_incompat) }.await?;
                    Ok(id)
                }
            }
        }
    }
    async fn grouping_incompats_update(
        &mut self,
        index: Self::GroupingIncompatId,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<
        (),
        CrossIdError<Self::InternalError, Self::GroupingIncompatId, Self::GroupingId>,
    > {
        async move {
            if !self.grouping_incompats_check_id(index).await? {
                return Err(CrossIdError::InvalidId(index));
            }

            let data_status = self
                .grouping_incompats_check_data(grouping_incompat)
                .await?;
            match data_status {
                DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
                DataStatusWithId::Ok => {
                    unsafe { self.grouping_incompats_update_unchecked(index, grouping_incompat) }
                        .await?;
                    Ok(())
                }
            }
        }
    }

    async unsafe fn subject_group_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
        subject_id: Option<Self::SubjectId>,
    ) -> std::result::Result<
        (),
        CrossId2Error<Self::InternalError, Self::StudentId, Self::SubjectGroupId, Self::SubjectId>,
    >;
    async fn subject_group_for_student_get(
        &self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
    ) -> std::result::Result<
        Option<Self::SubjectId>,
        Id2Error<Self::InternalError, Self::StudentId, Self::SubjectGroupId>,
    >;
    async unsafe fn incompat_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        incompat_id: Self::IncompatId,
        enabled: bool,
    ) -> std::result::Result<(), Id2Error<Self::InternalError, Self::StudentId, Self::IncompatId>>;
    async fn incompat_for_student_get(
        &self,
        student_id: Self::StudentId,
        incompat_id: Self::IncompatId,
    ) -> std::result::Result<bool, Id2Error<Self::InternalError, Self::StudentId, Self::IncompatId>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralData {
    interrogations_per_week: Option<std::ops::Range<u32>>,
    max_interrogations_per_day: Option<NonZeroU32>,
}

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Week(u32);

impl Week {
    pub fn new(number: u32) -> Week {
        Week(number)
    }

    pub fn get(&self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekPattern {
    pub name: String,
    pub weeks: BTreeSet<Week>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Teacher {
    pub surname: String,
    pub firstname: String,
    pub contact: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Student {
    pub surname: String,
    pub firstname: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectGroup {
    pub name: String,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotStart {
    day: crate::time::Weekday,
    time: crate::time::Time,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatSlot<WeekPatternId: OrdId> {
    week_pattern_id: WeekPatternId,
    start: SlotStart,
    duration: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatGroup<WeekPatternId: OrdId> {
    slots: BTreeSet<IncompatSlot<WeekPatternId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompat<WeekPatternId: OrdId> {
    pub name: String,
    pub max_count: usize,
    pub groups: BTreeSet<IncompatGroup<WeekPatternId>>,
}

impl<WeekPatternId: OrdId> Incompat<WeekPatternId> {
    pub fn references_week_pattern(&self, week_pattern_id: WeekPatternId) -> bool {
        for group in &self.groups {
            for slot in &group.slots {
                if slot.week_pattern_id == week_pattern_id {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub extendable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: OrdId> {
    pub name: String,
    pub groups: Vec<Group>,
    pub students_mapping: BTreeMap<StudentId, usize>,
}

impl<StudentId: OrdId> GroupList<StudentId> {
    pub fn references_student(&self, student_id: StudentId) -> bool {
        self.students_mapping.contains_key(&student_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalancingRequirements {
    pub teachers: bool,
    pub timeslots: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject<SubjectGroupId: OrdId, IncompatId: OrdId, GroupListId: OrdId> {
    pub name: String,
    pub subject_group_id: SubjectGroupId,
    pub incompat_id: Option<IncompatId>,
    pub group_list_id: Option<GroupListId>,
    pub duration: NonZeroU32,
    pub students_per_group: RangeInclusive<NonZeroUsize>,
    pub period: NonZeroU32,
    pub period_is_strict: bool,
    pub is_tutorial: bool,
    pub max_groups_per_slot: NonZeroUsize,
    pub balancing_requirements: BalancingRequirements,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeSlot<SubjectId: OrdId, TeacherId: OrdId, WeekPatternId: OrdId> {
    pub subject_id: SubjectId,
    pub teacher_id: TeacherId,
    pub start: SlotStart,
    pub week_pattern_id: WeekPatternId,
    pub room: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grouping<TimeSlotId: OrdId> {
    pub name: String,
    pub slots: BTreeSet<TimeSlotId>,
}

impl<TimeSlotId: OrdId> Grouping<TimeSlotId> {
    pub fn references_time_slot(&self, time_slot_id: TimeSlotId) -> bool {
        self.slots.contains(&time_slot_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupingIncompat<GroupingId: OrdId> {
    pub max_count: NonZeroUsize,
    pub groupings: BTreeSet<GroupingId>,
}

impl<GroupingId: OrdId> GroupingIncompat<GroupingId> {
    pub fn references_grouping(&self, grouping_id: GroupingId) -> bool {
        self.groupings.contains(&grouping_id)
    }
}
