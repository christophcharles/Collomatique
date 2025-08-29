use thiserror::Error;

mod handles;
mod history;
pub mod update;

use crate::backend;
use history::{
    AnnotatedOperation, AnnotatedStudentsOperation, AnnotatedSubjectGroupsOperation,
    AnnotatedTeachersOperation, AnnotatedWeekPatternsOperation, ModificationHistory,
    ReversibleOperation,
};
use update::private::ManagerInternal;

pub use handles::{
    GroupListHandle, GroupingHandle, GroupingIncompatHandle, IncompatHandle, StudentHandle,
    SubjectGroupHandle, SubjectHandle, TeacherHandle, TimeSlotHandle, WeekPatternHandle,
};
pub use update::{Manager, UpdateError};

use self::history::AggregatedOperations;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    GeneralData(backend::GeneralData),
    WeekPatterns(WeekPatternsOperation),
    Teachers(TeachersOperation),
    Students(StudentsOperation),
    SubjectGroups(SubjectGroupsOperation),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WeekPatternsOperation {
    Create(backend::WeekPattern),
    Remove(WeekPatternHandle),
    Update(WeekPatternHandle, backend::WeekPattern),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TeachersOperation {
    Create(backend::Teacher),
    Remove(TeacherHandle),
    Update(TeacherHandle, backend::Teacher),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudentsOperation {
    Create(backend::Student),
    Remove(StudentHandle),
    Update(StudentHandle, backend::Student),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubjectGroupsOperation {
    Create(backend::SubjectGroup),
    Remove(SubjectGroupHandle),
    Update(SubjectGroupHandle, backend::SubjectGroup),
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

    pub fn set_max_history_size(&mut self, max_history_size: Option<usize>) {
        self.mod_history.set_max_history_size(max_history_size);
    }
}

impl<S: backend::Storage> update::private::ManagerInternal for AppState<S> {
    type Storage = S;

    fn get_backend_logic(&self) -> &backend::Logic<S> {
        &self.backend_logic
    }
    fn get_backend_logic_mut(&mut self) -> &mut backend::Logic<S> {
        &mut self.backend_logic
    }

    fn get_handle_managers(&self) -> &handles::ManagerCollection<S> {
        &self.handle_managers
    }
    fn get_handle_managers_mut(&mut self) -> &mut handles::ManagerCollection<S> {
        &mut self.handle_managers
    }

    fn get_history(&self) -> &ModificationHistory {
        &self.mod_history
    }
    fn get_history_mut(&mut self) -> &mut ModificationHistory {
        &mut self.mod_history
    }
}

#[derive(Debug)]

pub struct AppSession<'a, T: update::Manager> {
    op_manager: &'a mut T,
    session_history: ModificationHistory,
}

impl<'a, T: update::Manager> AppSession<'a, T> {
    pub fn new(op_manager: &'a mut T) -> Self {
        AppSession {
            op_manager,
            session_history: ModificationHistory::new(),
        }
    }

    pub fn commit(mut self) {
        self.commit_internal()
    }

    pub async fn cancel(mut self) {
        while self.can_undo() {
            let Err(e) = self.undo().await else {
                continue;
            };

            match e {
                UndoError::HistoryDepleted => panic!("can_undo call should have garanteed that history is not depleted"),
                UndoError::InternalError(int_err) => panic!(
                    "Error while cancelling session. Backend end might be in an inconsistant state.\n{}",
                    int_err
                ),
            }
        }
    }

    fn commit_internal(&mut self) {
        let aggregated_ops = self.session_history.build_aggregated_ops();
        if aggregated_ops.inner().is_empty() {
            // If no operation needs commiting, do not add an event for this session
            return;
        }
        self.op_manager.get_history_mut().apply(aggregated_ops);
        self.session_history.clear_past_history();
    }
}

impl<'a, T: update::Manager> Drop for AppSession<'a, T> {
    fn drop(&mut self) {
        self.commit_internal()
    }
}

impl<'a, T: update::Manager> ManagerInternal for AppSession<'a, T> {
    type Storage = T::Storage;

    fn get_backend_logic(&self) -> &backend::Logic<Self::Storage> {
        <T as ManagerInternal>::get_backend_logic(&self.op_manager)
    }
    fn get_backend_logic_mut(&mut self) -> &mut backend::Logic<Self::Storage> {
        self.op_manager.get_backend_logic_mut()
    }

    fn get_handle_managers(&self) -> &handles::ManagerCollection<Self::Storage> {
        self.op_manager.get_handle_managers()
    }
    fn get_handle_managers_mut(&mut self) -> &mut handles::ManagerCollection<Self::Storage> {
        self.op_manager.get_handle_managers_mut()
    }

    fn get_history(&self) -> &ModificationHistory {
        &self.session_history
    }
    fn get_history_mut(&mut self) -> &mut ModificationHistory {
        &mut self.session_history
    }
}
