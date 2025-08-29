//! History module
//!
//! This module defines the various types
//! and functions to maintain a modification history.
//!
//! There are several parts to this:
//! - first, though modification operations are defined in [crate::InMemoryData::OriginalOperation],
//!   they are not *complete* which means they are not *reproducible*.
//!   For instance, when adding a new student the id is not set.
//!   So doing it once, then canceling it, then redoing it again would
//!   lead to a different id being issued.
//!
//!   It is a problem for instance in the following sequence: adding a
//!   student, then modifying the student, then cancelling both, then redoing both.
//!   In that sequence, the student modification is associated to the
//!   first id that was issued.
//!
//!   They are several ways out of this. The way that was chosen here
//!   is to actually issue an id at the moment of the operation creation
//!   and complete the operation into an [crate::InMemoryData::AnnotatedOperation] that contains
//!   all the information about the operation.
//!
//!   So when we first create the student, before doing any modification,
//!   we issue a new id, lets call it `student_id`, and create an annotated
//!   operation that says that we want to create a student with the specific
//!   `student_id`. Then, this operation, because it is a *complete* description
//!   of the result of the operation, can be done and redone and will lead to
//!   the same result every time.
//!
//!   This, of course, has a drawback: it leads to a new failure mode. It is
//!   possible to try to add a student with an existing id.
//!
//! - second, now that we have [crate::InMemoryData::AnnotatedOperation] that gives a *complete* description
//!   of the result of an operation, we need to make it *reversible*.
//!
//!   A [ReversibleOp] is an operation that contains the action that must be done
//!   to do the operation as well as to undo the operation.
//!
//!   This however is complicated because it depends on the state of the data when
//!   the action is applied.
//!
//!   For instance, a 'remove student operation' though its effects are clear do not
//!   contain the necessary information to reverse it. This information depends on the
//!   actual student description at the moment of removal.
//!   A [ReversibleOp] therefore depends on a particular [crate::InMemoryData] at a certain point in
//!   time.
//!
//!   The type is defined here but it is actually build with [crate::InMemoryData::build_rev_with_current_state]. When an
//!   operation is applied to [crate::InMemoryData], the state of [crate::InMemoryData] *at that moment* can be read
//!   and the corresponding reverse operation can be built. So before applying the operation
//!   we will call [crate::InMemoryData::build_rev_with_current_state] and build the corresponding
//!   [ReversibleOp] that can be store in the modification history.
//!
//! - third, this module defines [ModificationHistory] which actually contains and
//!   stores the modification history. Apart from the last point that will discuss
//!   below, it just maintains a list of reversible operations as well as a pointer to
//!   the last one applied.
//!
//!   It is able to handle capped history size as well as potentially infinite (well
//!   it is still memory limited...) history size.
//!
//! - fourth, we define [AggregatedOp]. This type is useful to have
//!   *larger* atomatic operations. Let's say for instance that we want to remove
//!   *all* students. No atomatic operation on the [crate::InMemoryData] is defined for this.
//!   But, we can actually aggregate the operations together and make it a unique
//!   operation in history.
//!
//!   This is useful for two scenarios in Collomatique. First, some GUI interactions
//!   naturally lead to aggregated operations. For instance, if we want to remove
//!   a student, it must not be referenced anywhere in other constraints. For instance
//!   it should unlisted for the various subjects.
//!
//!   It is useful to have a button in a GUI to remove this student as well as removing
//!   all the references to them in the other sections. But when we cancel this operation
//!   we want to restore everything at once. So, this leads to an [AggregatedOp] that
//!   will remove the student and all their references in one go, but also restore all
//!   of it in one go when cancelled.
//!
//!   The second scenario is linked. We want to be able to execute *scripts*. A simple
//!   example of this is when we import data from a csv. Whatever the way we choose
//!   to do the importation (actual rust function or Python script or something else)
//!   we want the whole importation to be a single operation in history.
//!
//!   Then, cancelling the importation will reverse all the data imported. But redoing
//!   the operation will reimport everything.
//!
//!   Therefore, [ModificationHistory] does not technically contain a list of [ReversibleOp]
//!   but rather a list of [AggregatedOp].
//!

use std::collections::VecDeque;

use crate::Operation;

/// Reversible operation
///
/// This type contains the description of an operation
/// as well as the reverse operation.
///
/// Be careful, this description is necesserally linked
/// to a state of [crate::InMemoryData] at a precise moment at which
/// the operation was applied.
///
/// See [super::history] for a full discussion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReversibleOp<T: Operation> {
    /// Forward operation
    pub(crate) forward: T,
    /// Backward (or reversed) operation
    pub(crate) backward: T,
}

impl<T: Operation> ReversibleOp<T> {
    /// Returns the reversed operation
    ///
    /// Because the operation is *reversible*
    /// it can actually be reversed.
    ///
    /// This returns a clone of the reversed operation.
    pub fn rev(&self) -> Self {
        self.clone().into_rev()
    }

    /// Returns the reversed operation
    ///
    /// Because the operation is *reversible*
    /// it can actually be reversed.
    ///
    /// This returns the reversed operation
    /// and consumes the original [ReversibleOp].
    pub fn into_rev(self) -> Self {
        ReversibleOp {
            forward: self.backward,
            backward: self.forward,
        }
    }

    /// Returns the primitive forward operation
    ///
    /// This returns a reference to the original op.
    pub fn inner(&self) -> &T {
        &self.forward
    }

    /// Returns the primitive forward operation
    ///
    /// This consumes the [ReversibleOp].
    pub fn into_inner(self) -> T {
        self.forward
    }
}

/// Aggregated operations type
///
/// This type groups together several [ReversibleOp]
/// into one single aggregated operation that takes
/// only one slot in the modification history.
///
/// Because it contains [ReversibleOp], an [AggregatedOp]
/// similarly is linked to a specific state of the [crate::InMemoryData].
///
/// See [super::history] for the full discussion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregatedOp<T: Operation>(Vec<ReversibleOp<T>>);

impl<T: Operation> AggregatedOp<T> {
    /// Builds an aggregated operation from a list of reversible ops
    ///
    /// Normally, you should never have to build an aggregated op manually.
    pub(crate) fn new(ops: Vec<ReversibleOp<T>>) -> Self {
        AggregatedOp(ops)
    }

    /// Returns the reversed operation
    ///
    /// Because the operation is *reversible*
    /// it can actually be reversed.
    ///
    /// This returns a new [AggregatedOp] that describes all
    /// the operations to do to reverse the complete [AggregatedOp].
    pub fn rev(&self) -> Self {
        AggregatedOp(self.0.iter().rev().map(|x| x.rev()).collect())
    }

    /// Returns the list of [ReversibleOp] in the [AggregatedOp].
    pub fn inner(&self) -> &Vec<ReversibleOp<T>> {
        &self.0
    }
}

/// Modification history
///
/// This type maintains a modification history.
/// It contains a list of [AggregatedOp] that describes what
/// modifications were done and how to reverse them.
///
/// It can optionnaly manage a finite size history and forget
/// operations that are deemed too old.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ModificationHistory<T: Operation> {
    /// Actual history of operations
    history: VecDeque<(AggregatedOp<T>, String)>,

    /// Points to the place of the next operation to store in history
    history_pointer: usize,

    /// Maximum size for the history
    ///
    /// Having arbitrary long history can be a problem. So we
    /// have the option to limit it. If the history becomes larger
    /// old operations will be forgotten and won't be able to be
    /// reversed anymore.
    ///
    /// It is still possible to set this to `None` and keep the
    /// history indefinitely. This is useful in particular when
    /// we want to execute a script.
    max_history_size: Option<usize>,
}

impl<T: Operation> ModificationHistory<T> {
    /// Used internally
    ///
    /// Truncate the history if it has become too big.
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

impl<T: Operation> ModificationHistory<T> {
    /// Creates a new modification history with default parameters
    ///
    /// By default, [ModificationHistory] maintains a potentially
    /// infinite history.
    pub fn new() -> Self {
        ModificationHistory {
            history: std::collections::VecDeque::new(),
            history_pointer: 0,
            max_history_size: None,
        }
    }

    /// Creates a new modification history with specific maximum history size
    ///
    /// Calling with `None` is equivalent to calling [ModificationHistory::new].
    pub fn with_max_history_size(max_history_size: Option<usize>) -> Self {
        ModificationHistory {
            history: std::collections::VecDeque::new(),
            history_pointer: 0,
            max_history_size,
        }
    }

    /// Get the maximum length of the history
    pub fn get_max_history_size(&self) -> Option<usize> {
        self.max_history_size
    }

    /// Change the maximum length of the history
    ///
    /// BEWARE: this will destroy older operations if the history is too long
    /// and it won't be possible to recover them.
    pub fn set_max_history_size(&mut self, max_history_size: Option<usize>) {
        self.max_history_size = max_history_size;

        self.truncate_history_as_needed();
    }

    /// Store an operation in history
    ///
    /// This will erase future operations. This means that
    /// if some operation was cancelled and remained in history
    /// to be able to apply them, they will be discarded and this branch
    /// of history is lost.
    pub fn store(&mut self, aggregated_op: AggregatedOp<T>, name: String) {
        self.history.truncate(self.history_pointer);

        self.history_pointer += 1;
        self.history.push_back((aggregated_op, name));

        self.truncate_history_as_needed();
    }

    /// Returns the name of the last operation if it exists
    pub fn get_undo_name(&self) -> Option<&str> {
        if self.history_pointer == 0 {
            return None;
        }

        Some(&self.history[self.history_pointer - 1].1)
    }

    /// Returns the name of the next operation if it exists
    pub fn get_redo_name(&self) -> Option<&str> {
        if self.history_pointer == self.history.len() {
            return None;
        }

        Some(&self.history[self.history_pointer].1)
    }

    /// Returns `true` if there is at least one operation to undo in history
    pub fn can_undo(&self) -> bool {
        self.history_pointer > 0
    }

    /// Returns `true` if there is at least one operation to redo in history
    pub fn can_redo(&self) -> bool {
        self.history_pointer < self.history.len()
    }

    /// Cancels the last operation in history
    ///
    /// The history is actually preserved but this changes
    /// the position of the pointer to the current operation.
    ///
    /// It also returns the (aggregated) reverse operation that should be done
    /// to cancel the last op.
    ///
    /// It can fail and returns `None` if no operation to be cancelled
    /// is found in history.
    pub fn undo(&mut self) -> Option<AggregatedOp<T>> {
        if !self.can_undo() {
            return None;
        }

        self.history_pointer -= 1;

        assert!(self.history_pointer < self.history.len());

        let last_ops = self.history[self.history_pointer].clone();

        Some(last_ops.0.rev())
    }

    /// Redo the last cancelled operation
    ///
    /// The history is actually preserved but this changes
    /// the position of the pointer to the current operation.
    ///
    /// It also returns the (aggregated) operation that should be done
    /// to restore the state.
    ///
    /// It can fail and returns `None` if no operation to be redone
    /// is found in history.
    pub fn redo(&mut self) -> Option<AggregatedOp<T>> {
        if !self.can_redo() {
            return None;
        }

        let new_ops = self.history[self.history_pointer].clone();
        self.history_pointer += 1;

        Some(new_ops.0)
    }

    /// Builds an aggregated operation corresponding to the full history
    ///
    /// This function creates a new aggregated operation that corresponds to
    /// the full history from the oldest operation stored to the current pointer
    /// in history (cancelled operations that remain to be redone are not aggregated).
    ///
    /// This is particularly useful when using a history without maximum size as this returns
    /// the single aggregated operation that transforms the initial state into the current state.
    ///
    /// It can be used for script execution: build a temporary history and use it rather than the
    /// main one. If the script fails, this allows cancellation of its operations.
    /// If the script succeeds, we can aggregate its operation into a single one and move it to
    /// the main history.
    pub fn build_aggregated_op(&self) -> AggregatedOp<T> {
        AggregatedOp::new(
            self.history
                .iter()
                .take(self.history_pointer)
                .flat_map(|aggregated_ops| aggregated_ops.0.inner().iter())
                .cloned()
                .collect(),
        )
    }

    /// Clear the past history
    ///
    /// This is irreversible: all past operations will be forgotten and lost.
    /// It is still possible to use `redo` for future operations after calling
    /// this function though.
    pub fn clear_past_history(&mut self) {
        let new_history = self.history.split_off(self.history_pointer);
        self.history = new_history;
        self.history_pointer = 0;
    }
}
