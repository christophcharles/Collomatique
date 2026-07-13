//! Test utilities
//!
//! A minimal [InMemoryData] implementation over a single integer,
//! with operations whose success depends on the current state.
//! This is enough to exercise history pointer logic, rollback of
//! aggregated operations and session commit/cancel semantics.

use crate::history::ReversibleOp;
use crate::traits::{InMemoryData, Operation};

use thiserror::Error;

/// Fake data: a single integer value
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FakeData {
    pub value: i64,
}

impl FakeData {
    pub fn new(value: i64) -> Self {
        FakeData { value }
    }
}

/// Fake operations on [FakeData]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FakeOp {
    /// Sets the value to `new`, but only succeeds if the current value is `old`
    ///
    /// The state-dependent failure makes it possible to build aggregated
    /// operations that fail midway.
    Set { old: i64, new: i64 },
    /// Always fails on apply
    Fail,
}

impl Operation for FakeOp {}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum FakeError {
    #[error("expected value {expected}, found {found}")]
    ValueMismatch { expected: i64, found: i64 },
    #[error("apply failed")]
    ApplyFailed,
}

impl InMemoryData for FakeData {
    type OriginalOperation = FakeOp;
    type AnnotatedOperation = FakeOp;
    type NewInfo = ();
    type Error = FakeError;

    fn annotate(&self, op: FakeOp) -> (FakeOp, ()) {
        (op, ())
    }

    fn apply(&mut self, op: &FakeOp) -> Result<FakeOp, FakeError> {
        match op {
            FakeOp::Set { old, new } => {
                if self.value != *old {
                    return Err(FakeError::ValueMismatch {
                        expected: *old,
                        found: self.value,
                    });
                }
                self.value = *new;
                Ok(FakeOp::Set {
                    old: *new,
                    new: *old,
                })
            }
            FakeOp::Fail => Err(FakeError::ApplyFailed),
        }
    }
}

/// Builds the [ReversibleOp] corresponding to `Set { old, new }`
pub fn rev_set(old: i64, new: i64) -> ReversibleOp<FakeOp> {
    ReversibleOp {
        forward: FakeOp::Set { old, new },
        backward: FakeOp::Set { old: new, new: old },
    }
}
