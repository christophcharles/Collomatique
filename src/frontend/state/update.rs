use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Error)]
pub enum UpdateError<IntError: std::error::Error + std::fmt::Debug> {
    #[error("Error in storage backend: {0:?}")]
    Internal(#[from] IntError),
    #[error("Cannot set week_count: some week_patterns must be truncated")]
    WeekPatternsNeedTruncating(Vec<WeekPatternHandle>),
    #[error("Cannot set interrogations_per_week range: the range must be non-empty")]
    InterrogationsPerWeekRangeIsEmpty,
    #[error("Cannot add the week pattern: it references weeks beyond week_count")]
    WeekNumberTooBig(u32),
    #[error("Cannot remove the week pattern: it is referenced by the database")]
    WeekPatternDependanciesRemaining(
        Vec<backend::WeekPatternDependancy<IncompatHandle, TimeSlotHandle>>,
    ),
}

use backend::{IdError, WeekPatternDependancy};

#[trait_variant::make(Send)]
pub trait Manager: ManagerInternal {
    fn get_backend_logic(&self) -> &backend::Logic<Self::Storage>;

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

    async fn apply(
        &mut self,
        op: Operation,
    ) -> Result<(), UpdateError<<Self::Storage as backend::Storage>::InternalError>>;
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;
    async fn undo(
        &mut self,
    ) -> Result<(), UndoError<<Self::Storage as backend::Storage>::InternalError>>;
    async fn redo(
        &mut self,
    ) -> Result<(), RedoError<<Self::Storage as backend::Storage>::InternalError>>;
}

impl<T: ManagerInternal> Manager for T {
    fn get_backend_logic(&self) -> &backend::Logic<T::Storage> {
        <Self as ManagerInternal>::get_backend_logic(self)
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
                    IdError::InvalidId(id) => panic!(
                        "Week pattern id {:?} should be valid as it was computed from handle {:?}",
                        id, handle
                    ),
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
                    IdError::InvalidId(id) => panic!(
                        "Week pattern id {:?} should be valid as it was computed from handle {:?}",
                        id, handle
                    ),
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

    fn apply(
        &mut self,
        op: Operation,
    ) -> impl core::future::Future<
        Output = Result<(), UpdateError<<Self::Storage as backend::Storage>::InternalError>>,
    > + Send {
        async {
            let rev_op = private::build_rev_op(self, op).await?;

            private::update_internal_state(self, &rev_op.forward).await?;

            let aggregated_ops = AggregatedOperations::new(vec![rev_op]);
            self.get_history_mut().apply(aggregated_ops);

            Ok(())
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
}

pub(super) mod private {
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

    pub async fn update_general_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedGeneralOperation,
    ) -> Result<(), UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedGeneralOperation::SetWeekCount(new_week_count) => {
                let mut general_data = manager.get_backend_logic().general_data_get().await?;
                general_data.week_count = *new_week_count;
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
                        backend::CheckedError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                    })?;
                Ok(())
            }
            AnnotatedGeneralOperation::SetMaxInterrogationsPerDay(
                new_max_interrogations_per_day,
            ) => {
                let mut general_data = manager.get_backend_logic().general_data_get().await?;
                general_data.max_interrogations_per_day = *new_max_interrogations_per_day;
                manager.get_backend_logic_mut()
                    .general_data_set(&general_data)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedError::CheckFailed(_data) => {
                            panic!("General data should be valid as modifying max_interrogations_per_day has no dependancy")
                        }
                        backend::CheckedError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                    })?;
                Ok(())
            }
            AnnotatedGeneralOperation::SetInterrogationsPerWeekRange(
                new_interrogations_per_week,
            ) => {
                if let Some(range) = new_interrogations_per_week {
                    if range.is_empty() {
                        return Err(UpdateError::InterrogationsPerWeekRangeIsEmpty);
                    }
                }
                let mut general_data = manager.get_backend_logic().general_data_get().await?;
                general_data.interrogations_per_week = new_interrogations_per_week.clone();
                manager.get_backend_logic_mut()
                    .general_data_set(&general_data)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedError::CheckFailed(_data) => {
                            panic!("General data should be valid as modifying interrogations_per_week has no dependancy")
                        }
                        backend::CheckedError::InternalError(int_error) => {
                            UpdateError::Internal(int_error)
                        }
                    })?;
                Ok(())
            }
        }
    }

    pub async fn update_week_patterns_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedWeekPatternsOperation,
    ) -> Result<(), UpdateError<<T::Storage as backend::Storage>::InternalError>> {
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
                Ok(())
            }
            AnnotatedWeekPatternsOperation::Remove(week_pattern_handle) => {
                let week_pattern_id = manager
                    .get_handle_managers()
                    .week_patterns
                    .get_id(*week_pattern_handle)
                    .expect("week pattern to remove should exist");
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
                Ok(())
            }
            AnnotatedWeekPatternsOperation::Update(week_pattern_handle, pattern) => {
                let week_pattern_id = manager
                    .get_handle_managers()
                    .week_patterns
                    .get_id(*week_pattern_handle)
                    .expect("week pattern to update should exist");
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
                Ok(())
            }
        }
    }

    pub async fn update_internal_state<T: ManagerInternal>(
        manager: &mut T,
        op: &AnnotatedOperation,
    ) -> Result<(), UpdateError<<T::Storage as backend::Storage>::InternalError>> {
        match op {
            AnnotatedOperation::General(op) => update_general_state(manager, op).await?,
            AnnotatedOperation::WeekPatterns(op) => update_week_patterns_state(manager, op).await?,
        }
        Ok(())
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

    pub async fn build_backward_general_op<T: ManagerInternal>(
        manager: &T,
        op: &AnnotatedGeneralOperation,
    ) -> Result<AnnotatedGeneralOperation, <T::Storage as backend::Storage>::InternalError> {
        let backward = match op {
            AnnotatedGeneralOperation::SetWeekCount(_new_week_count) => {
                let general_data = manager.get_backend_logic().general_data_get().await?;
                AnnotatedGeneralOperation::SetWeekCount(general_data.week_count)
            }
            AnnotatedGeneralOperation::SetMaxInterrogationsPerDay(_max_interrogations_per_day) => {
                let general_data = manager.get_backend_logic().general_data_get().await?;
                AnnotatedGeneralOperation::SetMaxInterrogationsPerDay(
                    general_data.max_interrogations_per_day,
                )
            }
            AnnotatedGeneralOperation::SetInterrogationsPerWeekRange(_interrogations_per_week) => {
                let general_data = manager.get_backend_logic().general_data_get().await?;
                AnnotatedGeneralOperation::SetInterrogationsPerWeekRange(
                    general_data.interrogations_per_week,
                )
            }
        };
        Ok(backward)
    }

    pub async fn build_backward_week_patterns_op<T: ManagerInternal>(
        manager: &T,
        op: &AnnotatedWeekPatternsOperation,
    ) -> Result<AnnotatedWeekPatternsOperation, <T::Storage as backend::Storage>::InternalError>
    {
        let backward = match op {
            AnnotatedWeekPatternsOperation::Create(handle, _pattern) => {
                AnnotatedWeekPatternsOperation::Remove(*handle)
            }
            AnnotatedWeekPatternsOperation::Remove(handle) => {
                let week_pattern_id = manager
                    .get_handle_managers()
                    .week_patterns
                    .get_id(*handle)
                    .expect("week pattern to remove should exist");
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
                    .expect("week pattern to update should exist");
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

    pub async fn build_rev_op<T: ManagerInternal>(
        manager: &mut T,
        op: Operation,
    ) -> Result<ReversibleOperation, <T::Storage as backend::Storage>::InternalError> {
        let forward = AnnotatedOperation::annotate(op, manager.get_handle_managers_mut());
        let backward = match &forward {
            AnnotatedOperation::General(op) => {
                AnnotatedOperation::General(build_backward_general_op(manager, op).await?)
            }
            AnnotatedOperation::WeekPatterns(op) => AnnotatedOperation::WeekPatterns(
                build_backward_week_patterns_op(manager, op).await?,
            ),
        };
        let rev_op = ReversibleOperation { forward, backward };
        Ok(rev_op)
    }
}
