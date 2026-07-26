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
pub struct AppState<T: InMemoryData, D: Description> {
    data: T,
    mod_history: ModificationHistory<T::AnnotatedOperation, D>,
}

impl<T: InMemoryData, D: Description> AppState<T, D> {
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

impl<T: InMemoryData, D: Description> traits::private::ManagerInternal for AppState<T, D> {
    type Data = T;
    type Desc = D;

    fn get_in_memory_data(&self) -> &Self::Data {
        &self.data
    }

    fn get_in_memory_data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
    }

    fn get_modification_history(&self) -> &ModificationHistory<T::AnnotatedOperation, D> {
        &self.mod_history
    }
    fn get_modification_history_mut(
        &mut self,
    ) -> &mut ModificationHistory<T::AnnotatedOperation, D> {
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
/// You should always finish by calling [AppSession::commit] or [AppSession::cancel]
/// as this will return the ownership of the [traits::Manager].
/// Simply droping [AppSession] means also loosing the corresponding [traits::Manager].
#[derive(Debug, Clone)]

pub struct AppSession<T: traits::Manager, D: Description> {
    op_manager: T,
    session_history: ModificationHistory<
        <<T as traits::private::ManagerInternal>::Data as InMemoryData>::AnnotatedOperation,
        D,
    >,
}

impl<T: traits::Manager, D: Description> AppSession<T, D> {
    /// Builds a new [AppSession]
    ///
    /// An [AppSession] is created from a mutable reference to
    /// an already existing [traits::Manager]. Typically, this
    /// will be an [AppState]. But technically, it is possible to
    /// nest sessions.
    pub fn new(op_manager: T) -> Self {
        AppSession {
            op_manager,
            // Modification history must be potentially infinite to
            // allow the restauration of the initial state of the session
            session_history: ModificationHistory::new(),
        }
    }

    /// Commits the session and returns the Manager with one aggregated op in history
    pub fn commit(mut self, desc: T::Desc) -> T {
        let aggregated_op = self.session_history.build_aggregated_op();
        // We only update the history: the state is already up to date
        self.op_manager
            .get_modification_history_mut()
            .store(aggregated_op, desc);
        self.op_manager
    }

    /// Cancels the whole session and returns the Manager in its initial state
    pub fn cancel(mut self) -> T {
        // Cancel all modifications to the initial state
        while <Self as traits::Manager>::can_undo(&self) {
            <Self as traits::Manager>::undo(&mut self).expect("History not depleted");
        }
        // Return the manager
        self.op_manager
    }
}

impl<T: traits::Manager, D: Description> traits::private::ManagerInternal for AppSession<T, D> {
    type Data = T::Data;
    type Desc = D;

    fn get_in_memory_data(&self) -> &Self::Data {
        self.op_manager.get_in_memory_data()
    }
    fn get_in_memory_data_mut(&mut self) -> &mut Self::Data {
        self.op_manager.get_in_memory_data_mut()
    }

    fn get_modification_history(
        &self,
    ) -> &ModificationHistory<<T::Data as InMemoryData>::AnnotatedOperation, D> {
        &self.session_history
    }
    fn get_modification_history_mut(
        &mut self,
    ) -> &mut ModificationHistory<<T::Data as InMemoryData>::AnnotatedOperation, D> {
        &mut self.session_history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{FakeData, FakeOp};
    use crate::traits::{HistoryError, Manager};

    fn set(old: i64, new: i64) -> FakeOp {
        FakeOp::Set { old, new }
    }

    fn new_state(value: i64) -> AppState<FakeData, &'static str> {
        AppState::new(FakeData::new(value))
    }

    #[test]
    fn max_history_size_is_wired_to_history() {
        let mut state =
            AppState::<_, &'static str>::with_max_history_size(FakeData::new(0), Some(2));
        assert_eq!(state.get_max_history_size(), Some(2));

        state.apply(set(0, 1), "set to 1").expect("valid op");
        state.apply(set(1, 2), "set to 2").expect("valid op");
        state.apply(set(2, 3), "set to 3").expect("valid op");

        // Oldest op was forgotten: only two undos are possible
        state.undo().expect("one op to undo");
        state.undo().expect("one op to undo");
        assert_eq!(state.undo(), Err(HistoryError::HistoryDepleted));
        assert_eq!(state.get_data().value, 1);
    }

    #[test]
    fn set_max_history_size_shrinks_existing_history() {
        let mut state = new_state(0);
        state.apply(set(0, 1), "set to 1").expect("valid op");
        state.apply(set(1, 2), "set to 2").expect("valid op");
        state.apply(set(2, 3), "set to 3").expect("valid op");

        state.set_max_history_size(Some(1));

        state.undo().expect("one op to undo");
        assert_eq!(state.undo(), Err(HistoryError::HistoryDepleted));
        assert_eq!(state.get_data().value, 2);
    }

    #[test]
    fn session_cancel_restores_state_and_leaves_parent_history_untouched() {
        let mut state = new_state(0);
        state.apply(set(0, 1), "set to 1").expect("valid op");

        let mut session = AppSession::<_, &'static str>::new(state);
        session.apply(set(1, 2), "set to 2").expect("valid op");
        session.apply(set(2, 3), "set to 3").expect("valid op");

        let state = session.cancel();

        assert_eq!(state.get_data().value, 1);
        assert_eq!(state.get_undo_name(), Some(&"set to 1"));
        assert!(!state.can_redo());
    }

    #[test]
    fn session_commit_is_a_single_atomic_parent_slot() {
        let mut state = new_state(0);
        state.apply(set(0, 1), "set to 1").expect("valid op");

        let mut session = AppSession::<_, &'static str>::new(state);
        session.apply(set(1, 2), "set to 2").expect("valid op");
        session.apply(set(2, 3), "set to 3").expect("valid op");
        session.apply(set(3, 4), "set to 4").expect("valid op");

        let mut state = session.commit("batch");

        assert_eq!(state.get_data().value, 4);
        assert_eq!(state.get_undo_name(), Some(&"batch"));

        // One undo cancels the whole session at once
        state.undo().expect("one op to undo");
        assert_eq!(state.get_data().value, 1);
        assert_eq!(state.get_undo_name(), Some(&"set to 1"));

        state.redo().expect("one op to redo");
        assert_eq!(state.get_data().value, 4);
    }

    #[test]
    fn session_commit_excludes_ops_undone_within_the_session() {
        let state = new_state(0);

        let mut session = AppSession::<_, &'static str>::new(state);
        session.apply(set(0, 1), "set to 1").expect("valid op");
        session.apply(set(1, 2), "set to 2").expect("valid op");
        session.undo().expect("one op to undo");

        let mut state = session.commit("batch");

        assert_eq!(state.get_data().value, 1);
        state.undo().expect("one op to undo");
        assert_eq!(state.get_data().value, 0);
    }

    #[test]
    fn zero_op_session_commit_stores_an_empty_slot() {
        // Pins current behavior: committing an empty session still
        // takes one slot in the parent history (undoing it is a no-op)
        let state = new_state(0);

        let session = AppSession::<_, &'static str>::new(state);
        let mut state = session.commit("empty batch");

        assert!(state.can_undo());
        assert_eq!(state.get_undo_name(), Some(&"empty batch"));
        let undone = state.undo().expect("one (empty) op to undo");
        assert!(undone.inner().is_empty());
        assert_eq!(state.get_data().value, 0);
    }

    #[test]
    fn nested_session_commit_then_outer_cancel_restores_initial_state() {
        let state = new_state(0);

        let mut outer = AppSession::<_, &'static str>::new(state);
        outer.apply(set(0, 1), "set to 1").expect("valid op");

        let mut inner = AppSession::<_, &'static str>::new(outer);
        inner.apply(set(1, 2), "set to 2").expect("valid op");

        let outer = inner.commit("inner batch");
        assert_eq!(outer.get_data().value, 2);

        let state = outer.cancel();
        assert_eq!(state.get_data().value, 0);
        assert!(!state.can_undo());
    }

    #[test]
    fn nested_session_commit_both_makes_one_parent_slot() {
        let state = new_state(0);

        let mut outer = AppSession::<_, &'static str>::new(state);
        outer.apply(set(0, 1), "set to 1").expect("valid op");

        let mut inner = AppSession::<_, &'static str>::new(outer);
        inner.apply(set(1, 2), "set to 2").expect("valid op");

        let outer = inner.commit("inner batch");
        let mut state = outer.commit("outer batch");

        assert_eq!(state.get_data().value, 2);
        assert_eq!(state.get_undo_name(), Some(&"outer batch"));

        state.undo().expect("one op to undo");
        assert_eq!(state.get_data().value, 0);
        assert!(!state.can_undo());
    }
}
