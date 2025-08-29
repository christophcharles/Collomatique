use std::num::NonZeroU32;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    GeneralSetWeekCount(NonZeroU32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReversibleOperation {
    forward: Operation,
    backward: Operation,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct ModificationHistory {
    history: std::collections::VecDeque<ReversibleOperation>,
    history_pointer: usize,
    max_history_size: Option<usize>,
}

impl ModificationHistory {
    fn truncate_history_as_needed(&mut self) {
        if let Some(max_hist_size) = self.max_history_size {
            if max_hist_size >= self.history.len() {
                return;
            }

            // Try to keep undo history as a priority (rather than redo history)
            // So we remove the beginning of the queue only if we really can't keep it
            if self.history_pointer > max_hist_size {
                let split_point = self.history_pointer - max_hist_size;
                let new_history = self.history.split_off(split_point);
                self.history = new_history;

                self.history_pointer = max_hist_size;
            }

            self.history.truncate(max_hist_size);
        }
    }
}

impl ModificationHistory {
    fn new() -> Self {
        ModificationHistory {
            history: std::collections::VecDeque::new(),
            history_pointer: 0,
            max_history_size: None,
        }
    }

    fn with_max_history_size(max_history_size: Option<usize>) -> Self {
        ModificationHistory {
            history: std::collections::VecDeque::new(),
            history_pointer: 0,
            max_history_size,
        }
    }

    fn get_max_history_size(&self) -> Option<usize> {
        self.max_history_size
    }

    fn set_max_history_size(&mut self, max_history_size: Option<usize>) {
        self.max_history_size = max_history_size;

        self.truncate_history_as_needed();
    }

    fn apply(&mut self, reversible_op: ReversibleOperation) {
        self.history.truncate(self.history_pointer);

        self.history_pointer += 1;
        self.history.push_back(reversible_op);

        self.truncate_history_as_needed();
    }

    fn can_undo(&self) -> bool {
        self.history_pointer > 0
    }

    fn can_redo(&self) -> bool {
        self.history_pointer < self.history.len()
    }

    fn undo(&mut self) -> Option<Operation> {
        if !self.can_undo() {
            return None;
        }

        self.history_pointer -= 1;

        assert!(self.history_pointer < self.history.len());

        let last_op = self.history[self.history_pointer].clone();

        Some(last_op.backward)
    }

    fn redo(&mut self) -> Option<Operation> {
        if !self.can_redo() {
            return None;
        }

        let new_op = self.history[self.history_pointer].clone();
        self.history_pointer += 1;

        Some(new_op.forward)
    }
}

use crate::backend;

#[derive(Debug)]
pub struct AppState<T: backend::Storage> {
    backend_logic: backend::Logic<T>,
    mod_history: ModificationHistory,
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
        }
    }

    pub fn with_max_history_size(
        backend_logic: backend::Logic<T>,
        max_history_size: Option<usize>,
    ) -> Self {
        AppState {
            backend_logic,
            mod_history: ModificationHistory::with_max_history_size(max_history_size),
        }
    }

    pub fn get_max_history_size(&self) -> Option<usize> {
        self.mod_history.get_max_history_size()
    }

    pub fn get_backend_logic(&self) -> &backend::Logic<T> {
        &self.backend_logic
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
}

impl<T: backend::Storage> AppState<T> {
    async fn build_rev_op(&self, op: Operation) -> Result<ReversibleOperation, T::InternalError> {
        match op {
            Operation::GeneralSetWeekCount(_new_week_count) => {
                let general_data = self.backend_logic.general_data_get().await?;

                let rev_op = ReversibleOperation {
                    forward: op,
                    backward: Operation::GeneralSetWeekCount(general_data.week_count),
                };

                Ok(rev_op)
            }
        }
    }

    async fn update_internal_state(&mut self, op: &Operation) -> Result<(), UpdateError<T>> {
        match op {
            Operation::GeneralSetWeekCount(new_week_count) => {
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
        }
    }
}
