use std::num::NonZeroU32;
use thiserror::Error;

pub mod handles;
mod history;

use crate::backend;
use history::{AnnotatedOperation, ModificationHistory, ReversibleOperation};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    GeneralSetWeekCount(NonZeroU32),
    GeneralSetMaxInterrogationsPerDay(Option<NonZeroU32>),
    GeneralSetInterrogationsPerWeekRange(Option<std::ops::Range<u32>>),

    WeekPatternsAdd(backend::WeekPattern),
    WeekPatternsRemove(handles::WeekPatternHandle),
}

#[derive(Debug)]
pub struct AppState<T: backend::Storage> {
    backend_logic: backend::Logic<T>,
    mod_history: ModificationHistory,
    handle_managers: handles::ManagerCollection<T>,
}

#[derive(Debug, Clone, Error)]
pub enum UndoError<T: std::fmt::Debug + std::error::Error> {
    #[error("Operation history is depleted. Cannot undo any other operation.")]
    HistoryDepleted,
    #[error("Error in storage backend: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Debug, Clone, Error)]
pub enum RedoError<T: std::fmt::Debug + std::error::Error> {
    #[error("Operation history completly rewounded. Cannot redo any other operation.")]
    HistoryFullyRewounded,
    #[error("Error in storage backend: {0:?}")]
    InternalError(#[from] T),
}

impl<T: backend::Storage> AppState<T> {
    pub fn new(backend_logic: backend::Logic<T>) -> Self {
        AppState {
            backend_logic,
            mod_history: ModificationHistory::new(),
            handle_managers: handles::ManagerCollection::new(),
        }
    }

    pub fn with_max_history_size(
        backend_logic: backend::Logic<T>,
        max_history_size: Option<usize>,
    ) -> Self {
        AppState {
            backend_logic,
            mod_history: ModificationHistory::with_max_history_size(max_history_size),
            handle_managers: handles::ManagerCollection::new(),
        }
    }

    pub fn get_max_history_size(&self) -> Option<usize> {
        self.mod_history.get_max_history_size()
    }

    pub fn get_backend_logic(&self) -> &backend::Logic<T> {
        &self.backend_logic
    }

    pub fn get_week_pattern_handle(&mut self, id: T::WeekPatternId) -> handles::WeekPatternHandle {
        self.handle_managers.week_patterns.get_handle(id)
    }

    pub fn set_max_history_size(&mut self, max_history_size: Option<usize>) {
        self.mod_history.set_max_history_size(max_history_size);
    }

    pub async fn apply(&mut self, op: Operation) -> Result<(), UpdateError<T>> {
        let rev_op = self.build_rev_op(op).await?;

        self.update_internal_state(&rev_op.forward).await?;
        self.mod_history.apply(rev_op);

        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.mod_history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.mod_history.can_redo()
    }

    pub async fn undo(&mut self) -> Result<(), UndoError<T::InternalError>> {
        match self.mod_history.undo() {
            Some(op) => {
                self.update_internal_state(&op).await.map_err(
                    |e| match e {
                        UpdateError::InternalError(int_err) => UndoError::InternalError(int_err),
                        _ => panic!("Data should be consistent as it was automatically build from previous state"),
                    }
                )?;
                Ok(())
            }
            None => Err(UndoError::HistoryDepleted),
        }
    }

    pub async fn redo(&mut self) -> Result<(), RedoError<T::InternalError>> {
        match self.mod_history.redo() {
            Some(op) => {
                self.update_internal_state(&op).await.map_err(
                    |e| match e {
                        UpdateError::InternalError(int_err) => RedoError::InternalError(int_err),
                        _ => panic!("Data should be consistent as it was automatically build from previous state"),
                    }
                )?;
                Ok(())
            }
            None => Err(RedoError::HistoryFullyRewounded),
        }
    }
}

#[derive(Debug, Error)]
pub enum UpdateError<T: backend::Storage, IntError = <T as backend::Storage>::InternalError>
where
    IntError: std::fmt::Debug + std::error::Error,
{
    #[error("Error in storage backend: {0:?}")]
    InternalError(#[from] IntError),
    #[error("Cannot set week_count: some week_patterns must be truncated")]
    CannotSetWeekCountWeekPatternsNeedTruncating(Vec<T::WeekPatternId>),
    #[error("Cannot set interrogations_per_week range: the range must be non-empty")]
    CannotSetInterrogationsPerWeekRangeIsEmpty,
    #[error("Cannot add the week pattern: it references weeks beyond week_count")]
    CannotAddWeekPatternWeekNumberTooBig(u32),
    #[error("Cannot remove the week pattern: it is referenced by the database")]
    CannotRemoveWeekPatternBecauseOfDependancies(
        Vec<backend::WeekPatternDependancy<T::IncompatId, T::TimeSlotId>>,
    ),
}

impl<T: backend::Storage> AppState<T> {
    async fn build_rev_op(
        &mut self,
        op: Operation,
    ) -> Result<ReversibleOperation, T::InternalError> {
        let forward = AnnotatedOperation::annotate(op, &mut self.handle_managers);
        match forward {
            AnnotatedOperation::GeneralSetWeekCount(_new_week_count) => {
                let general_data = self.backend_logic.general_data_get().await?;

                let rev_op = ReversibleOperation {
                    forward,
                    backward: AnnotatedOperation::GeneralSetWeekCount(general_data.week_count),
                };

                Ok(rev_op)
            }
            AnnotatedOperation::GeneralSetMaxInterrogationsPerDay(_max_interrogations_per_day) => {
                let general_data = self.backend_logic.general_data_get().await?;

                let rev_op = ReversibleOperation {
                    forward,
                    backward: AnnotatedOperation::GeneralSetMaxInterrogationsPerDay(
                        general_data.max_interrogations_per_day,
                    ),
                };

                Ok(rev_op)
            }
            AnnotatedOperation::GeneralSetInterrogationsPerWeekRange(
                ref _interrogations_per_week,
            ) => {
                let general_data = self.backend_logic.general_data_get().await?;

                let rev_op = ReversibleOperation {
                    forward,
                    backward: AnnotatedOperation::GeneralSetInterrogationsPerWeekRange(
                        general_data.interrogations_per_week,
                    ),
                };

                Ok(rev_op)
            }
            AnnotatedOperation::WeekPatternsAdd(handle, ref _pattern) => Ok(ReversibleOperation {
                forward,
                backward: AnnotatedOperation::WeekPatternsRemove(handle),
            }),
            AnnotatedOperation::WeekPatternsRemove(handle) => {
                let week_pattern_id = self
                    .handle_managers
                    .week_patterns
                    .get_id(handle)
                    .expect("week pattern to remove should exist");
                let pattern = self
                    .backend_logic
                    .week_patterns_get(week_pattern_id)
                    .await
                    .map_err(|e| match e {
                        backend::IdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::IdError::InternalError(int_err) => int_err,
                    })?;
                let rev_op = ReversibleOperation {
                    forward,
                    backward: AnnotatedOperation::WeekPatternsAdd(handle, pattern),
                };
                Ok(rev_op)
            }
        }
    }

    async fn update_internal_state(
        &mut self,
        op: &AnnotatedOperation,
    ) -> Result<(), UpdateError<T>> {
        match op {
            AnnotatedOperation::GeneralSetWeekCount(new_week_count) => {
                let mut general_data = self.backend_logic.general_data_get().await?;
                general_data.week_count = *new_week_count;
                self.backend_logic
                    .general_data_set(&general_data)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedError::CheckFailed(data) => {
                            UpdateError::CannotSetWeekCountWeekPatternsNeedTruncating(data)
                        }
                        backend::CheckedError::InternalError(int_error) => {
                            UpdateError::InternalError(int_error)
                        }
                    })?;
                Ok(())
            }
            AnnotatedOperation::GeneralSetMaxInterrogationsPerDay(
                new_max_interrogations_per_day,
            ) => {
                let mut general_data = self.backend_logic.general_data_get().await?;
                general_data.max_interrogations_per_day = *new_max_interrogations_per_day;
                self.backend_logic
                    .general_data_set(&general_data)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedError::CheckFailed(_data) => {
                            panic!("General data should be valid as modifying max_interrogations_per_day has no dependancy")
                        }
                        backend::CheckedError::InternalError(int_error) => {
                            UpdateError::InternalError(int_error)
                        }
                    })?;
                Ok(())
            }
            AnnotatedOperation::GeneralSetInterrogationsPerWeekRange(
                new_interrogations_per_week,
            ) => {
                if let Some(range) = new_interrogations_per_week {
                    if range.is_empty() {
                        return Err(UpdateError::CannotSetInterrogationsPerWeekRangeIsEmpty);
                    }
                }
                let mut general_data = self.backend_logic.general_data_get().await?;
                general_data.interrogations_per_week = new_interrogations_per_week.clone();
                self.backend_logic
                    .general_data_set(&general_data)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedError::CheckFailed(_data) => {
                            panic!("General data should be valid as modifying interrogations_per_week has no dependancy")
                        }
                        backend::CheckedError::InternalError(int_error) => {
                            UpdateError::InternalError(int_error)
                        }
                    })?;
                Ok(())
            }
            AnnotatedOperation::WeekPatternsAdd(week_pattern_handle, pattern) => {
                let new_id = self
                    .backend_logic
                    .week_patterns_add(pattern)
                    .await
                    .map_err(|e| match e {
                        backend::WeekPatternError::WeekNumberTooBig(week_number) => {
                            UpdateError::CannotAddWeekPatternWeekNumberTooBig(week_number)
                        }
                        backend::WeekPatternError::InternalError(int_error) => {
                            UpdateError::InternalError(int_error)
                        }
                    })?;
                self.handle_managers
                    .week_patterns
                    .update_handle(*week_pattern_handle, Some(new_id));
                Ok(())
            }
            AnnotatedOperation::WeekPatternsRemove(week_pattern_handle) => {
                let week_pattern_id = self
                    .handle_managers
                    .week_patterns
                    .get_id(*week_pattern_handle)
                    .expect("week pattern to remove should exist");
                self.backend_logic
                    .week_patterns_remove(week_pattern_id)
                    .await
                    .map_err(|e| match e {
                        backend::CheckedIdError::InvalidId(id) => {
                            panic!("id ({:?}) from the handle manager should be valid", id)
                        }
                        backend::CheckedIdError::InternalError(int_err) => {
                            UpdateError::InternalError(int_err)
                        }
                        backend::CheckedIdError::CheckFailed(dependancies) => {
                            UpdateError::CannotRemoveWeekPatternBecauseOfDependancies(dependancies)
                        }
                    })?;
                self.handle_managers
                    .week_patterns
                    .update_handle(*week_pattern_handle, None);
                Ok(())
            }
        }
    }
}
