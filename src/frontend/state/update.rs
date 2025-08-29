use self::backend::SubjectGroupDependancy;

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
    TeacherDependanciesRemaining(Vec<TimeSlotHandle>),
    #[error("Student corresponding to handle {0:?} was previously removed")]
    StudentRemoved(StudentHandle),
    #[error("Cannot remove student: it is referenced by the database")]
    StudentDependanciesRemaining(Vec<GroupListHandle>),
    #[error("Subject group corresponding to handle {0:?} was previously removed")]
    SubjectGroupRemoved(SubjectGroupHandle),
    #[error("Cannot remove subject group: it is referenced by the database")]
    SubjectGroupDependanciesRemaining(
        Vec<backend::SubjectGroupDependancy<SubjectHandle, StudentHandle>>,
    ),
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
}

impl<IntError: std::error::Error> From<RevError<IntError>> for UpdateError<IntError> {
    fn from(value: RevError<IntError>) -> Self {
        match value {
            RevError::Internal(int_error) => UpdateError::Internal(int_error),
            RevError::WeekPatternRemoved(handle) => UpdateError::WeekPatternRemoved(handle),
            RevError::TeacherRemoved(handle) => UpdateError::TeacherRemoved(handle),
            RevError::StudentRemoved(handle) => UpdateError::StudentRemoved(handle),
            RevError::SubjectGroupRemoved(handle) => UpdateError::SubjectGroupRemoved(handle),
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
}

use backend::{IdError, WeekPatternDependancy, WeekPatternError};

#[trait_variant::make(Send)]
pub trait Manager: ManagerInternal {
    fn get_logic(&self) -> &backend::Logic<Self::Storage>;

    async fn general_data_get(
        &self,
    ) -> Result<backend::GeneralData, <Self::Storage as backend::Storage>::InternalError>;
    async fn week_patterns_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<WeekPatternHandle, backend::WeekPattern>,
        <Self::Storage as backend::Storage>::InternalError,
    >;
    async fn week_patterns_get(
        &self,
        handle: WeekPatternHandle,
    ) -> Result<
        backend::WeekPattern,
        IdError<<Self::Storage as backend::Storage>::InternalError, WeekPatternHandle>,
    >;
    async fn week_patterns_check_can_remove(
        &mut self,
        handle: WeekPatternHandle,
    ) -> Result<
        Vec<WeekPatternDependancy<IncompatHandle, TimeSlotHandle>>,
        IdError<<Self::Storage as backend::Storage>::InternalError, WeekPatternHandle>,
    >;
    async fn week_patterns_check_data(
        &self,
        pattern: &backend::WeekPattern,
    ) -> std::result::Result<(), WeekPatternError<<Self::Storage as backend::Storage>::InternalError>>;
    async fn teachers_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<TeacherHandle, backend::Teacher>,
        <Self::Storage as backend::Storage>::InternalError,
    >;
    async fn teachers_get(
        &self,
        handle: TeacherHandle,
    ) -> Result<
        backend::Teacher,
        IdError<<Self::Storage as backend::Storage>::InternalError, TeacherHandle>,
    >;
    async fn teachers_check_can_remove(
        &mut self,
        handle: TeacherHandle,
    ) -> Result<
        Vec<TimeSlotHandle>,
        IdError<<Self::Storage as backend::Storage>::InternalError, TeacherHandle>,
    >;
    async fn students_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<StudentHandle, backend::Student>,
        <Self::Storage as backend::Storage>::InternalError,
    >;
    async fn students_get(
        &self,
        handle: StudentHandle,
    ) -> Result<
        backend::Student,
        IdError<<Self::Storage as backend::Storage>::InternalError, StudentHandle>,
    >;
    async fn students_check_can_remove(
        &mut self,
        handle: StudentHandle,
    ) -> Result<
        Vec<GroupListHandle>,
        IdError<<Self::Storage as backend::Storage>::InternalError, StudentHandle>,
    >;
    async fn subject_groups_get_all(
        &mut self,
    ) -> Result<
        BTreeMap<SubjectGroupHandle, backend::SubjectGroup>,
        <Self::Storage as backend::Storage>::InternalError,
    >;
    async fn subject_groups_get(
        &self,
        handle: SubjectGroupHandle,
    ) -> Result<
        backend::SubjectGroup,
        IdError<<Self::Storage as backend::Storage>::InternalError, SubjectGroupHandle>,
    >;
    async fn subject_groups_check_can_remove(
        &mut self,
        handle: SubjectGroupHandle,
    ) -> Result<
        Vec<SubjectGroupDependancy<SubjectHandle, StudentHandle>>,
        IdError<<Self::Storage as backend::Storage>::InternalError, SubjectGroupHandle>,
    >;

    async fn apply(
        &mut self,
        op: Operation,
    ) -> Result<ReturnHandle, UpdateError<<Self::Storage as backend::Storage>::InternalError>>;
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
            Vec<TimeSlotHandle>,
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

            let teacher_deps = teacher_deps_backend
                .into_iter()
                .map(|dep| time_slot_handle_manager.get_handle(dep))
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
            Vec<GroupListHandle>,
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

            let student_deps = student_deps_backend
                .into_iter()
                .map(|dep| group_list_handle_manager.get_handle(dep))
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
    use self::backend::SubjectGroupDependancy;

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
                                .map(|dep| {
                                    manager.get_handle_managers_mut().time_slots.get_handle(dep)
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
                                .map(|dep| {
                                    manager
                                        .get_handle_managers_mut()
                                        .group_lists
                                        .get_handle(dep)
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
        };
        let rev_op = ReversibleOperation { forward, backward };
        Ok(rev_op)
    }
}
