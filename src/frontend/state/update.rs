use self::backend::{IncompatDependancy, SubjectGroupDependancy};

use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Error)]
pub enum UpdateError<IntError: std::error::Error> {
    #[error("Error in storage backend: {0:?}")]
    Internal(#[from] IntError),
    #[error("Cannot set week_count: some week_patterns must be truncated")]
    WeekPatternsNeedTruncating(Vec<WeekPatternHandle>),
    #[error("Cannot set interrogations_per_week range: the range must be non-empty")]
    InterrogationsPerWeekRangeIsEmpty,
    #[error("Cannot add week pattern: it references weeks beyond week_count")]
    WeekNumberTooBig(u32),
    #[error("Cannot remove week pattern: it is referenced by the database")]
    WeekPatternDependanciesRemaining(
        Vec<backend::WeekPatternDependancy<IncompatHandle, TimeSlotHandle>>,
    ),
    #[error("Week pattern corresponding to handle {0:?} was previously removed")]
    WeekPatternRemoved(WeekPatternHandle),
    #[error("Teacher corresponding to handle {0:?} was previously removed")]
    TeacherRemoved(TeacherHandle),
    #[error("Cannot remove teacher: it is referenced by the database")]
    TeacherDependanciesRemaining(Vec<backend::TeacherDependancy<TimeSlotHandle, ColloscopeHandle>>),
    #[error("Student corresponding to handle {0:?} was previously removed")]
    StudentRemoved(StudentHandle),
    #[error("Cannot remove student: it is referenced by the database")]
    StudentDependanciesRemaining(
        Vec<backend::StudentDependancy<GroupListHandle, ColloscopeHandle>>,
    ),
    #[error("Subject group corresponding to handle {0:?} was previously removed")]
    SubjectGroupRemoved(SubjectGroupHandle),
    #[error("Cannot remove subject group: it is referenced by the database")]
    SubjectGroupDependanciesRemaining(
        Vec<backend::SubjectGroupDependancy<SubjectHandle, StudentHandle>>,
    ),
    #[error("Incompat corresponding to handle {0:?} was previously removed")]
    IncompatRemoved(IncompatHandle),
    #[error("Incompat references a bad week pattern (probably removed) of id {0:?}")]
    IncompatBadWeekPattern(WeekPatternHandle),
    #[error("Cannot remove incompat: it is referenced by the database")]
    IncompatDependanciesRemaining(Vec<backend::IncompatDependancy<SubjectHandle, StudentHandle>>),
    #[error("Group list corresponding to handle {0:?} was previously removed")]
    GroupListRemoved(GroupListHandle),
    #[error("Group list has inconsistent student mapping")]
    GroupListWithInconsistentStudentMapping,
    #[error("Group list references a bad student (probably removed) of id {0:?}")]
    GroupListBadStudent(StudentHandle),
    #[error("Cannot remove incompat: it is referenced by the database")]
    GroupListDependanciesRemaining(Vec<SubjectHandle>),
    #[error("Subject corresponding to handle {0:?} was previously removed")]
    SubjectRemoved(SubjectHandle),
    #[error("Subject references a bad subject group (probably removed) of id {0:?}")]
    SubjectBadSubjectGroup(SubjectGroupHandle),
    #[error("Subject references a bad incompat (probably removed) of id {0:?}")]
    SubjectBadIncompat(IncompatHandle),
    #[error("Subject references a bad group list (probably removed) of id {0:?}")]
    SubjectBadGroupList(GroupListHandle),
    #[error("Cannot remove subject: it is referenced by the database")]
    SubjectDependanciesRemaining(
        Vec<backend::SubjectDependancy<TimeSlotHandle, StudentHandle, ColloscopeHandle>>,
    ),
    #[error("Cannot update subject: a student is still registerd")]
    SubjectWithStudentRegistered(StudentHandle),
    #[error("Time slot corresponding to handle {0:?} was previously removed")]
    TimeSlotRemoved(TimeSlotHandle),
    #[error("Time slot references a bad subject (probably removed) of id {0:?}")]
    TimeSlotBadSubject(SubjectHandle),
    #[error("Time slot references a bad teacher (probably removed) of id {0:?}")]
    TimeSlotBadTeacher(TeacherHandle),
    #[error("Time slot references a bad week pattern (probably removed) of id {0:?}")]
    TimeSlotBadWeekPattern(WeekPatternHandle),
    #[error("Cannot remove time slot: it is referenced by the database")]
    TimeSlotDependanciesRemaining(Vec<GroupingHandle>),
    #[error("Grouping corresponding to handle {0:?} was previously removed")]
    GroupingRemoved(GroupingHandle),
    #[error("Grouping references a bad time slot (probably removed) of id {0:?}")]
    GroupingBadTimeSlot(TimeSlotHandle),
    #[error("Cannot remove grouping: it is referenced by the database")]
    GroupingDependanciesRemaining(Vec<GroupingIncompatHandle>),
    #[error("Grouping ncompat corresponding to handle {0:?} was previously removed")]
    GroupingIncompatRemoved(GroupingIncompatHandle),
    #[error("Grouping incompat references a bad grouping (probably removed) of id {0:?}")]
    GroupingIncompatBadGrouping(GroupingHandle),
    #[error("Cannot register student: Subject {1:?} is not in subject group {0:?}")]
    RegisterStudentBadSubject(SubjectGroupHandle, SubjectHandle),
    #[error("Colloscope corresponding to handle {0:?} was previously removed")]
    ColloscopeRemoved(ColloscopeHandle),
    #[error("Colloscope references a bad teacher (probably removed) of id {0:?}")]
    ColloscopeBadTeacher(TeacherHandle),
    #[error("Colloscope references a bad subject (probably removed) of id {0:?}")]
    ColloscopeBadSubject(SubjectHandle),
    #[error("Colloscope references a bad student (probably removed) of id {0:?}")]
    ColloscopeBadStudent(StudentHandle),
}

#[derive(Debug, Error)]
pub enum RevError<IntError: std::error::Error> {
    #[error("Error in storage backend: {0:?}")]
    Internal(#[from] IntError),
    #[error("Week pattern corresponding to handle {0:?} was previously removed")]
    WeekPatternRemoved(WeekPatternHandle),
    #[error("Teacher corresponding to handle {0:?} was previously removed")]
    TeacherRemoved(TeacherHandle),
    #[error("Student corresponding to handle {0:?} was previously removed")]
    StudentRemoved(StudentHandle),
    #[error("Subject group corresponding to handle {0:?} was previously removed")]
    SubjectGroupRemoved(SubjectGroupHandle),
    #[error("Incompat corresponding to handle {0:?} was previously removed")]
    IncompatRemoved(IncompatHandle),
    #[error("Group list corresponding to handle {0:?} was previously removed")]
    GroupListRemoved(GroupListHandle),
    #[error("Subject corresponding to handle {0:?} was previously removed")]
    SubjectRemoved(SubjectHandle),
    #[error("Time slot corresponding to handle {0:?} was previously removed")]
    TimeSlotRemoved(TimeSlotHandle),
    #[error("Grouping corresponding to handle {0:?} was previously removed")]
    GroupingRemoved(GroupingHandle),
    #[error("Grouping incompat corresponding to handle {0:?} was previously removed")]
    GroupingIncompatRemoved(GroupingIncompatHandle),
    #[error("Colloscope corresponding to handle {0:?} was previously removed")]
    ColloscopeRemoved(ColloscopeHandle),
}

impl<IntError: std::error::Error> From<RevError<IntError>> for UpdateError<IntError> {
    fn from(value: RevError<IntError>) -> Self {
        match value {
            RevError::Internal(int_error) => UpdateError::Internal(int_error),
            RevError::WeekPatternRemoved(handle) => UpdateError::WeekPatternRemoved(handle),
            RevError::TeacherRemoved(handle) => UpdateError::TeacherRemoved(handle),
            RevError::StudentRemoved(handle) => UpdateError::StudentRemoved(handle),
            RevError::SubjectGroupRemoved(handle) => UpdateError::SubjectGroupRemoved(handle),
            RevError::IncompatRemoved(handle) => UpdateError::IncompatRemoved(handle),
            RevError::GroupListRemoved(handle) => UpdateError::GroupListRemoved(handle),
            RevError::SubjectRemoved(handle) => UpdateError::SubjectRemoved(handle),
            RevError::TimeSlotRemoved(handle) => UpdateError::TimeSlotRemoved(handle),
            RevError::GroupingRemoved(handle) => UpdateError::GroupingRemoved(handle),
            RevError::GroupingIncompatRemoved(handle) => {
                UpdateError::GroupingIncompatRemoved(handle)
            }
            RevError::ColloscopeRemoved(handle) => UpdateError::ColloscopeRemoved(handle),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ReturnHandle {
    NoHandle,
    WeekPattern(WeekPatternHandle),
    Teacher(TeacherHandle),
    Student(StudentHandle),
    SubjectGroup(SubjectGroupHandle),
    Incompat(IncompatHandle),
    GroupList(GroupListHandle),
    Subject(SubjectHandle),
    TimeSlot(TimeSlotHandle),
    Grouping(GroupingHandle),
    GroupingIncompat(GroupingIncompatHandle),
    Colloscope(ColloscopeHandle),
}

use backend::{IdError, WeekPatternDependancy, WeekPatternError};

#[trait_variant::make(Send)]
pub trait Manager: ManagerInternal {
    type InternalStorage: backend::Storage;

    fn get_logic(&self) -> &backend::Logic<Self::InternalStorage>;

    async fn general_data_get(
        &self,
    ) -> Result<backend::GeneralData, <Self::InternalStorage as backend::Storage>::InternalError>;
    async fn week_patterns_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<WeekPatternHandle, backend::WeekPattern>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn week_patterns_get(
        &self,
        handle: WeekPatternHandle,
    ) -> Result<
        backend::WeekPattern,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, WeekPatternHandle>,
    >;
    async fn week_patterns_check_can_remove(
        &mut self,
        handle: WeekPatternHandle,
    ) -> Result<
        Vec<WeekPatternDependancy<IncompatHandle, TimeSlotHandle>>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, WeekPatternHandle>,
    >;
    async fn week_patterns_check_data(
        &self,
        pattern: &backend::WeekPattern,
    ) -> std::result::Result<
        (),
        WeekPatternError<<Self::InternalStorage as backend::Storage>::InternalError>,
    >;
    async fn teachers_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<TeacherHandle, backend::Teacher>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn teachers_get(
        &self,
        handle: TeacherHandle,
    ) -> Result<
        backend::Teacher,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, TeacherHandle>,
    >;
    async fn teachers_check_can_remove(
        &mut self,
        handle: TeacherHandle,
    ) -> Result<
        Vec<backend::TeacherDependancy<TimeSlotHandle, ColloscopeHandle>>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, TeacherHandle>,
    >;
    async fn students_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<StudentHandle, backend::Student>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn students_get(
        &self,
        handle: StudentHandle,
    ) -> Result<
        backend::Student,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, StudentHandle>,
    >;
    async fn students_check_can_remove(
        &mut self,
        handle: StudentHandle,
    ) -> Result<
        Vec<backend::StudentDependancy<GroupListHandle, ColloscopeHandle>>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, StudentHandle>,
    >;
    async fn subject_groups_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<SubjectGroupHandle, backend::SubjectGroup>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn subject_groups_get(
        &self,
        handle: SubjectGroupHandle,
    ) -> Result<
        backend::SubjectGroup,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, SubjectGroupHandle>,
    >;
    async fn subject_groups_check_can_remove(
        &mut self,
        handle: SubjectGroupHandle,
    ) -> Result<
        Vec<SubjectGroupDependancy<SubjectHandle, StudentHandle>>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, SubjectGroupHandle>,
    >;
    async fn incompats_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<IncompatHandle, backend::Incompat<WeekPatternHandle>>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn incompats_get(
        &mut self,
        handle: IncompatHandle,
    ) -> Result<
        backend::Incompat<WeekPatternHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, IncompatHandle>,
    >;
    async fn incompats_check_data(
        &self,
        incompat: &backend::Incompat<WeekPatternHandle>,
    ) -> Result<
        backend::DataStatusWithId<WeekPatternHandle>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn incompats_check_can_remove(
        &mut self,
        handle: IncompatHandle,
    ) -> Result<
        Vec<backend::IncompatDependancy<SubjectHandle, StudentHandle>>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, IncompatHandle>,
    >;
    async fn group_lists_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<GroupListHandle, backend::GroupList<StudentHandle>>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn group_lists_get(
        &mut self,
        handle: GroupListHandle,
    ) -> Result<
        backend::GroupList<StudentHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, GroupListHandle>,
    >;
    async fn group_lists_check_data(
        &self,
        group_list: &backend::GroupList<StudentHandle>,
    ) -> Result<
        backend::DataStatusWithIdAndInvalidState<StudentHandle>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn group_lists_check_can_remove(
        &mut self,
        handle: GroupListHandle,
    ) -> Result<
        Vec<SubjectHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, GroupListHandle>,
    >;
    async fn subjects_get_all(
        &mut self,
    ) -> std::result::Result<
        BTreeMap<
            SubjectHandle,
            backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
        >,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn subjects_get(
        &mut self,
        handle: SubjectHandle,
    ) -> Result<
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, SubjectHandle>,
    >;
    async fn subjects_check_data(
        &self,
        subject: &backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    ) -> Result<
        backend::DataStatusWithId3<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn subjects_check_can_remove(
        &mut self,
        handle: SubjectHandle,
    ) -> Result<
        Vec<backend::SubjectDependancy<TimeSlotHandle, StudentHandle, ColloscopeHandle>>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, SubjectHandle>,
    >;
    async fn time_slots_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<
            TimeSlotHandle,
            backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
        >,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn time_slots_get(
        &mut self,
        handle: TimeSlotHandle,
    ) -> Result<
        backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, TimeSlotHandle>,
    >;
    async fn time_slots_check_data(
        &self,
        time_slot: &backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
    ) -> Result<
        backend::DataStatusWithId3<SubjectHandle, TeacherHandle, WeekPatternHandle>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn time_slots_check_can_remove(
        &mut self,
        handle: TimeSlotHandle,
    ) -> Result<
        Vec<GroupingHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, TimeSlotHandle>,
    >;
    async fn groupings_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<GroupingHandle, backend::Grouping<TimeSlotHandle>>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn groupings_get(
        &mut self,
        handle: GroupingHandle,
    ) -> Result<
        backend::Grouping<TimeSlotHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, GroupingHandle>,
    >;
    async fn groupings_check_data(
        &self,
        grouping: &backend::Grouping<TimeSlotHandle>,
    ) -> Result<
        backend::DataStatusWithId<TimeSlotHandle>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn groupings_check_can_remove(
        &mut self,
        handle: GroupingHandle,
    ) -> Result<
        Vec<GroupingIncompatHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, GroupingHandle>,
    >;
    async fn grouping_incompats_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<GroupingIncompatHandle, backend::GroupingIncompat<GroupingHandle>>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn grouping_incompats_get(
        &mut self,
        handle: GroupingIncompatHandle,
    ) -> Result<
        backend::GroupingIncompat<GroupingHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, GroupingIncompatHandle>,
    >;
    async fn grouping_incompats_check_data(
        &self,
        grouping_incompat: &backend::GroupingIncompat<GroupingHandle>,
    ) -> Result<
        backend::DataStatusWithId<GroupingHandle>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn subject_group_for_student_get(
        &mut self,
        student_handle: StudentHandle,
        subject_group_handle: SubjectGroupHandle,
    ) -> Result<
        Option<SubjectHandle>,
        backend::Id2Error<
            <Self::InternalStorage as backend::Storage>::InternalError,
            StudentHandle,
            SubjectGroupHandle,
        >,
    >;
    async fn incompat_for_student_get(
        &self,
        student_handle: StudentHandle,
        incompat_handle: IncompatHandle,
    ) -> Result<
        bool,
        backend::Id2Error<
            <Self::InternalStorage as backend::Storage>::InternalError,
            StudentHandle,
            IncompatHandle,
        >,
    >;
    async fn colloscopes_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<
            ColloscopeHandle,
            backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
        >,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;
    async fn colloscopes_get(
        &mut self,
        handle: ColloscopeHandle,
    ) -> Result<
        backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
        IdError<<Self::InternalStorage as backend::Storage>::InternalError, ColloscopeHandle>,
    >;
    async fn colloscopes_check_data(
        &self,
        colloscope: &backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
    ) -> Result<
        backend::DataStatusWithId3<TeacherHandle, SubjectHandle, StudentHandle>,
        <Self::InternalStorage as backend::Storage>::InternalError,
    >;

    async fn apply(
        &mut self,
        op: Operation,
    ) -> Result<ReturnHandle, UpdateError<<Self::InternalStorage as backend::Storage>::InternalError>>;
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;
    async fn undo(
        &mut self,
    ) -> Result<(), UndoError<<Self::Storage as backend::Storage>::InternalError>>;
    async fn redo(
        &mut self,
    ) -> Result<(), RedoError<<Self::Storage as backend::Storage>::InternalError>>;
    fn get_aggregated_history(&self) -> AggregatedOperations;
}

impl<T: ManagerInternal> Manager for T {
    type InternalStorage = T::Storage;

    fn get_logic(&self) -> &backend::Logic<T::Storage> {
        self.get_backend_logic()
    }

    fn general_data_get(
        &self,
    ) -> impl core::future::Future<
        Output = Result<backend::GeneralData, <Self::Storage as backend::Storage>::InternalError>,
    > + Send {
        async { self.get_backend_logic().general_data_get().await }
    }

    fn week_patterns_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<WeekPatternHandle, backend::WeekPattern>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let week_patterns_backend = self.get_backend_logic().week_patterns_get_all().await?;

            let handle_manager = &mut self.get_handle_managers_mut().week_patterns;
            let week_patterns = week_patterns_backend
                .into_iter()
                .map(|(id, week_pattern)| {
                    let handle = handle_manager.get_handle(id);
                    (handle, week_pattern)
                })
                .collect();

            Ok(week_patterns)
        }
    }

    fn week_patterns_get(
        &self,
        handle: WeekPatternHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::WeekPattern,
            IdError<<Self::Storage as backend::Storage>::InternalError, WeekPatternHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().week_patterns;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let week_pattern = self
                .get_backend_logic()
                .week_patterns_get(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            Ok(week_pattern)
        }
    }

    fn week_patterns_check_can_remove(
        &mut self,
        handle: WeekPatternHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<WeekPatternDependancy<IncompatHandle, TimeSlotHandle>>,
            IdError<<Self::Storage as backend::Storage>::InternalError, WeekPatternHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().week_patterns;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let week_pattern_deps_backend = self
                .get_backend_logic()
                .week_patterns_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let incompat_handle_manager = &mut handle_managers.incompats;
            let time_slot_handle_manager = &mut handle_managers.time_slots;

            let week_pattern_deps = week_pattern_deps_backend
                .into_iter()
                .map(|dep| match dep {
                    WeekPatternDependancy::Incompat(id) => {
                        WeekPatternDependancy::Incompat(incompat_handle_manager.get_handle(id))
                    }
                    WeekPatternDependancy::TimeSlot(id) => {
                        WeekPatternDependancy::TimeSlot(time_slot_handle_manager.get_handle(id))
                    }
                })
                .collect();

            Ok(week_pattern_deps)
        }
    }

    fn week_patterns_check_data(
        &self,
        pattern: &backend::WeekPattern,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            (),
            WeekPatternError<<Self::Storage as backend::Storage>::InternalError>,
        >,
    > + Send {
        async {
            self.get_backend_logic()
                .week_patterns_check_data(pattern)
                .await
        }
    }

    fn teachers_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<TeacherHandle, backend::Teacher>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let teachers_backend = self.get_backend_logic().teachers_get_all().await?;

            let handle_manager = &mut self.get_handle_managers_mut().teachers;
            let teachers = teachers_backend
                .into_iter()
                .map(|(id, teacher)| {
                    let handle = handle_manager.get_handle(id);
                    (handle, teacher)
                })
                .collect();

            Ok(teachers)
        }
    }

    fn teachers_get(
        &self,
        handle: TeacherHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::Teacher,
            IdError<<Self::Storage as backend::Storage>::InternalError, TeacherHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().teachers;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let teacher =
                self.get_backend_logic()
                    .teachers_get(index)
                    .await
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => IdError::InternalError(int_err),
                        IdError::InvalidId(_id) => IdError::InvalidId(handle),
                    })?;

            Ok(teacher)
        }
    }

    fn teachers_check_can_remove(
        &mut self,
        handle: TeacherHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<backend::TeacherDependancy<TimeSlotHandle, ColloscopeHandle>>,
            IdError<<Self::Storage as backend::Storage>::InternalError, TeacherHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().teachers;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let teacher_deps_backend = self
                .get_backend_logic()
                .teachers_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let time_slot_handle_manager = &mut handle_managers.time_slots;
            let colloscope_handle_manager = &mut handle_managers.colloscopes;

            let teacher_deps = teacher_deps_backend
                .into_iter()
                .map(|dep| match dep {
                    backend::TeacherDependancy::TimeSlot(id) => {
                        backend::TeacherDependancy::TimeSlot(
                            time_slot_handle_manager.get_handle(id),
                        )
                    }
                    backend::TeacherDependancy::Colloscope(id) => {
                        backend::TeacherDependancy::Colloscope(
                            colloscope_handle_manager.get_handle(id),
                        )
                    }
                })
                .collect();

            Ok(teacher_deps)
        }
    }

    fn students_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<StudentHandle, backend::Student>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let students_backend = self.get_backend_logic().students_get_all().await?;

            let handle_manager = &mut self.get_handle_managers_mut().students;
            let students = students_backend
                .into_iter()
                .map(|(id, student)| {
                    let handle = handle_manager.get_handle(id);
                    (handle, student)
                })
                .collect();

            Ok(students)
        }
    }

    fn students_get(
        &self,
        handle: StudentHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::Student,
            IdError<<Self::Storage as backend::Storage>::InternalError, StudentHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().students;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let student =
                self.get_backend_logic()
                    .students_get(index)
                    .await
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => IdError::InternalError(int_err),
                        IdError::InvalidId(_id) => IdError::InvalidId(handle),
                    })?;

            Ok(student)
        }
    }

    fn students_check_can_remove(
        &mut self,
        handle: StudentHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<backend::StudentDependancy<GroupListHandle, ColloscopeHandle>>,
            IdError<<Self::Storage as backend::Storage>::InternalError, StudentHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().students;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let student_deps_backend = self
                .get_backend_logic()
                .students_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let group_list_handle_manager = &mut handle_managers.group_lists;
            let colloscope_handle_manager = &mut handle_managers.colloscopes;

            let student_deps = student_deps_backend
                .into_iter()
                .map(|dep| match dep {
                    backend::StudentDependancy::GroupList(id) => {
                        backend::StudentDependancy::GroupList(
                            group_list_handle_manager.get_handle(id),
                        )
                    }
                    backend::StudentDependancy::Colloscope(id) => {
                        backend::StudentDependancy::Colloscope(
                            colloscope_handle_manager.get_handle(id),
                        )
                    }
                })
                .collect();

            Ok(student_deps)
        }
    }

    fn subject_groups_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<SubjectGroupHandle, backend::SubjectGroup>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let subject_groups_backend = self.get_backend_logic().subject_groups_get_all().await?;

            let handle_manager = &mut self.get_handle_managers_mut().subject_groups;
            let subject_groups = subject_groups_backend
                .into_iter()
                .map(|(id, subject_group)| {
                    let handle = handle_manager.get_handle(id);
                    (handle, subject_group)
                })
                .collect();

            Ok(subject_groups)
        }
    }

    fn subject_groups_get(
        &self,
        handle: SubjectGroupHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::SubjectGroup,
            IdError<<Self::Storage as backend::Storage>::InternalError, SubjectGroupHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().subject_groups;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let subject_group = self
                .get_backend_logic()
                .subject_groups_get(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            Ok(subject_group)
        }
    }

    fn subject_groups_check_can_remove(
        &mut self,
        handle: SubjectGroupHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<SubjectGroupDependancy<SubjectHandle, StudentHandle>>,
            IdError<<Self::Storage as backend::Storage>::InternalError, SubjectGroupHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().subject_groups;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let subject_group_deps_backend = self
                .get_backend_logic()
                .subject_groups_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let subject_handle_manager = &mut handle_managers.subjects;
            let student_handle_manager = &mut handle_managers.students;

            let subject_group_deps = subject_group_deps_backend
                .into_iter()
                .map(|dep| match dep {
                    SubjectGroupDependancy::Student(id) => {
                        SubjectGroupDependancy::Student(student_handle_manager.get_handle(id))
                    }
                    SubjectGroupDependancy::Subject(id) => {
                        SubjectGroupDependancy::Subject(subject_handle_manager.get_handle(id))
                    }
                })
                .collect();

            Ok(subject_group_deps)
        }
    }

    fn incompats_get(
        &mut self,
        handle: IncompatHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::Incompat<WeekPatternHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, IncompatHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().incompats;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let incompat = self
                .get_backend_logic()
                .incompats_get(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            Ok(private::convert_incompat_to_handles(
                incompat,
                self.get_handle_managers_mut(),
            ))
        }
    }

    fn incompats_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<IncompatHandle, backend::Incompat<WeekPatternHandle>>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let incompats_backend = self.get_backend_logic().incompats_get_all().await?;

            let incompats = incompats_backend
                .into_iter()
                .map(|(id, incompat)| {
                    let handle = self.get_handle_managers_mut().incompats.get_handle(id);
                    let incompat = private::convert_incompat_to_handles(
                        incompat,
                        self.get_handle_managers_mut(),
                    );
                    (handle, incompat)
                })
                .collect();

            Ok(incompats)
        }
    }

    fn incompats_check_data(
        &self,
        incompat: &backend::Incompat<WeekPatternHandle>,
    ) -> impl core::future::Future<
        Output = Result<
            backend::DataStatusWithId<WeekPatternHandle>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let incompat_backend = match private::convert_incompat_from_handles(
                incompat.clone(),
                self.get_handle_managers(),
            ) {
                Ok(val) => val,
                Err(status) => return Ok(status),
            };

            let status_backend = self
                .get_backend_logic()
                .incompats_check_data(&incompat_backend)
                .await?;

            let status = match status_backend {
                backend::DataStatusWithId::BadCrossId(_id) => {
                    panic!("WeekPatternId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId::Ok => backend::DataStatusWithId::Ok,
            };

            Ok(status)
        }
    }

    fn incompats_check_can_remove(
        &mut self,
        handle: IncompatHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<backend::IncompatDependancy<SubjectHandle, StudentHandle>>,
            IdError<<Self::Storage as backend::Storage>::InternalError, IncompatHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().incompats;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let incompat_deps_backend = self
                .get_backend_logic()
                .incompats_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let subject_handle_manager = &mut handle_managers.subjects;
            let student_handle_manager = &mut handle_managers.students;

            let incompat_deps = incompat_deps_backend
                .into_iter()
                .map(|dep| match dep {
                    IncompatDependancy::Student(id) => {
                        IncompatDependancy::Student(student_handle_manager.get_handle(id))
                    }
                    IncompatDependancy::Subject(id) => {
                        IncompatDependancy::Subject(subject_handle_manager.get_handle(id))
                    }
                })
                .collect();

            Ok(incompat_deps)
        }
    }

    fn group_lists_get(
        &mut self,
        handle: GroupListHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::GroupList<StudentHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, GroupListHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().group_lists;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let group_list = self
                .get_backend_logic()
                .group_lists_get(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            Ok(private::convert_group_list_to_handles(
                group_list,
                self.get_handle_managers_mut(),
            ))
        }
    }

    fn group_lists_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<GroupListHandle, backend::GroupList<StudentHandle>>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let group_lists_backend = self.get_backend_logic().group_lists_get_all().await?;

            let group_lists = group_lists_backend
                .into_iter()
                .map(|(id, group_list)| {
                    let handle = self.get_handle_managers_mut().group_lists.get_handle(id);
                    let group_list = private::convert_group_list_to_handles(
                        group_list,
                        self.get_handle_managers_mut(),
                    );
                    (handle, group_list)
                })
                .collect();

            Ok(group_lists)
        }
    }

    fn group_lists_check_data(
        &self,
        group_list: &backend::GroupList<StudentHandle>,
    ) -> impl core::future::Future<
        Output = Result<
            backend::DataStatusWithIdAndInvalidState<StudentHandle>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let group_list_backend = match private::convert_group_list_from_handles(
                group_list.clone(),
                self.get_handle_managers(),
            ) {
                Ok(val) => val,
                Err(status) => return Ok(status),
            };

            let status_backend = self
                .get_backend_logic()
                .group_lists_check_data(&group_list_backend)
                .await?;

            let status = match status_backend {
                backend::DataStatusWithIdAndInvalidState::BadCrossId(_id) => {
                    panic!("StudentId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithIdAndInvalidState::InvalidData => {
                    backend::DataStatusWithIdAndInvalidState::Ok
                }
                backend::DataStatusWithIdAndInvalidState::Ok => {
                    backend::DataStatusWithIdAndInvalidState::Ok
                }
            };

            Ok(status)
        }
    }

    fn group_lists_check_can_remove(
        &mut self,
        handle: GroupListHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<SubjectHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, GroupListHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().group_lists;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let group_list_deps_backend = self
                .get_backend_logic()
                .group_lists_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let subject_handle_manager = &mut handle_managers.subjects;

            let group_list_deps = group_list_deps_backend
                .into_iter()
                .map(|dep| subject_handle_manager.get_handle(dep))
                .collect();

            Ok(group_list_deps)
        }
    }

    fn subjects_get(
        &mut self,
        handle: SubjectHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, SubjectHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().subjects;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let subject =
                self.get_backend_logic()
                    .subjects_get(index)
                    .await
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => IdError::InternalError(int_err),
                        IdError::InvalidId(_id) => IdError::InvalidId(handle),
                    })?;

            Ok(private::convert_subject_to_handles(
                subject,
                self.get_handle_managers_mut(),
            ))
        }
    }

    fn subjects_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = std::result::Result<
            BTreeMap<
                SubjectHandle,
                backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
            >,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let subjects_backend = self.get_backend_logic().subjects_get_all().await?;

            let subjects = subjects_backend
                .into_iter()
                .map(|(id, subject)| {
                    let handle = self.get_handle_managers_mut().subjects.get_handle(id);
                    let subject = private::convert_subject_to_handles(
                        subject,
                        self.get_handle_managers_mut(),
                    );
                    (handle, subject)
                })
                .collect();

            Ok(subjects)
        }
    }

    fn subjects_check_data(
        &self,
        subject: &backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    ) -> impl core::future::Future<
        Output = Result<
            backend::DataStatusWithId3<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let subject_backend = match private::convert_subject_from_handles(
                subject.clone(),
                self.get_handle_managers(),
            ) {
                Ok(val) => val,
                Err(status) => return Ok(status),
            };

            let status_backend = self
                .get_backend_logic()
                .subjects_check_data(&subject_backend)
                .await?;

            let status = match status_backend {
                backend::DataStatusWithId3::BadCrossId1(_id) => {
                    panic!(
                        "SubjectGroupId was taken from a handle manager and thus should be valid"
                    )
                }
                backend::DataStatusWithId3::BadCrossId2(_id) => {
                    panic!("IncompatId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId3::BadCrossId3(_id) => {
                    panic!("GroupListId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId3::Ok => backend::DataStatusWithId3::Ok,
            };

            Ok(status)
        }
    }

    fn subjects_check_can_remove(
        &mut self,
        handle: SubjectHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<backend::SubjectDependancy<TimeSlotHandle, StudentHandle, ColloscopeHandle>>,
            IdError<<Self::Storage as backend::Storage>::InternalError, SubjectHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().subjects;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let subject_deps_backend = self
                .get_backend_logic()
                .subjects_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let time_slot_handle_manager = &mut handle_managers.time_slots;
            let student_handle_manager = &mut handle_managers.students;
            let colloscope_handle_manager = &mut handle_managers.colloscopes;

            let subject_deps = subject_deps_backend
                .into_iter()
                .map(|dep| match dep {
                    backend::SubjectDependancy::TimeSlot(id) => {
                        backend::SubjectDependancy::TimeSlot(
                            time_slot_handle_manager.get_handle(id),
                        )
                    }
                    backend::SubjectDependancy::Student(id) => {
                        backend::SubjectDependancy::Student(student_handle_manager.get_handle(id))
                    }
                    backend::SubjectDependancy::Colloscope(id) => {
                        backend::SubjectDependancy::Colloscope(
                            colloscope_handle_manager.get_handle(id),
                        )
                    }
                })
                .collect();

            Ok(subject_deps)
        }
    }

    fn time_slots_get(
        &mut self,
        handle: TimeSlotHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, TimeSlotHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().time_slots;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let time_slot = self
                .get_backend_logic()
                .time_slots_get(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            Ok(private::convert_time_slot_to_handles(
                time_slot,
                self.get_handle_managers_mut(),
            ))
        }
    }

    fn time_slots_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<
                TimeSlotHandle,
                backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
            >,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let time_slots_backend = self.get_backend_logic().time_slots_get_all().await?;

            let time_slots = time_slots_backend
                .into_iter()
                .map(|(id, time_slot)| {
                    let handle = self.get_handle_managers_mut().time_slots.get_handle(id);
                    let time_slot = private::convert_time_slot_to_handles(
                        time_slot,
                        self.get_handle_managers_mut(),
                    );
                    (handle, time_slot)
                })
                .collect();

            Ok(time_slots)
        }
    }

    fn time_slots_check_data(
        &self,
        time_slot: &backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
    ) -> impl core::future::Future<
        Output = Result<
            backend::DataStatusWithId3<SubjectHandle, TeacherHandle, WeekPatternHandle>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let time_slot_backend = match private::convert_time_slot_from_handles(
                time_slot.clone(),
                self.get_handle_managers(),
            ) {
                Ok(val) => val,
                Err(status) => return Ok(status),
            };

            let status_backend = self
                .get_backend_logic()
                .time_slots_check_data(&time_slot_backend)
                .await?;

            let status = match status_backend {
                backend::DataStatusWithId3::BadCrossId1(_id) => {
                    panic!("SubjectId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId3::BadCrossId2(_id) => {
                    panic!("TeacherId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId3::BadCrossId3(_id) => {
                    panic!("WeekPatternId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId3::Ok => backend::DataStatusWithId3::Ok,
            };

            Ok(status)
        }
    }

    fn time_slots_check_can_remove(
        &mut self,
        handle: TimeSlotHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<GroupingHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, TimeSlotHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().time_slots;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let time_slot_deps_backend = self
                .get_backend_logic()
                .time_slots_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let grouping_handle_manager = &mut handle_managers.groupings;

            let time_slot_deps = time_slot_deps_backend
                .into_iter()
                .map(|dep| grouping_handle_manager.get_handle(dep))
                .collect();

            Ok(time_slot_deps)
        }
    }

    fn groupings_get(
        &mut self,
        handle: GroupingHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::Grouping<TimeSlotHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, GroupingHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().groupings;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let grouping = self
                .get_backend_logic()
                .groupings_get(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            Ok(private::convert_grouping_to_handles(
                grouping,
                self.get_handle_managers_mut(),
            ))
        }
    }

    fn groupings_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<GroupingHandle, backend::Grouping<TimeSlotHandle>>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let groupings_backend = self.get_backend_logic().groupings_get_all().await?;

            let groupings = groupings_backend
                .into_iter()
                .map(|(id, grouping)| {
                    let handle = self.get_handle_managers_mut().groupings.get_handle(id);
                    let grouping = private::convert_grouping_to_handles(
                        grouping,
                        self.get_handle_managers_mut(),
                    );
                    (handle, grouping)
                })
                .collect();

            Ok(groupings)
        }
    }

    fn groupings_check_data(
        &self,
        grouping: &backend::Grouping<TimeSlotHandle>,
    ) -> impl core::future::Future<
        Output = Result<
            backend::DataStatusWithId<TimeSlotHandle>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let grouping_backend = match private::convert_grouping_from_handles(
                grouping.clone(),
                self.get_handle_managers(),
            ) {
                Ok(val) => val,
                Err(status) => return Ok(status),
            };

            let status_backend = self
                .get_backend_logic()
                .groupings_check_data(&grouping_backend)
                .await?;

            let status = match status_backend {
                backend::DataStatusWithId::BadCrossId(_id) => {
                    panic!("TimeSlotId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId::Ok => backend::DataStatusWithId::Ok,
            };

            Ok(status)
        }
    }

    fn groupings_check_can_remove(
        &mut self,
        handle: GroupingHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Vec<GroupingIncompatHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, GroupingHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().groupings;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let grouping_deps_backend = self
                .get_backend_logic()
                .groupings_check_can_remove(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            let handle_managers = &mut self.get_handle_managers_mut();
            let grouping_incompat_handle_manager = &mut handle_managers.grouping_incompats;

            let grouping_deps = grouping_deps_backend
                .into_iter()
                .map(|dep| grouping_incompat_handle_manager.get_handle(dep))
                .collect();

            Ok(grouping_deps)
        }
    }

    fn grouping_incompats_get(
        &mut self,
        handle: GroupingIncompatHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::GroupingIncompat<GroupingHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, GroupingIncompatHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().grouping_incompats;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let grouping_incompat = self
                .get_backend_logic()
                .grouping_incompats_get(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            Ok(private::convert_grouping_incompat_to_handles(
                grouping_incompat,
                self.get_handle_managers_mut(),
            ))
        }
    }

    fn grouping_incompats_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<GroupingIncompatHandle, backend::GroupingIncompat<GroupingHandle>>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let grouping_incompats_backend = self
                .get_backend_logic()
                .grouping_incompats_get_all()
                .await?;

            let grouping_incompats = grouping_incompats_backend
                .into_iter()
                .map(|(id, grouping_incompat)| {
                    let handle = self
                        .get_handle_managers_mut()
                        .grouping_incompats
                        .get_handle(id);
                    let grouping_incompat = private::convert_grouping_incompat_to_handles(
                        grouping_incompat,
                        self.get_handle_managers_mut(),
                    );
                    (handle, grouping_incompat)
                })
                .collect();

            Ok(grouping_incompats)
        }
    }

    fn grouping_incompats_check_data(
        &self,
        grouping_incompat: &backend::GroupingIncompat<GroupingHandle>,
    ) -> impl core::future::Future<
        Output = Result<
            backend::DataStatusWithId<GroupingHandle>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let grouping_incompat_backend = match private::convert_grouping_incompat_from_handles(
                grouping_incompat.clone(),
                self.get_handle_managers(),
            ) {
                Ok(val) => val,
                Err(status) => return Ok(status),
            };

            let status_backend = self
                .get_backend_logic()
                .grouping_incompats_check_data(&grouping_incompat_backend)
                .await?;

            let status = match status_backend {
                backend::DataStatusWithId::BadCrossId(_id) => {
                    panic!("GroupingId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId::Ok => backend::DataStatusWithId::Ok,
            };

            Ok(status)
        }
    }

    fn subject_group_for_student_get(
        &mut self,
        student_handle: StudentHandle,
        subject_group_handle: SubjectGroupHandle,
    ) -> impl core::future::Future<
        Output = Result<
            Option<SubjectHandle>,
            backend::Id2Error<
                <Self::Storage as backend::Storage>::InternalError,
                StudentHandle,
                SubjectGroupHandle,
            >,
        >,
    > + Send {
        async move {
            let student_id = self
                .get_handle_managers()
                .students
                .get_id(student_handle)
                .ok_or(backend::Id2Error::InvalidId1(student_handle))?;
            let subject_group_id = self
                .get_handle_managers()
                .subject_groups
                .get_id(subject_group_handle)
                .ok_or(backend::Id2Error::InvalidId2(subject_group_handle))?;

            let subject_id = self
                .get_backend_logic()
                .subject_group_for_student_get(student_id, subject_group_id)
                .await
                .map_err(|e| match e {
                    backend::Id2Error::InternalError(int_err) => {
                        backend::Id2Error::InternalError(int_err)
                    }
                    backend::Id2Error::InvalidId1(_) => {
                        panic!("StudentId was taken from a handle manager and thus should be valid")
                    }
                    backend::Id2Error::InvalidId2(_) => panic!(
                        "SubjectGroupId was taken from a handle manager and thus should be valid"
                    ),
                })?;

            let subject_handle =
                subject_id.map(|x| self.get_handle_managers_mut().subjects.get_handle(x));

            Ok(subject_handle)
        }
    }

    fn incompat_for_student_get(
        &self,
        student_handle: StudentHandle,
        incompat_handle: IncompatHandle,
    ) -> impl core::future::Future<
        Output = Result<
            bool,
            backend::Id2Error<
                <Self::Storage as backend::Storage>::InternalError,
                StudentHandle,
                IncompatHandle,
            >,
        >,
    > + Send {
        async move {
            let student_id = self
                .get_handle_managers()
                .students
                .get_id(student_handle)
                .ok_or(backend::Id2Error::InvalidId1(student_handle))?;
            let incompat_id = self
                .get_handle_managers()
                .incompats
                .get_id(incompat_handle)
                .ok_or(backend::Id2Error::InvalidId2(incompat_handle))?;

            let output = self
                .get_backend_logic()
                .incompat_for_student_get(student_id, incompat_id)
                .await
                .map_err(|e| match e {
                    backend::Id2Error::InternalError(int_err) => {
                        backend::Id2Error::InternalError(int_err)
                    }
                    backend::Id2Error::InvalidId1(_) => {
                        panic!("StudentId was taken from a handle manager and thus should be valid")
                    }
                    backend::Id2Error::InvalidId2(_) => panic!(
                        "SubjectGroupId was taken from a handle manager and thus should be valid"
                    ),
                })?;

            Ok(output)
        }
    }

    fn colloscopes_get(
        &mut self,
        handle: ColloscopeHandle,
    ) -> impl core::future::Future<
        Output = Result<
            backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
            IdError<<Self::Storage as backend::Storage>::InternalError, ColloscopeHandle>,
        >,
    > + Send {
        async move {
            let handle_manager = &self.get_handle_managers().colloscopes;
            let Some(index) = handle_manager.get_id(handle) else {
                return Err(IdError::InvalidId(handle));
            };

            let colloscope = self
                .get_backend_logic()
                .colloscopes_get(index)
                .await
                .map_err(|e| match e {
                    IdError::InternalError(int_err) => IdError::InternalError(int_err),
                    IdError::InvalidId(_id) => IdError::InvalidId(handle),
                })?;

            Ok(private::convert_colloscope_to_handles(
                colloscope,
                self.get_handle_managers_mut(),
            ))
        }
    }

    fn colloscopes_get_all(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<
            BTreeMap<
                ColloscopeHandle,
                backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
            >,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let colloscopes_backend = self.get_backend_logic().colloscopes_get_all().await?;

            let colloscopes = colloscopes_backend
                .into_iter()
                .map(|(id, colloscope)| {
                    let handle = self.get_handle_managers_mut().colloscopes.get_handle(id);
                    let colloscope = private::convert_colloscope_to_handles(
                        colloscope,
                        self.get_handle_managers_mut(),
                    );
                    (handle, colloscope)
                })
                .collect();

            Ok(colloscopes)
        }
    }

    fn colloscopes_check_data(
        &self,
        colloscope: &backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
    ) -> impl core::future::Future<
        Output = Result<
            backend::DataStatusWithId3<TeacherHandle, SubjectHandle, StudentHandle>,
            <Self::Storage as backend::Storage>::InternalError,
        >,
    > + Send {
        async {
            let colloscope_backend = match private::convert_colloscope_from_handles(
                colloscope.clone(),
                self.get_handle_managers(),
            ) {
                Ok(val) => val,
                Err(status) => return Ok(status),
            };

            let status_backend = self
                .get_backend_logic()
                .colloscopes_check_data(&colloscope_backend)
                .await?;

            let status = match status_backend {
                backend::DataStatusWithId3::BadCrossId1(_id) => {
                    panic!("TeacherId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId3::BadCrossId2(_id) => {
                    panic!("SubjectId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId3::BadCrossId3(_id) => {
                    panic!("StudentId was taken from a handle manager and thus should be valid")
                }
                backend::DataStatusWithId3::Ok => backend::DataStatusWithId3::Ok,
            };

            Ok(status)
        }
    }

    fn apply(
        &mut self,
        op: Operation,
    ) -> impl core::future::Future<
        Output = Result<
            ReturnHandle,
            UpdateError<<Self::Storage as backend::Storage>::InternalError>,
        >,
    > + Send {
        async {
            let rev_op = private::build_rev_op(self, op).await?;

            let output = private::update_internal_state(self, &rev_op.forward).await?;

            let aggregated_ops = AggregatedOperations::new(vec![rev_op]);
            self.get_history_mut().apply(aggregated_ops);

            Ok(output)
        }
    }

    fn can_undo(&self) -> bool {
        self.get_history().can_undo()
    }
    fn can_redo(&self) -> bool {
        self.get_history().can_redo()
    }

    fn undo(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<(), UndoError<<Self::Storage as backend::Storage>::InternalError>>,
    > + Send {
        async {
            match self.get_history_mut().undo() {
                Some(aggregated_ops) => {
                    private::update_internal_state_with_aggregated(self, &aggregated_ops).await.map_err(
                        |e| match e {
                            UpdateError::Internal(int_err) => UndoError::InternalError(int_err),
                            _ => panic!("Data should be consistent as it was automatically build from previous state.\n{}", e),
                        }
                    )?;
                    Ok(())
                }
                None => Err(UndoError::HistoryDepleted),
            }
        }
    }

    fn redo(
        &mut self,
    ) -> impl core::future::Future<
        Output = Result<(), RedoError<<Self::Storage as backend::Storage>::InternalError>>,
    > + Send {
        async {
            match self.get_history_mut().redo() {
                Some(aggregated_ops) => {
                    private::update_internal_state_with_aggregated(self, &aggregated_ops).await.map_err(
                        |e| match e {
                            UpdateError::Internal(int_err) => RedoError::InternalError(int_err),
                            _ => panic!("Data should be consistent as it was automatically build from previous state"),
                        }
                    )?;
                    Ok(())
                }
                None => Err(RedoError::HistoryFullyRewounded),
            }
        }
    }

    fn get_aggregated_history(&self) -> AggregatedOperations {
        self.get_history().build_aggregated_ops()
    }
}

pub(super) mod private {
    use self::backend::{
        DataStatusWithId, DataStatusWithIdAndInvalidState, SubjectDependancy,
        SubjectGroupDependancy,
    };

    use super::*;

    #[trait_variant::make(Send)]
    pub trait ManagerInternal: Sized + Send + Sync {
        type Storage: backend::Storage;

        fn get_backend_logic_mut(&mut self) -> &mut backend::Logic<Self::Storage>;
        fn get_handle_managers_mut(&mut self) -> &mut handles::ManagerCollection<Self::Storage>;
        fn get_history_mut(&mut self) -> &mut ModificationHistory;

        fn get_backend_logic(&self) -> &backend::Logic<Self::Storage>;
        fn get_handle_managers(&self) -> &handles::ManagerCollection<Self::Storage>;
        fn get_history(&self) -> &ModificationHistory;
    }

    pub async fn update_general_data_state<T: ManagerInternal>(
        manager: &mut T,
        general_data: &backend::GeneralData,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        if let Some(range) = &general_data.interrogations_per_week {
            if range.is_empty() {
                return Err(UpdateError::InterrogationsPerWeekRangeIsEmpty);
            }
        }

        manager
            .get_backend_logic_mut()
            .general_data_set(&general_data)
            .await
            .map_err(|e| match e {
                backend::CheckedError::CheckFailed(data) => {
                    let translated_data = data
                        .into_iter()
                        .map(|id| {
                            manager
                                .get_handle_managers_mut()
                                .week_patterns
                                .get_handle(id)
                        })
                        .collect();
                    UpdateError::WeekPatternsNeedTruncating(translated_data)
                }
                backend::CheckedError::InternalError(int_error) => UpdateError::Internal(int_error),
            })?;
        Ok(ReturnHandle::NoHandle)
    }

    pub async fn update_week_patterns_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedWeekPatternsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedWeekPatternsOperation::Create(week_pattern_handle, pattern) => {
                let new_id = manager
                    .get_backend_logic_mut()
                    .week_patterns_add(pattern)
                    .await
                    .map_err(|e| match e {
                        backend::WeekPatternError::WeekNumberTooBig(week_number) => {
                            UpdateError::WeekNumberTooBig(week_number)
                        }
                        backend::WeekPatternError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .week_patterns
                    .update_handle(*week_pattern_handle, Some(new_id));
                Ok(ReturnHandle::WeekPattern(*week_pattern_handle))
            }
            AnnotatedWeekPatternsOperation::Remove(week_pattern_handle) => {
                let week_pattern_id = manager
                    .get_handle_managers()
                    .week_patterns
                    .get_id(*week_pattern_handle)
                    .ok_or(UpdateError::WeekPatternRemoved(*week_pattern_handle))?;
                manager
                    .get_backend_logic_mut()
                    .week_patterns_remove(week_pattern_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| match dep {
                                    WeekPatternDependancy::Incompat(id) => {
                                        WeekPatternDependancy::Incompat(
                                            manager
                                                .get_handle_managers_mut()
                                                .incompats
                                                .get_handle(id),
                                        )
                                    }
                                    WeekPatternDependancy::TimeSlot(id) => {
                                        WeekPatternDependancy::TimeSlot(
                                            manager
                                                .get_handle_managers_mut()
                                                .time_slots
                                                .get_handle(id),
                                        )
                                    }
                                })
                                .collect();
                            UpdateError::WeekPatternDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .week_patterns
                    .update_handle(*week_pattern_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedWeekPatternsOperation::Update(week_pattern_handle, pattern) => {
                let week_pattern_id = manager
                    .get_handle_managers()
                    .week_patterns
                    .get_id(*week_pattern_handle)
                    .ok_or(UpdateError::WeekPatternRemoved(*week_pattern_handle))?;
                manager
                    .get_backend_logic_mut()
                    .week_patterns_update(week_pattern_id, pattern)
                    .await
                    .map_err(|e| match e {
                        backend::WeekPatternIdError::WeekNumberTooBig(week_number) => {
                            UpdateError::WeekNumberTooBig(week_number)
                        }
                        backend::WeekPatternIdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::WeekPatternIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_teachers_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedTeachersOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedTeachersOperation::Create(teacher_handle, teacher) => {
                let new_id = manager
                    .get_backend_logic_mut()
                    .teachers_add(teacher)
                    .await
                    .map_err(|e| UpdateError::Internal(e))?;
                manager
                    .get_handle_managers_mut()
                    .teachers
                    .update_handle(*teacher_handle, Some(new_id));
                Ok(ReturnHandle::Teacher(*teacher_handle))
            }
            AnnotatedTeachersOperation::Remove(teacher_handle) => {
                let teacher_id = manager
                    .get_handle_managers()
                    .teachers
                    .get_id(*teacher_handle)
                    .ok_or(UpdateError::TeacherRemoved(*teacher_handle))?;
                manager
                    .get_backend_logic_mut()
                    .teachers_remove(teacher_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| match dep {
                                    backend::TeacherDependancy::TimeSlot(id) => {
                                        backend::TeacherDependancy::TimeSlot(
                                            manager
                                                .get_handle_managers_mut()
                                                .time_slots
                                                .get_handle(id),
                                        )
                                    }
                                    backend::TeacherDependancy::Colloscope(id) => {
                                        backend::TeacherDependancy::Colloscope(
                                            manager
                                                .get_handle_managers_mut()
                                                .colloscopes
                                                .get_handle(id),
                                        )
                                    }
                                })
                                .collect();
                            UpdateError::TeacherDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .teachers
                    .update_handle(*teacher_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedTeachersOperation::Update(teacher_handle, teacher) => {
                let teacher_id = manager
                    .get_handle_managers()
                    .teachers
                    .get_id(*teacher_handle)
                    .ok_or(UpdateError::TeacherRemoved(*teacher_handle))?;
                manager
                    .get_backend_logic_mut()
                    .teachers_update(teacher_id, teacher)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_students_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedStudentsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedStudentsOperation::Create(student_handle, student) => {
                let new_id = manager
                    .get_backend_logic_mut()
                    .students_add(student)
                    .await
                    .map_err(|e| UpdateError::Internal(e))?;
                manager
                    .get_handle_managers_mut()
                    .students
                    .update_handle(*student_handle, Some(new_id));
                Ok(ReturnHandle::Student(*student_handle))
            }
            AnnotatedStudentsOperation::Remove(student_handle) => {
                let student_id = manager
                    .get_handle_managers()
                    .students
                    .get_id(*student_handle)
                    .ok_or(UpdateError::StudentRemoved(*student_handle))?;
                manager
                    .get_backend_logic_mut()
                    .students_remove(student_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| match dep {
                                    backend::StudentDependancy::GroupList(id) => {
                                        backend::StudentDependancy::GroupList(
                                            manager
                                                .get_handle_managers_mut()
                                                .group_lists
                                                .get_handle(id),
                                        )
                                    }
                                    backend::StudentDependancy::Colloscope(id) => {
                                        backend::StudentDependancy::Colloscope(
                                            manager
                                                .get_handle_managers_mut()
                                                .colloscopes
                                                .get_handle(id),
                                        )
                                    }
                                })
                                .collect();
                            UpdateError::StudentDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .students
                    .update_handle(*student_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedStudentsOperation::Update(student_handle, student) => {
                let student_id = manager
                    .get_handle_managers()
                    .students
                    .get_id(*student_handle)
                    .ok_or(UpdateError::StudentRemoved(*student_handle))?;
                manager
                    .get_backend_logic_mut()
                    .students_update(student_id, student)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_subject_groups_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedSubjectGroupsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedSubjectGroupsOperation::Create(subject_group_handle, subject_group) => {
                let new_id = manager
                    .get_backend_logic_mut()
                    .subject_groups_add(subject_group)
                    .await
                    .map_err(|e| UpdateError::Internal(e))?;
                manager
                    .get_handle_managers_mut()
                    .subject_groups
                    .update_handle(*subject_group_handle, Some(new_id));
                Ok(ReturnHandle::SubjectGroup(*subject_group_handle))
            }
            AnnotatedSubjectGroupsOperation::Remove(subject_group_handle) => {
                let subject_group_id = manager
                    .get_handle_managers()
                    .subject_groups
                    .get_id(*subject_group_handle)
                    .ok_or(UpdateError::SubjectGroupRemoved(*subject_group_handle))?;
                manager
                    .get_backend_logic_mut()
                    .subject_groups_remove(subject_group_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| match dep {
                                    SubjectGroupDependancy::Student(id) => {
                                        SubjectGroupDependancy::Student(
                                            manager
                                                .get_handle_managers_mut()
                                                .students
                                                .get_handle(id),
                                        )
                                    }
                                    SubjectGroupDependancy::Subject(id) => {
                                        SubjectGroupDependancy::Subject(
                                            manager
                                                .get_handle_managers_mut()
                                                .subjects
                                                .get_handle(id),
                                        )
                                    }
                                })
                                .collect();
                            UpdateError::SubjectGroupDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .subject_groups
                    .update_handle(*subject_group_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedSubjectGroupsOperation::Update(subject_group_handle, subject_group) => {
                let student_id = manager
                    .get_handle_managers()
                    .subject_groups
                    .get_id(*subject_group_handle)
                    .ok_or(UpdateError::SubjectGroupRemoved(*subject_group_handle))?;
                manager
                    .get_backend_logic_mut()
                    .subject_groups_update(student_id, subject_group)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_incompats_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedIncompatsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedIncompatsOperation::Create(incompat_handle, incompat) => {
                let incompat_backend = match convert_incompat_from_handles(
                    incompat.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        DataStatusWithId::Ok => panic!("DataStatusWithId::Ok is not an error"),
                        DataStatusWithId::BadCrossId(week_pattern_handle) => {
                            return Err(UpdateError::IncompatBadWeekPattern(week_pattern_handle))
                        }
                    },
                };
                let new_id = manager
                    .get_backend_logic_mut()
                    .incompats_add(&incompat_backend)
                    .await
                    .map_err(|e| match e {
                        backend::CrossError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CrossError::InvalidCrossId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .incompats
                    .update_handle(*incompat_handle, Some(new_id));
                Ok(ReturnHandle::Incompat(*incompat_handle))
            }
            AnnotatedIncompatsOperation::Remove(incompat_handle) => {
                let incompat_id = manager
                    .get_handle_managers()
                    .incompats
                    .get_id(*incompat_handle)
                    .ok_or(UpdateError::IncompatRemoved(*incompat_handle))?;
                manager
                    .get_backend_logic_mut()
                    .incompats_remove(incompat_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| match dep {
                                    IncompatDependancy::Student(id) => IncompatDependancy::Student(
                                        manager.get_handle_managers_mut().students.get_handle(id),
                                    ),
                                    IncompatDependancy::Subject(id) => IncompatDependancy::Subject(
                                        manager.get_handle_managers_mut().subjects.get_handle(id),
                                    ),
                                })
                                .collect();
                            UpdateError::IncompatDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .incompats
                    .update_handle(*incompat_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedIncompatsOperation::Update(incompat_handle, incompat) => {
                let incompat_backend = match convert_incompat_from_handles(
                    incompat.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        DataStatusWithId::Ok => panic!("DataStatusWithId::Ok is not an error"),
                        DataStatusWithId::BadCrossId(week_pattern_handle) => {
                            return Err(UpdateError::IncompatBadWeekPattern(week_pattern_handle))
                        }
                    },
                };
                let incompat_id = manager
                    .get_handle_managers()
                    .incompats
                    .get_id(*incompat_handle)
                    .ok_or(UpdateError::IncompatRemoved(*incompat_handle))?;
                manager
                    .get_backend_logic_mut()
                    .incompats_update(incompat_id, &incompat_backend)
                    .await
                    .map_err(|e| match e {
                        backend::CrossIdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::CrossIdError::InvalidCrossId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CrossIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_group_lists_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedGroupListsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedGroupListsOperation::Create(group_list_handle, group_list) => {
                let group_list_backend = match convert_group_list_from_handles(
                    group_list.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        DataStatusWithIdAndInvalidState::Ok => {
                            panic!("DataStatusWithIdAndInvalidState::Ok is not an error")
                        }
                        DataStatusWithIdAndInvalidState::BadCrossId(student_handle) => {
                            return Err(UpdateError::GroupListBadStudent(student_handle))
                        }
                        DataStatusWithIdAndInvalidState::InvalidData => {
                            return Err(UpdateError::GroupListWithInconsistentStudentMapping)
                        }
                    },
                };
                let new_id = manager
                    .get_backend_logic_mut()
                    .group_lists_add(&group_list_backend)
                    .await
                    .map_err(|e| match e {
                        backend::InvalidCrossError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::InvalidCrossError::InvalidCrossId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::InvalidCrossError::InvalidData(_data) => {
                            UpdateError::GroupListWithInconsistentStudentMapping
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .group_lists
                    .update_handle(*group_list_handle, Some(new_id));
                Ok(ReturnHandle::GroupList(*group_list_handle))
            }
            AnnotatedGroupListsOperation::Remove(group_list_handle) => {
                let group_list_id = manager
                    .get_handle_managers()
                    .group_lists
                    .get_id(*group_list_handle)
                    .ok_or(UpdateError::GroupListRemoved(*group_list_handle))?;
                manager
                    .get_backend_logic_mut()
                    .group_lists_remove(group_list_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| {
                                    manager.get_handle_managers_mut().subjects.get_handle(dep)
                                })
                                .collect();
                            UpdateError::GroupListDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .group_lists
                    .update_handle(*group_list_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedGroupListsOperation::Update(group_list_handle, group_list) => {
                let group_list_backend = match convert_group_list_from_handles(
                    group_list.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        DataStatusWithIdAndInvalidState::Ok => {
                            panic!("DataStatusWithIdAndInvalidState::Ok is not an error")
                        }
                        DataStatusWithIdAndInvalidState::BadCrossId(student_handle) => {
                            return Err(UpdateError::GroupListBadStudent(student_handle))
                        }
                        DataStatusWithIdAndInvalidState::InvalidData => {
                            return Err(UpdateError::GroupListWithInconsistentStudentMapping)
                        }
                    },
                };
                let group_list_id = manager
                    .get_handle_managers()
                    .group_lists
                    .get_id(*group_list_handle)
                    .ok_or(UpdateError::GroupListRemoved(*group_list_handle))?;
                manager
                    .get_backend_logic_mut()
                    .group_lists_update(group_list_id, &group_list_backend)
                    .await
                    .map_err(|e| match e {
                        backend::InvalidCrossIdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::InvalidCrossIdError::InvalidCrossId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::InvalidCrossIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::InvalidCrossIdError::InvalidData(_) => {
                            UpdateError::GroupListWithInconsistentStudentMapping
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_subjects_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedSubjectsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedSubjectsOperation::Create(subject_handle, subject) => {
                let subject_backend = match convert_subject_from_handles(
                    subject.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId3::Ok => {
                            panic!("DataStatusWithIdAndInvalidState::Ok is not an error")
                        }
                        backend::DataStatusWithId3::BadCrossId1(subject_group_handle) => {
                            return Err(UpdateError::SubjectBadSubjectGroup(subject_group_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId2(incompat_handle) => {
                            return Err(UpdateError::SubjectBadIncompat(incompat_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId3(group_list_handle) => {
                            return Err(UpdateError::SubjectBadGroupList(group_list_handle))
                        }
                    },
                };
                let new_id = manager
                    .get_backend_logic_mut()
                    .subjects_add(&subject_backend)
                    .await
                    .map_err(|e| match e {
                        backend::Cross3Error::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::Cross3Error::InvalidCrossId1(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3Error::InvalidCrossId2(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3Error::InvalidCrossId3(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .subjects
                    .update_handle(*subject_handle, Some(new_id));
                Ok(ReturnHandle::Subject(*subject_handle))
            }
            AnnotatedSubjectsOperation::Remove(subject_handle) => {
                let subject_id = manager
                    .get_handle_managers()
                    .subjects
                    .get_id(*subject_handle)
                    .ok_or(UpdateError::SubjectRemoved(*subject_handle))?;
                manager
                    .get_backend_logic_mut()
                    .subjects_remove(subject_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| match dep {
                                    SubjectDependancy::Student(id) => SubjectDependancy::Student(
                                        manager.get_handle_managers_mut().students.get_handle(id),
                                    ),
                                    SubjectDependancy::TimeSlot(id) => SubjectDependancy::TimeSlot(
                                        manager.get_handle_managers_mut().time_slots.get_handle(id),
                                    ),
                                    SubjectDependancy::Colloscope(id) => {
                                        SubjectDependancy::Colloscope(
                                            manager
                                                .get_handle_managers_mut()
                                                .colloscopes
                                                .get_handle(id),
                                        )
                                    }
                                })
                                .collect();
                            UpdateError::SubjectDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .subjects
                    .update_handle(*subject_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedSubjectsOperation::Update(subject_handle, subject) => {
                let subject_backend = match convert_subject_from_handles(
                    subject.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId3::Ok => {
                            panic!("DataStatusWithIdAndInvalidState::Ok is not an error")
                        }
                        backend::DataStatusWithId3::BadCrossId1(subject_group_handle) => {
                            return Err(UpdateError::SubjectBadSubjectGroup(subject_group_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId2(incompat_handle) => {
                            return Err(UpdateError::SubjectBadIncompat(incompat_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId3(group_list_handle) => {
                            return Err(UpdateError::SubjectBadGroupList(group_list_handle))
                        }
                    },
                };
                let subject_id = manager
                    .get_handle_managers()
                    .subjects
                    .get_id(*subject_handle)
                    .ok_or(UpdateError::SubjectRemoved(*subject_handle))?;
                manager
                    .get_backend_logic_mut()
                    .subjects_update(subject_id, &subject_backend)
                    .await
                    .map_err(|e| match e {
                        backend::Cross3IdWithDepError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::Cross3IdWithDepError::InvalidCrossId1(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdWithDepError::InvalidCrossId2(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdWithDepError::InvalidCrossId3(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdWithDepError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdWithDepError::BlockingDependancy(id) => {
                            UpdateError::SubjectWithStudentRegistered(
                                manager.get_handle_managers_mut().students.get_handle(id),
                            )
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_time_slots_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedTimeSlotsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedTimeSlotsOperation::Create(time_slot_handle, time_slot) => {
                let time_slot_backend = match convert_time_slot_from_handles(
                    time_slot.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId3::Ok => {
                            panic!("DataStatusWithId3::Ok is not an error")
                        }
                        backend::DataStatusWithId3::BadCrossId1(subject_handle) => {
                            return Err(UpdateError::TimeSlotBadSubject(subject_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId2(teacher_handle) => {
                            return Err(UpdateError::TimeSlotBadTeacher(teacher_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId3(week_pattern_handle) => {
                            return Err(UpdateError::TimeSlotBadWeekPattern(week_pattern_handle))
                        }
                    },
                };
                let new_id = manager
                    .get_backend_logic_mut()
                    .time_slots_add(&time_slot_backend)
                    .await
                    .map_err(|e| match e {
                        backend::Cross3Error::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::Cross3Error::InvalidCrossId1(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3Error::InvalidCrossId2(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3Error::InvalidCrossId3(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .time_slots
                    .update_handle(*time_slot_handle, Some(new_id));
                Ok(ReturnHandle::TimeSlot(*time_slot_handle))
            }
            AnnotatedTimeSlotsOperation::Remove(time_slot_handle) => {
                let time_slot_id = manager
                    .get_handle_managers()
                    .time_slots
                    .get_id(*time_slot_handle)
                    .ok_or(UpdateError::TimeSlotRemoved(*time_slot_handle))?;
                manager
                    .get_backend_logic_mut()
                    .time_slots_remove(time_slot_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| {
                                    manager.get_handle_managers_mut().groupings.get_handle(dep)
                                })
                                .collect();
                            UpdateError::TimeSlotDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .time_slots
                    .update_handle(*time_slot_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedTimeSlotsOperation::Update(time_slot_handle, time_slot) => {
                let time_slot_backend = match convert_time_slot_from_handles(
                    time_slot.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId3::Ok => {
                            panic!("DataStatusWithId3::Ok is not an error")
                        }
                        backend::DataStatusWithId3::BadCrossId1(subject_handle) => {
                            return Err(UpdateError::TimeSlotBadSubject(subject_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId2(teacher_handle) => {
                            return Err(UpdateError::TimeSlotBadTeacher(teacher_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId3(week_pattern_handle) => {
                            return Err(UpdateError::TimeSlotBadWeekPattern(week_pattern_handle))
                        }
                    },
                };
                let time_slot_id = manager
                    .get_handle_managers()
                    .time_slots
                    .get_id(*time_slot_handle)
                    .ok_or(UpdateError::TimeSlotRemoved(*time_slot_handle))?;
                manager
                    .get_backend_logic_mut()
                    .time_slots_update(time_slot_id, &time_slot_backend)
                    .await
                    .map_err(|e| match e {
                        backend::Cross3IdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::Cross3IdError::InvalidCrossId1(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdError::InvalidCrossId2(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdError::InvalidCrossId3(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_groupings_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedGroupingsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedGroupingsOperation::Create(grouping_handle, grouping) => {
                let grouping_backend = match convert_grouping_from_handles(
                    grouping.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId::Ok => {
                            panic!("DataStatusWithId::Ok is not an error")
                        }
                        backend::DataStatusWithId::BadCrossId(time_slot_handle) => {
                            return Err(UpdateError::GroupingBadTimeSlot(time_slot_handle))
                        }
                    },
                };
                let new_id = manager
                    .get_backend_logic_mut()
                    .groupings_add(&grouping_backend)
                    .await
                    .map_err(|e| match e {
                        backend::CrossError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CrossError::InvalidCrossId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .groupings
                    .update_handle(*grouping_handle, Some(new_id));
                Ok(ReturnHandle::Grouping(*grouping_handle))
            }
            AnnotatedGroupingsOperation::Remove(grouping_handle) => {
                let grouping_id = manager
                    .get_handle_managers()
                    .groupings
                    .get_id(*grouping_handle)
                    .ok_or(UpdateError::GroupingRemoved(*grouping_handle))?;
                manager
                    .get_backend_logic_mut()
                    .groupings_remove(grouping_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            let new_dependancies = dependancies
                                .into_iter()
                                .map(|dep| {
                                    manager
                                        .get_handle_managers_mut()
                                        .grouping_incompats
                                        .get_handle(dep)
                                })
                                .collect();
                            UpdateError::GroupingDependanciesRemaining(new_dependancies)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .groupings
                    .update_handle(*grouping_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedGroupingsOperation::Update(grouping_handle, grouping) => {
                let grouping_backend = match convert_grouping_from_handles(
                    grouping.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId::Ok => {
                            panic!("DataStatusWithId::Ok is not an error")
                        }
                        backend::DataStatusWithId::BadCrossId(time_slot_handle) => {
                            return Err(UpdateError::GroupingBadTimeSlot(time_slot_handle))
                        }
                    },
                };
                let grouping_id = manager
                    .get_handle_managers()
                    .groupings
                    .get_id(*grouping_handle)
                    .ok_or(UpdateError::GroupingRemoved(*grouping_handle))?;
                manager
                    .get_backend_logic_mut()
                    .groupings_update(grouping_id, &grouping_backend)
                    .await
                    .map_err(|e| match e {
                        backend::CrossIdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::CrossIdError::InvalidCrossId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CrossIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_grouping_incompats_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedGroupingIncompatsOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedGroupingIncompatsOperation::Create(
                grouping_incompat_handle,
                grouping_incompat,
            ) => {
                let grouping_incompat_backend = match convert_grouping_incompat_from_handles(
                    grouping_incompat.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId::Ok => {
                            panic!("DataStatusWithId::Ok is not an error")
                        }
                        backend::DataStatusWithId::BadCrossId(grouping_handle) => {
                            return Err(UpdateError::GroupingIncompatBadGrouping(grouping_handle))
                        }
                    },
                };
                let new_id = manager
                    .get_backend_logic_mut()
                    .grouping_incompats_add(&grouping_incompat_backend)
                    .await
                    .map_err(|e| match e {
                        backend::CrossError::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CrossError::InvalidCrossId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .grouping_incompats
                    .update_handle(*grouping_incompat_handle, Some(new_id));
                Ok(ReturnHandle::GroupingIncompat(*grouping_incompat_handle))
            }
            AnnotatedGroupingIncompatsOperation::Remove(grouping_incompat_handle) => {
                let grouping_incompat_id = manager
                    .get_handle_managers()
                    .grouping_incompats
                    .get_id(*grouping_incompat_handle)
                    .ok_or(UpdateError::GroupingIncompatRemoved(
                        *grouping_incompat_handle,
                    ))?;
                manager
                    .get_backend_logic_mut()
                    .grouping_incompats_remove(grouping_incompat_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => UpdateError::Internal(int_err),
                    })?;
                manager
                    .get_handle_managers_mut()
                    .grouping_incompats
                    .update_handle(*grouping_incompat_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedGroupingIncompatsOperation::Update(
                grouping_incompat_handle,
                grouping_incompat,
            ) => {
                let grouping_incompat_backend = match convert_grouping_incompat_from_handles(
                    grouping_incompat.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId::Ok => {
                            panic!("DataStatusWithId::Ok is not an error")
                        }
                        backend::DataStatusWithId::BadCrossId(grouping_handle) => {
                            return Err(UpdateError::GroupingIncompatBadGrouping(grouping_handle))
                        }
                    },
                };
                let grouping_incompat_id = manager
                    .get_handle_managers()
                    .grouping_incompats
                    .get_id(*grouping_incompat_handle)
                    .ok_or(UpdateError::GroupingIncompatRemoved(
                        *grouping_incompat_handle,
                    ))?;
                manager
                    .get_backend_logic_mut()
                    .grouping_incompats_update(grouping_incompat_id, &grouping_incompat_backend)
                    .await
                    .map_err(|e| match e {
                        backend::CrossIdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::CrossIdError::InvalidCrossId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CrossIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_register_student_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedRegisterStudentOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedRegisterStudentOperation::InSubjectGroup(
                student_handle,
                subject_group_handle,
                subject_handle,
            ) => {
                let student_id = manager
                    .get_handle_managers()
                    .students
                    .get_id(*student_handle)
                    .ok_or(UpdateError::StudentRemoved(*student_handle))?;

                let subject_group_id = manager
                    .get_handle_managers()
                    .subject_groups
                    .get_id(*subject_group_handle)
                    .ok_or(UpdateError::SubjectGroupRemoved(*subject_group_handle))?;

                let subject_id = subject_handle
                    .map(|x| {
                        manager
                            .get_handle_managers()
                            .subjects
                            .get_id(x)
                            .ok_or(UpdateError::SubjectRemoved(x))
                    })
                    .transpose()?;

                manager
                    .get_backend_logic_mut()
                    .subject_group_for_student_set(student_id, subject_group_id, subject_id)
                    .await
                    .map_err(|e| match e {
                        backend::CrossId3Error::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::CrossId3Error::InvalidId1(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CrossId3Error::InvalidId2(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CrossId3Error::InvalidId3(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CrossId3Error::InvalidCrossId(_id) => {
                            UpdateError::RegisterStudentBadSubject(
                                *subject_group_handle,
                                subject_handle
                                    .expect("To be a problem, the subject should be Some(...)"),
                            )
                        }
                    })?;

                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedRegisterStudentOperation::InIncompat(
                student_handle,
                incompat_handle,
                enabled,
            ) => {
                let student_id = manager
                    .get_handle_managers()
                    .students
                    .get_id(*student_handle)
                    .ok_or(UpdateError::StudentRemoved(*student_handle))?;

                let incompat_id = manager
                    .get_handle_managers()
                    .incompats
                    .get_id(*incompat_handle)
                    .ok_or(UpdateError::IncompatRemoved(*incompat_handle))?;

                manager
                    .get_backend_logic_mut()
                    .incompat_for_student_set(student_id, incompat_id, *enabled)
                    .await
                    .map_err(|e| match e {
                        backend::Id2Error::InternalError(int_err) => UpdateError::Internal(int_err),
                        backend::Id2Error::InvalidId1(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Id2Error::InvalidId2(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;

                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_colloscopes_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedColloscopesOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedColloscopesOperation::Create(colloscope_handle, colloscope) => {
                let colloscope_backend = match convert_colloscope_from_handles(
                    colloscope.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId3::Ok => {
                            panic!("DataStatusWithId3::Ok is not an error")
                        }
                        backend::DataStatusWithId3::BadCrossId1(teacher_handle) => {
                            return Err(UpdateError::ColloscopeBadTeacher(teacher_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId2(subject_handle) => {
                            return Err(UpdateError::ColloscopeBadSubject(subject_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId3(student_handle) => {
                            return Err(UpdateError::ColloscopeBadStudent(student_handle))
                        }
                    },
                };
                let new_id = manager
                    .get_backend_logic_mut()
                    .colloscopes_add(&colloscope_backend)
                    .await
                    .map_err(|e| match e {
                        backend::Cross3Error::InternalError(int_err) => {
                            UpdateError::Internal(int_err)
                        }
                        backend::Cross3Error::InvalidCrossId1(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3Error::InvalidCrossId2(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3Error::InvalidCrossId3(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                manager
                    .get_handle_managers_mut()
                    .colloscopes
                    .update_handle(*colloscope_handle, Some(new_id));
                Ok(ReturnHandle::Colloscope(*colloscope_handle))
            }
            AnnotatedColloscopesOperation::Remove(colloscope_handle) => {
                let colloscope_id = manager
                    .get_handle_managers()
                    .colloscopes
                    .get_id(*colloscope_handle)
                    .ok_or(UpdateError::ColloscopeRemoved(*colloscope_handle))?;
                manager
                    .get_backend_logic_mut()
                    .colloscopes_remove(colloscope_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => UpdateError::Internal(int_err),
                    })?;
                manager
                    .get_handle_managers_mut()
                    .colloscopes
                    .update_handle(*colloscope_handle, None);
                Ok(ReturnHandle::NoHandle)
            }
            AnnotatedColloscopesOperation::Update(colloscope_handle, colloscope) => {
                let colloscope_backend = match convert_colloscope_from_handles(
                    colloscope.clone(),
                    manager.get_handle_managers(),
                ) {
                    Ok(val) => val,
                    Err(err) => match err {
                        backend::DataStatusWithId3::Ok => {
                            panic!("DataStatusWithId3::Ok is not an error")
                        }
                        backend::DataStatusWithId3::BadCrossId1(teacher_handle) => {
                            return Err(UpdateError::ColloscopeBadTeacher(teacher_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId2(subject_handle) => {
                            return Err(UpdateError::ColloscopeBadSubject(subject_handle))
                        }
                        backend::DataStatusWithId3::BadCrossId3(student_handle) => {
                            return Err(UpdateError::ColloscopeBadStudent(student_handle))
                        }
                    },
                };
                let colloscope_id = manager
                    .get_handle_managers()
                    .colloscopes
                    .get_id(*colloscope_handle)
                    .ok_or(UpdateError::ColloscopeRemoved(*colloscope_handle))?;
                manager
                    .get_backend_logic_mut()
                    .colloscopes_update(colloscope_id, &colloscope_backend)
                    .await
                    .map_err(|e| match e {
                        backend::Cross3IdError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                        backend::Cross3IdError::InvalidCrossId1(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdError::InvalidCrossId2(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdError::InvalidCrossId3(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::Cross3IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                    })?;
                Ok(ReturnHandle::NoHandle)
            }
        }
    }

    pub async fn update_internal_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedOperation,
    ) -> Result<ReturnHandle, UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedOperation::GeneralData(data) => update_general_data_state(manager, data).await,
            AnnotatedOperation::WeekPatterns(op) => update_week_patterns_state(manager, op).await,
            AnnotatedOperation::Teachers(op) => update_teachers_state(manager, op).await,
            AnnotatedOperation::Students(op) => update_students_state(manager, op).await,
            AnnotatedOperation::SubjectGroups(op) => update_subject_groups_state(manager, op).await,
            AnnotatedOperation::Incompats(op) => update_incompats_state(manager, op).await,
            AnnotatedOperation::GroupLists(op) => update_group_lists_state(manager, op).await,
            AnnotatedOperation::Subjects(op) => update_subjects_state(manager, op).await,
            AnnotatedOperation::TimeSlots(op) => update_time_slots_state(manager, op).await,
            AnnotatedOperation::Groupings(op) => update_groupings_state(manager, op).await,
            AnnotatedOperation::GroupingIncompats(op) => {
                update_grouping_incompats_state(manager, op).await
            }
            AnnotatedOperation::RegisterStudent(op) => {
                update_register_student_state(manager, op).await
            }
            AnnotatedOperation::Colloscopes(op) => update_colloscopes_state(manager, op).await,
        }
    }

    pub async fn update_internal_state_with_aggregated<T: ManagerInternal>(
        manager: &mut T,
        aggregated_ops: &AggregatedOperations,
    ) -> Result<(), UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        let ops = aggregated_ops.inner();

        let mut error = None;
        let mut count = 0;

        for rev_op in ops {
            let result = update_internal_state(manager, &rev_op.forward).await;

            if let Err(err) = result {
                error = Some(err);
                break;
            }

            count += 1;
        }

        let Some(err) = error else {
            return Ok(());
        };

        let skip_size = ops.len() - count;
        for rev_op in ops.iter().rev().skip(skip_size) {
            let result = update_internal_state(manager, &rev_op.backward).await;

            if let Err(e) = result {
                panic!(
                    r#"Failed to reverse failed aggregated operations.
    Initial failed op: {:?}
    Initial error: {:?}
    Problematic op to reverse: {:?}
    Error in reversing: {:?}"#,
                    ops[count], err, rev_op, e,
                );
            }
        }

        Err(err)
    }

    pub async fn build_backward_general_data<T: ManagerInternal>(
        manager: &T,
    ) -> Result<backend::GeneralData, <T::Storage as backend::Storage>::InternalError> {
        manager.get_backend_logic().general_data_get().await
    }

    pub async fn build_backward_week_patterns_op<T: ManagerInternal>(
        manager: &T,
        op: &AnnotatedWeekPatternsOperation,
    ) -> Result<
        AnnotatedWeekPatternsOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedWeekPatternsOperation::Create(handle, _pattern) => {
                AnnotatedWeekPatternsOperation::Remove(*handle)
            }
            AnnotatedWeekPatternsOperation::Remove(handle) => {
                let week_pattern_id = manager
                    .get_handle_managers()
                    .week_patterns
                    .get_id(*handle)
                    .ok_or(RevError::WeekPatternRemoved(*handle))?;
                let pattern = manager
                    .get_backend_logic()
                    .week_patterns_get(week_pattern_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedWeekPatternsOperation::Create(*handle, pattern)
            }
            AnnotatedWeekPatternsOperation::Update(handle, _new_pattern) => {
                let week_pattern_id = manager
                    .get_handle_managers()
                    .week_patterns
                    .get_id(*handle)
                    .ok_or(RevError::WeekPatternRemoved(*handle))?;
                let pattern = manager
                    .get_backend_logic()
                    .week_patterns_get(week_pattern_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedWeekPatternsOperation::Update(*handle, pattern)
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_teachers_op<T: ManagerInternal>(
        manager: &T,
        op: &AnnotatedTeachersOperation,
    ) -> Result<AnnotatedTeachersOperation, RevError<<T::Storage as backend::Storage>::InternalError>>
    {
        let backward = match op {
            AnnotatedTeachersOperation::Create(handle, _pattern) => {
                AnnotatedTeachersOperation::Remove(*handle)
            }
            AnnotatedTeachersOperation::Remove(handle) => {
                let teacher_id = manager
                    .get_handle_managers()
                    .teachers
                    .get_id(*handle)
                    .ok_or(RevError::TeacherRemoved(*handle))?;
                let teacher = manager
                    .get_backend_logic()
                    .teachers_get(teacher_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedTeachersOperation::Create(*handle, teacher)
            }
            AnnotatedTeachersOperation::Update(handle, _new_teacher) => {
                let teacher_id = manager
                    .get_handle_managers()
                    .teachers
                    .get_id(*handle)
                    .ok_or(RevError::TeacherRemoved(*handle))?;
                let teacher = manager
                    .get_backend_logic()
                    .teachers_get(teacher_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedTeachersOperation::Update(*handle, teacher)
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_students_op<T: ManagerInternal>(
        manager: &T,
        op: &AnnotatedStudentsOperation,
    ) -> Result<AnnotatedStudentsOperation, RevError<<T::Storage as backend::Storage>::InternalError>>
    {
        let backward = match op {
            AnnotatedStudentsOperation::Create(handle, _pattern) => {
                AnnotatedStudentsOperation::Remove(*handle)
            }
            AnnotatedStudentsOperation::Remove(handle) => {
                let student_id = manager
                    .get_handle_managers()
                    .students
                    .get_id(*handle)
                    .ok_or(RevError::StudentRemoved(*handle))?;
                let student = manager
                    .get_backend_logic()
                    .students_get(student_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedStudentsOperation::Create(*handle, student)
            }
            AnnotatedStudentsOperation::Update(handle, _new_teacher) => {
                let student_id = manager
                    .get_handle_managers()
                    .students
                    .get_id(*handle)
                    .ok_or(RevError::StudentRemoved(*handle))?;
                let teacher = manager
                    .get_backend_logic()
                    .students_get(student_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedStudentsOperation::Update(*handle, teacher)
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_subject_groups_op<T: ManagerInternal>(
        manager: &T,
        op: &AnnotatedSubjectGroupsOperation,
    ) -> Result<
        AnnotatedSubjectGroupsOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedSubjectGroupsOperation::Create(handle, _subject_group) => {
                AnnotatedSubjectGroupsOperation::Remove(*handle)
            }
            AnnotatedSubjectGroupsOperation::Remove(handle) => {
                let subject_group_id = manager
                    .get_handle_managers()
                    .subject_groups
                    .get_id(*handle)
                    .ok_or(RevError::SubjectGroupRemoved(*handle))?;
                let subject_group = manager
                    .get_backend_logic()
                    .subject_groups_get(subject_group_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedSubjectGroupsOperation::Create(*handle, subject_group)
            }
            AnnotatedSubjectGroupsOperation::Update(handle, _new_subject_group) => {
                let subject_group_id = manager
                    .get_handle_managers()
                    .subject_groups
                    .get_id(*handle)
                    .ok_or(RevError::SubjectGroupRemoved(*handle))?;
                let subject_group = manager
                    .get_backend_logic()
                    .subject_groups_get(subject_group_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedSubjectGroupsOperation::Update(*handle, subject_group)
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_incompats_op<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedIncompatsOperation,
    ) -> Result<
        AnnotatedIncompatsOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedIncompatsOperation::Create(handle, _incompat) => {
                AnnotatedIncompatsOperation::Remove(*handle)
            }
            AnnotatedIncompatsOperation::Remove(handle) => {
                let incompat_id = manager
                    .get_handle_managers()
                    .incompats
                    .get_id(*handle)
                    .ok_or(RevError::IncompatRemoved(*handle))?;
                let incompat = manager
                    .get_backend_logic()
                    .incompats_get(incompat_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedIncompatsOperation::Create(
                    *handle,
                    convert_incompat_to_handles(incompat, manager.get_handle_managers_mut()),
                )
            }
            AnnotatedIncompatsOperation::Update(handle, _new_incompat) => {
                let incompat_id = manager
                    .get_handle_managers()
                    .incompats
                    .get_id(*handle)
                    .ok_or(RevError::IncompatRemoved(*handle))?;
                let incompat = manager
                    .get_backend_logic()
                    .incompats_get(incompat_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedIncompatsOperation::Update(
                    *handle,
                    convert_incompat_to_handles(incompat, manager.get_handle_managers_mut()),
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_group_lists_op<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedGroupListsOperation,
    ) -> Result<
        AnnotatedGroupListsOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedGroupListsOperation::Create(handle, _group_list) => {
                AnnotatedGroupListsOperation::Remove(*handle)
            }
            AnnotatedGroupListsOperation::Remove(handle) => {
                let group_list_id = manager
                    .get_handle_managers()
                    .group_lists
                    .get_id(*handle)
                    .ok_or(RevError::GroupListRemoved(*handle))?;
                let group_list = manager
                    .get_backend_logic()
                    .group_lists_get(group_list_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedGroupListsOperation::Create(
                    *handle,
                    convert_group_list_to_handles(group_list, manager.get_handle_managers_mut()),
                )
            }
            AnnotatedGroupListsOperation::Update(handle, _new_group_list) => {
                let group_list_id = manager
                    .get_handle_managers()
                    .group_lists
                    .get_id(*handle)
                    .ok_or(RevError::GroupListRemoved(*handle))?;
                let group_list = manager
                    .get_backend_logic()
                    .group_lists_get(group_list_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedGroupListsOperation::Update(
                    *handle,
                    convert_group_list_to_handles(group_list, manager.get_handle_managers_mut()),
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_subjects_op<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedSubjectsOperation,
    ) -> Result<AnnotatedSubjectsOperation, RevError<<T::Storage as backend::Storage>::InternalError>>
    {
        let backward = match op {
            AnnotatedSubjectsOperation::Create(handle, _subject) => {
                AnnotatedSubjectsOperation::Remove(*handle)
            }
            AnnotatedSubjectsOperation::Remove(handle) => {
                let subject_id = manager
                    .get_handle_managers()
                    .subjects
                    .get_id(*handle)
                    .ok_or(RevError::SubjectRemoved(*handle))?;
                let subject = manager
                    .get_backend_logic()
                    .subjects_get(subject_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedSubjectsOperation::Create(
                    *handle,
                    convert_subject_to_handles(subject, manager.get_handle_managers_mut()),
                )
            }
            AnnotatedSubjectsOperation::Update(handle, _new_subject) => {
                let subject_id = manager
                    .get_handle_managers()
                    .subjects
                    .get_id(*handle)
                    .ok_or(RevError::SubjectRemoved(*handle))?;
                let subject = manager
                    .get_backend_logic()
                    .subjects_get(subject_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedSubjectsOperation::Update(
                    *handle,
                    convert_subject_to_handles(subject, manager.get_handle_managers_mut()),
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_time_slots_op<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedTimeSlotsOperation,
    ) -> Result<
        AnnotatedTimeSlotsOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedTimeSlotsOperation::Create(handle, _time_slot) => {
                AnnotatedTimeSlotsOperation::Remove(*handle)
            }
            AnnotatedTimeSlotsOperation::Remove(handle) => {
                let time_slot_id = manager
                    .get_handle_managers()
                    .time_slots
                    .get_id(*handle)
                    .ok_or(RevError::TimeSlotRemoved(*handle))?;
                let time_slot = manager
                    .get_backend_logic()
                    .time_slots_get(time_slot_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedTimeSlotsOperation::Create(
                    *handle,
                    convert_time_slot_to_handles(time_slot, manager.get_handle_managers_mut()),
                )
            }
            AnnotatedTimeSlotsOperation::Update(handle, _new_time_slot) => {
                let time_slot_id = manager
                    .get_handle_managers()
                    .time_slots
                    .get_id(*handle)
                    .ok_or(RevError::TimeSlotRemoved(*handle))?;
                let time_slot = manager
                    .get_backend_logic()
                    .time_slots_get(time_slot_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedTimeSlotsOperation::Update(
                    *handle,
                    convert_time_slot_to_handles(time_slot, manager.get_handle_managers_mut()),
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_groupings_op<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedGroupingsOperation,
    ) -> Result<
        AnnotatedGroupingsOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedGroupingsOperation::Create(handle, _grouping) => {
                AnnotatedGroupingsOperation::Remove(*handle)
            }
            AnnotatedGroupingsOperation::Remove(handle) => {
                let grouping_id = manager
                    .get_handle_managers()
                    .groupings
                    .get_id(*handle)
                    .ok_or(RevError::GroupingRemoved(*handle))?;
                let grouping = manager
                    .get_backend_logic()
                    .groupings_get(grouping_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedGroupingsOperation::Create(
                    *handle,
                    convert_grouping_to_handles(grouping, manager.get_handle_managers_mut()),
                )
            }
            AnnotatedGroupingsOperation::Update(handle, _new_grouping) => {
                let grouping_id = manager
                    .get_handle_managers()
                    .groupings
                    .get_id(*handle)
                    .ok_or(RevError::GroupingRemoved(*handle))?;
                let grouping = manager
                    .get_backend_logic()
                    .groupings_get(grouping_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedGroupingsOperation::Update(
                    *handle,
                    convert_grouping_to_handles(grouping, manager.get_handle_managers_mut()),
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_grouping_incompats_op<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedGroupingIncompatsOperation,
    ) -> Result<
        AnnotatedGroupingIncompatsOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedGroupingIncompatsOperation::Create(handle, _grouping_incompat) => {
                AnnotatedGroupingIncompatsOperation::Remove(*handle)
            }
            AnnotatedGroupingIncompatsOperation::Remove(handle) => {
                let grouping_incompat_id = manager
                    .get_handle_managers()
                    .grouping_incompats
                    .get_id(*handle)
                    .ok_or(RevError::GroupingIncompatRemoved(*handle))?;
                let grouping_incompat = manager
                    .get_backend_logic()
                    .grouping_incompats_get(grouping_incompat_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedGroupingIncompatsOperation::Create(
                    *handle,
                    convert_grouping_incompat_to_handles(
                        grouping_incompat,
                        manager.get_handle_managers_mut(),
                    ),
                )
            }
            AnnotatedGroupingIncompatsOperation::Update(handle, _new_grouping_incompat) => {
                let grouping_incompat_id = manager
                    .get_handle_managers()
                    .grouping_incompats
                    .get_id(*handle)
                    .ok_or(RevError::GroupingIncompatRemoved(*handle))?;
                let grouping_incompat = manager
                    .get_backend_logic()
                    .grouping_incompats_get(grouping_incompat_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedGroupingIncompatsOperation::Update(
                    *handle,
                    convert_grouping_incompat_to_handles(
                        grouping_incompat,
                        manager.get_handle_managers_mut(),
                    ),
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_register_student_op<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedRegisterStudentOperation,
    ) -> Result<
        AnnotatedRegisterStudentOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedRegisterStudentOperation::InSubjectGroup(
                student_handle,
                subject_group_handle,
                _subject_handle,
            ) => {
                let student_id = manager
                    .get_handle_managers()
                    .students
                    .get_id(*student_handle)
                    .ok_or(RevError::StudentRemoved(*student_handle))?;
                let subject_group_id = manager
                    .get_handle_managers()
                    .subject_groups
                    .get_id(*subject_group_handle)
                    .ok_or(RevError::SubjectGroupRemoved(*subject_group_handle))?;

                let current_subject_id = manager
                    .get_backend_logic()
                    .subject_group_for_student_get(student_id, subject_group_id)
                    .await
                    .map_err(|e| match e {
                        backend::Id2Error::InternalError(int_err) => RevError::Internal(int_err),
                        backend::Id2Error::InvalidId1(_) => {
                            panic!("Student id was given by handle manager. It should be valid")
                        }
                        backend::Id2Error::InvalidId2(_) => panic!(
                            "Subject group id was given by handle manager. It should be valid"
                        ),
                    })?;

                let current_subject_handle = current_subject_id
                    .map(|x| manager.get_handle_managers_mut().subjects.get_handle(x));

                AnnotatedRegisterStudentOperation::InSubjectGroup(
                    *student_handle,
                    *subject_group_handle,
                    current_subject_handle,
                )
            }
            AnnotatedRegisterStudentOperation::InIncompat(
                student_handle,
                incompat_handle,
                _enabled,
            ) => {
                let student_id = manager
                    .get_handle_managers()
                    .students
                    .get_id(*student_handle)
                    .ok_or(RevError::StudentRemoved(*student_handle))?;
                let incompat_id = manager
                    .get_handle_managers()
                    .incompats
                    .get_id(*incompat_handle)
                    .ok_or(RevError::IncompatRemoved(*incompat_handle))?;

                let current_enabled = manager
                    .get_backend_logic()
                    .incompat_for_student_get(student_id, incompat_id)
                    .await
                    .map_err(|e| match e {
                        backend::Id2Error::InternalError(int_err) => RevError::Internal(int_err),
                        backend::Id2Error::InvalidId1(_) => {
                            panic!("Student id was given by handle manager. It should be valid")
                        }
                        backend::Id2Error::InvalidId2(_) => {
                            panic!("Incompat id was given by handle manager. It should be valid")
                        }
                    })?;

                AnnotatedRegisterStudentOperation::InIncompat(
                    *student_handle,
                    *incompat_handle,
                    current_enabled,
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_colloscopes_op<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedColloscopesOperation,
    ) -> Result<
        AnnotatedColloscopesOperation,
        RevError<<T::Storage as backend::Storage>::InternalError>,
    > {
        let backward = match op {
            AnnotatedColloscopesOperation::Create(handle, _colloscope) => {
                AnnotatedColloscopesOperation::Remove(*handle)
            }
            AnnotatedColloscopesOperation::Remove(handle) => {
                let colloscope_id = manager
                    .get_handle_managers()
                    .colloscopes
                    .get_id(*handle)
                    .ok_or(RevError::ColloscopeRemoved(*handle))?;
                let colloscope = manager
                    .get_backend_logic()
                    .colloscopes_get(colloscope_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedColloscopesOperation::Create(
                    *handle,
                    convert_colloscope_to_handles(colloscope, manager.get_handle_managers_mut()),
                )
            }
            AnnotatedColloscopesOperation::Update(handle, _new_colloscope) => {
                let colloscope_id = manager
                    .get_handle_managers()
                    .colloscopes
                    .get_id(*handle)
                    .ok_or(RevError::ColloscopeRemoved(*handle))?;
                let colloscope = manager
                    .get_backend_logic()
                    .colloscopes_get(colloscope_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                AnnotatedColloscopesOperation::Update(
                    *handle,
                    convert_colloscope_to_handles(colloscope, manager.get_handle_managers_mut()),
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_rev_op<T: ManagerInternal>(
        manager: &mut T,
        op: Operation,
    ) -> Result<ReversibleOperation, RevError<<T::Storage as backend::Storage>::InternalError>>
    {
        let forward = AnnotatedOperation::annotate(op, manager.get_handle_managers_mut());
        let backward = match &forward {
            AnnotatedOperation::GeneralData(_data) => {
                AnnotatedOperation::GeneralData(build_backward_general_data(manager).await?)
            }
            AnnotatedOperation::WeekPatterns(op) => AnnotatedOperation::WeekPatterns(
                build_backward_week_patterns_op(manager, op).await?,
            ),
            AnnotatedOperation::Teachers(op) => {
                AnnotatedOperation::Teachers(build_backward_teachers_op(manager, op).await?)
            }
            AnnotatedOperation::Students(op) => {
                AnnotatedOperation::Students(build_backward_students_op(manager, op).await?)
            }
            AnnotatedOperation::SubjectGroups(op) => AnnotatedOperation::SubjectGroups(
                build_backward_subject_groups_op(manager, op).await?,
            ),
            AnnotatedOperation::Incompats(op) => {
                AnnotatedOperation::Incompats(build_backward_incompats_op(manager, op).await?)
            }
            AnnotatedOperation::GroupLists(op) => {
                AnnotatedOperation::GroupLists(build_backward_group_lists_op(manager, op).await?)
            }
            AnnotatedOperation::Subjects(op) => {
                AnnotatedOperation::Subjects(build_backward_subjects_op(manager, op).await?)
            }
            AnnotatedOperation::TimeSlots(op) => {
                AnnotatedOperation::TimeSlots(build_backward_time_slots_op(manager, op).await?)
            }
            AnnotatedOperation::Groupings(op) => {
                AnnotatedOperation::Groupings(build_backward_groupings_op(manager, op).await?)
            }
            AnnotatedOperation::GroupingIncompats(op) => AnnotatedOperation::GroupingIncompats(
                build_backward_grouping_incompats_op(manager, op).await?,
            ),
            AnnotatedOperation::RegisterStudent(op) => AnnotatedOperation::RegisterStudent(
                build_backward_register_student_op(manager, op).await?,
            ),
            AnnotatedOperation::Colloscopes(op) => {
                AnnotatedOperation::Colloscopes(build_backward_colloscopes_op(manager, op).await?)
            }
        };
        let rev_op = ReversibleOperation { forward, backward };
        Ok(rev_op)
    }

    pub fn convert_incompat_to_handles<T: backend::Storage>(
        incompat: backend::Incompat<T::WeekPatternId>,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> backend::Incompat<WeekPatternHandle> {
        backend::Incompat {
            name: incompat.name,
            max_count: incompat.max_count,
            groups: incompat
                .groups
                .into_iter()
                .map(|g| backend::IncompatGroup {
                    slots: g
                        .slots
                        .into_iter()
                        .map(|s| backend::IncompatSlot {
                            week_pattern_id: handle_managers
                                .week_patterns
                                .get_handle(s.week_pattern_id),
                            start: s.start,
                            duration: s.duration,
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn convert_incompat_from_handles<T: backend::Storage>(
        incompat: backend::Incompat<WeekPatternHandle>,
        handle_managers: &handles::ManagerCollection<T>,
    ) -> Result<backend::Incompat<T::WeekPatternId>, backend::DataStatusWithId<WeekPatternHandle>>
    {
        Ok(backend::Incompat {
            name: incompat.name,
            max_count: incompat.max_count,
            groups: incompat
                .groups
                .into_iter()
                .map(|g| {
                    Ok(backend::IncompatGroup {
                        slots: g
                            .slots
                            .into_iter()
                            .map(|s| {
                                Ok(backend::IncompatSlot {
                                    week_pattern_id: handle_managers
                                        .week_patterns
                                        .get_id(s.week_pattern_id)
                                        .ok_or(backend::DataStatusWithId::BadCrossId(
                                            s.week_pattern_id,
                                        ))?,
                                    start: s.start,
                                    duration: s.duration,
                                })
                            })
                            .collect::<Result<_, _>>()?,
                    })
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn convert_group_list_to_handles<T: backend::Storage>(
        group_list: backend::GroupList<T::StudentId>,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> backend::GroupList<StudentHandle> {
        backend::GroupList {
            name: group_list.name,
            groups: group_list.groups,
            students_mapping: group_list
                .students_mapping
                .into_iter()
                .map(|(student_id, group)| (handle_managers.students.get_handle(student_id), group))
                .collect(),
        }
    }

    pub fn convert_group_list_from_handles<T: backend::Storage>(
        group_list: backend::GroupList<StudentHandle>,
        handle_managers: &handles::ManagerCollection<T>,
    ) -> Result<
        backend::GroupList<T::StudentId>,
        backend::DataStatusWithIdAndInvalidState<StudentHandle>,
    > {
        Ok(backend::GroupList {
            name: group_list.name,
            groups: group_list.groups,
            students_mapping: group_list
                .students_mapping
                .into_iter()
                .map(|(student_handle, group)| {
                    Ok((
                        handle_managers.students.get_id(student_handle).ok_or(
                            backend::DataStatusWithIdAndInvalidState::BadCrossId(student_handle),
                        )?,
                        group,
                    ))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn convert_subject_to_handles<T: backend::Storage>(
        subject: backend::Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle> {
        backend::Subject {
            name: subject.name,
            subject_group_id: handle_managers
                .subject_groups
                .get_handle(subject.subject_group_id),
            incompat_id: subject
                .incompat_id
                .map(|x| handle_managers.incompats.get_handle(x)),
            group_list_id: subject
                .group_list_id
                .map(|x| handle_managers.group_lists.get_handle(x)),
            duration: subject.duration,
            students_per_group: subject.students_per_group,
            period: subject.period,
            period_is_strict: subject.period_is_strict,
            is_tutorial: subject.is_tutorial,
            max_groups_per_slot: subject.max_groups_per_slot,
            balancing_requirements: subject.balancing_requirements,
        }
    }

    pub fn convert_subject_from_handles<T: backend::Storage>(
        subject: backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
        handle_managers: &handles::ManagerCollection<T>,
    ) -> Result<
        backend::Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
        backend::DataStatusWithId3<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    > {
        Ok(backend::Subject {
            name: subject.name,
            subject_group_id: handle_managers
                .subject_groups
                .get_id(subject.subject_group_id)
                .ok_or(backend::DataStatusWithId3::BadCrossId1(
                    subject.subject_group_id,
                ))?,
            incompat_id: subject
                .incompat_id
                .map(|x| {
                    handle_managers
                        .incompats
                        .get_id(x)
                        .ok_or(backend::DataStatusWithId3::BadCrossId2(x))
                })
                .transpose()?,
            group_list_id: subject
                .group_list_id
                .map(|x| {
                    handle_managers
                        .group_lists
                        .get_id(x)
                        .ok_or(backend::DataStatusWithId3::BadCrossId3(x))
                })
                .transpose()?,
            duration: subject.duration,
            students_per_group: subject.students_per_group,
            period: subject.period,
            period_is_strict: subject.period_is_strict,
            is_tutorial: subject.is_tutorial,
            max_groups_per_slot: subject.max_groups_per_slot,
            balancing_requirements: subject.balancing_requirements,
        })
    }

    pub fn convert_time_slot_to_handles<T: backend::Storage>(
        time_slot: backend::TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle> {
        backend::TimeSlot {
            subject_id: handle_managers.subjects.get_handle(time_slot.subject_id),
            teacher_id: handle_managers.teachers.get_handle(time_slot.teacher_id),
            start: time_slot.start,
            week_pattern_id: handle_managers
                .week_patterns
                .get_handle(time_slot.week_pattern_id),
            room: time_slot.room,
            cost: time_slot.cost,
        }
    }

    pub fn convert_time_slot_from_handles<T: backend::Storage>(
        time_slot: backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
        handle_managers: &handles::ManagerCollection<T>,
    ) -> Result<
        backend::TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
        backend::DataStatusWithId3<SubjectHandle, TeacherHandle, WeekPatternHandle>,
    > {
        Ok(backend::TimeSlot {
            subject_id: handle_managers
                .subjects
                .get_id(time_slot.subject_id)
                .ok_or(backend::DataStatusWithId3::BadCrossId1(
                    time_slot.subject_id,
                ))?,
            teacher_id: handle_managers
                .teachers
                .get_id(time_slot.teacher_id)
                .ok_or(backend::DataStatusWithId3::BadCrossId2(
                    time_slot.teacher_id,
                ))?,
            start: time_slot.start,
            week_pattern_id: handle_managers
                .week_patterns
                .get_id(time_slot.week_pattern_id)
                .ok_or(backend::DataStatusWithId3::BadCrossId3(
                    time_slot.week_pattern_id,
                ))?,
            room: time_slot.room,
            cost: time_slot.cost,
        })
    }

    pub fn convert_grouping_to_handles<T: backend::Storage>(
        grouping: backend::Grouping<T::TimeSlotId>,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> backend::Grouping<TimeSlotHandle> {
        backend::Grouping {
            name: grouping.name,
            slots: grouping
                .slots
                .into_iter()
                .map(|x| handle_managers.time_slots.get_handle(x))
                .collect(),
        }
    }

    pub fn convert_grouping_from_handles<T: backend::Storage>(
        grouping: backend::Grouping<TimeSlotHandle>,
        handle_managers: &handles::ManagerCollection<T>,
    ) -> Result<backend::Grouping<T::TimeSlotId>, backend::DataStatusWithId<TimeSlotHandle>> {
        Ok(backend::Grouping {
            name: grouping.name,
            slots: grouping
                .slots
                .into_iter()
                .map(|x| {
                    handle_managers
                        .time_slots
                        .get_id(x)
                        .ok_or(backend::DataStatusWithId::BadCrossId(x))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn convert_grouping_incompat_to_handles<T: backend::Storage>(
        grouping_incompat: backend::GroupingIncompat<T::GroupingId>,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> backend::GroupingIncompat<GroupingHandle> {
        backend::GroupingIncompat {
            max_count: grouping_incompat.max_count,
            groupings: grouping_incompat
                .groupings
                .into_iter()
                .map(|x| handle_managers.groupings.get_handle(x))
                .collect(),
        }
    }

    pub fn convert_grouping_incompat_from_handles<T: backend::Storage>(
        grouping_incompat: backend::GroupingIncompat<GroupingHandle>,
        handle_managers: &handles::ManagerCollection<T>,
    ) -> Result<backend::GroupingIncompat<T::GroupingId>, backend::DataStatusWithId<GroupingHandle>>
    {
        Ok(backend::GroupingIncompat {
            max_count: grouping_incompat.max_count,
            groupings: grouping_incompat
                .groupings
                .into_iter()
                .map(|x| {
                    handle_managers
                        .groupings
                        .get_id(x)
                        .ok_or(backend::DataStatusWithId::BadCrossId(x))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    fn convert_collo_subject_to_handles<T: backend::Storage>(
        collo_subject: backend::ColloscopeSubject<T::TeacherId, T::StudentId>,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> backend::ColloscopeSubject<TeacherHandle, StudentHandle> {
        backend::ColloscopeSubject {
            time_slots: collo_subject
                .time_slots
                .into_iter()
                .map(|time_slot| backend::ColloscopeTimeSlot {
                    teacher_id: handle_managers.teachers.get_handle(time_slot.teacher_id),
                    start: time_slot.start,
                    room: time_slot.room,
                    group_assignments: time_slot.group_assignments,
                })
                .collect(),
            group_list: backend::ColloscopeGroupList {
                name: collo_subject.group_list.name,
                groups: collo_subject.group_list.groups,
                students_mapping: collo_subject
                    .group_list
                    .students_mapping
                    .into_iter()
                    .map(|(student_id, group)| {
                        (handle_managers.students.get_handle(student_id), group)
                    })
                    .collect(),
            },
        }
    }

    pub fn convert_colloscope_to_handles<T: backend::Storage>(
        colloscope: backend::Colloscope<T::TeacherId, T::SubjectId, T::StudentId>,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle> {
        backend::Colloscope {
            name: colloscope.name,
            subjects: colloscope
                .subjects
                .into_iter()
                .map(|(subject_id, collo_subject)| {
                    (
                        handle_managers.subjects.get_handle(subject_id),
                        convert_collo_subject_to_handles(collo_subject, handle_managers),
                    )
                })
                .collect(),
        }
    }

    fn convert_collo_subject_from_handles<T: backend::Storage>(
        collo_subject: backend::ColloscopeSubject<TeacherHandle, StudentHandle>,
        handle_managers: &handles::ManagerCollection<T>,
    ) -> Result<
        backend::ColloscopeSubject<T::TeacherId, T::StudentId>,
        backend::DataStatusWithId2<TeacherHandle, StudentHandle>,
    > {
        Ok(backend::ColloscopeSubject {
            time_slots: collo_subject
                .time_slots
                .into_iter()
                .map(|time_slot| {
                    Ok(backend::ColloscopeTimeSlot {
                        teacher_id: handle_managers
                            .teachers
                            .get_id(time_slot.teacher_id)
                            .ok_or(backend::DataStatusWithId2::BadCrossId1(
                                time_slot.teacher_id,
                            ))?,
                        start: time_slot.start,
                        room: time_slot.room,
                        group_assignments: time_slot.group_assignments,
                    })
                })
                .collect::<Result<_, _>>()?,
            group_list: backend::ColloscopeGroupList {
                name: collo_subject.group_list.name,
                groups: collo_subject.group_list.groups,
                students_mapping: collo_subject
                    .group_list
                    .students_mapping
                    .into_iter()
                    .map(|(student_handle, group)| {
                        Ok((
                            handle_managers
                                .students
                                .get_id(student_handle)
                                .ok_or(backend::DataStatusWithId2::BadCrossId2(student_handle))?,
                            group,
                        ))
                    })
                    .collect::<Result<_, _>>()?,
            },
        })
    }

    pub fn convert_colloscope_from_handles<T: backend::Storage>(
        colloscope: backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
        handle_managers: &handles::ManagerCollection<T>,
    ) -> Result<
        backend::Colloscope<T::TeacherId, T::SubjectId, T::StudentId>,
        backend::DataStatusWithId3<TeacherHandle, SubjectHandle, StudentHandle>,
    > {
        Ok(backend::Colloscope {
            name: colloscope.name,
            subjects: colloscope
                .subjects
                .into_iter()
                .map(|(subject_handle, collo_subject)| {
                    Ok((
                        handle_managers
                            .subjects
                            .get_id(subject_handle)
                            .ok_or(backend::DataStatusWithId3::BadCrossId2(subject_handle))?,
                        convert_collo_subject_from_handles(collo_subject, handle_managers)
                            .map_err(|e| match e {
                                backend::DataStatusWithId2::Ok => panic!("not an error"),
                                backend::DataStatusWithId2::BadCrossId1(id) => {
                                    backend::DataStatusWithId3::BadCrossId1(id)
                                }
                                backend::DataStatusWithId2::BadCrossId2(id) => {
                                    backend::DataStatusWithId3::BadCrossId3(id)
                                }
                            })?,
                    ))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}
