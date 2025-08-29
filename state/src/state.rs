//! State module
//!
//! Contains the definition of [AppState] and [AppSession].
//! These are the principal interface for maintaining the
//! state of a file in the application.

use super::*;
use history::ModificationHistory;

/// Representation of the state of a file for the application
///
/// This is the main structure for interfacing state.
/// Build an [AppState] with [AppState::new] starting with some
/// structure implementing [InMemoryData]. [AppState] takes
/// ownership of this data and maintains a consistent history of modifications.
#[derive(Debug, Clone)]
pub struct AppState<T: InMemoryData> {
    data: T,
    mod_history: ModificationHistory<T::AnnotatedOperation>,
}

impl<T: InMemoryData> AppState<T> {
    /// Builds a new [AppState] from an [InMemoryData] structure
    ///
    /// The new [AppState] takes ownership of the structure.
    pub fn new(data: T) -> Self {
        AppState {
            data,
            mod_history: ModificationHistory::new(),
        }
    }

    /// Builds a new [AppState] from an [InMemoryData] structure
    ///
    /// This is similar to [AppState::new] except that the modification
    /// history is build using [ModificationHistory::with_max_history_size]
    /// rather than [ModificationHistory::new].
    /// As a consequence, you can control the maximum length of the history.
    pub fn with_max_history_size(data: T, max_history_size: Option<usize>) -> Self {
        AppState {
            data,
            mod_history: ModificationHistory::with_max_history_size(max_history_size),
        }
    }

    /// Returns the maximum history size
    pub fn get_max_history_size(&self) -> Option<usize> {
        self.mod_history.get_max_history_size()
    }

    /// Sets the maximum history size
    ///
    /// See [ModificationHistory::set_max_history_size].
    pub fn set_max_history_size(&mut self, max_history_size: Option<usize>) {
        self.mod_history.set_max_history_size(max_history_size);
    }
}

impl<T: InMemoryData> traits::private::ManagerInternal for AppState<T> {
    type Data = T;

    fn get_in_memory_data(&self) -> &Self::Data {
        &self.data
    }

    fn get_in_memory_data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
    }

    fn get_modification_history(&self) -> &ModificationHistory<T::AnnotatedOperation> {
        &self.mod_history
    }
    fn get_modification_history_mut(&mut self) -> &mut ModificationHistory<T::AnnotatedOperation> {
        &mut self.mod_history
    }
}

/// A modification session
///
/// Sometimes it is necesseray to create sessions in an application.
/// This is when we quit the flow of normal file editing. Instead,
/// we set a blank history and start working on the document.
///
/// At the end of the session, the history is either dismissed and the previous state
/// is restored, or the history is commited to the main history (as an atomic operation).
///
/// This is useful for instance to run scripts on the file. Scripts can do
/// many thigns and finally fail. This way, we marked the start of the script.
/// If it fails, we simply dismiss the modifications.
/// If it succeeds, we can commit it into history as a single "script" operation.
///
/// [AppSession] implements [Drop]. By default, a session is cancelled on dropping
/// if neither [AppSession::commit] nor [AppSession::cancel] is called.
#[derive(Debug)]

pub struct AppSession<'a, T: traits::Manager> {
    op_manager: &'a mut T,
    session_history: ModificationHistory<
        <<T as traits::private::ManagerInternal>::Data as InMemoryData>::AnnotatedOperation,
    >,
}

impl<'a, T: traits::Manager> AppSession<'a, T> {
    /// Builds a new [AppSession]
    ///
    /// An [AppSession] is created from a mutable reference to
    /// an already existing [traits::Manager]. Typically, this
    /// will be an [AppState]. But technically, it is possible to
    /// nest sessions.
    pub fn new(op_manager: &'a mut T) -> Self {
        AppSession {
            op_manager,
            // Modification history must be potentially infinite to
            // allow the restauration of the initial state of the session
            session_history: ModificationHistory::new(),
        }
    }

    /// Commit the session
    pub fn commit(mut self) {
        self.commit_internal()
    }

    /// Cancel the session (implicit if dropped)
    pub fn cancel(self) {
        drop(self)
    }

    /// Used internally
    ///
    /// Actually commit the history
    fn commit_internal(&mut self) {
        let aggregated_op = self.session_history.build_aggregated_op();
        if aggregated_op.inner().is_empty() {
            // If no operation needs commiting, do not add an event for this session
            return;
        }
        // We only update the history: the state is already up to date
        self.op_manager
            .get_modification_history_mut()
            .store(aggregated_op);

        // We NEED to clear the history, otherwise it would be cancelled on drop
        self.session_history.clear_past_history();
    }
}

impl<'a, T: traits::Manager> Drop for AppSession<'a, T> {
    fn drop(&mut self) {
        while <Self as traits::Manager>::can_undo(self) {
            <Self as traits::Manager>::undo(self).expect("History not depleted");
        }
    }
}

impl<'a, T: traits::Manager> traits::private::ManagerInternal for AppSession<'a, T> {
    type Data = T::Data;

    fn get_in_memory_data(&self) -> &Self::Data {
        self.op_manager.get_in_memory_data()
    }
    fn get_in_memory_data_mut(&mut self) -> &mut Self::Data {
        self.op_manager.get_in_memory_data_mut()
    }

    fn get_modification_history(
        &self,
    ) -> &ModificationHistory<<T::Data as InMemoryData>::AnnotatedOperation> {
        &self.session_history
    }
    fn get_modification_history_mut(
        &mut self,
    ) -> &mut ModificationHistory<<T::Data as InMemoryData>::AnnotatedOperation> {
        &mut self.session_history
    }
}
