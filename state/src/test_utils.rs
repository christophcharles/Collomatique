//! Test utilities
//!
//! Minimal [InMemoryData] implementations used to exercise the state machinery.
//!
//! [FakeData] is a single integer with state-dependent ops — enough for
//! history-pointer logic, rollback of aggregated operations and session
//! commit/cancel semantics. It has no invariants.
//!
//! [QuoteData] is the smallest state *with* an invariant (every quote's author
//! must exist), so it can drive the cascade ([crate::cascade]); [EvilQuoteData]
//! is a resolution-map wrapper whose deliberately misbehaving fixes exercise the
//! engine's panic and restore paths.

use std::collections::{BTreeMap, BTreeSet};

use crate::cascade::Fixable;
use crate::history::ReversibleOp;
use crate::traits::{ApplyError, InMemoryData, Operation};

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
    // FakeData has no invariants: the resolvable tier is uninhabited, so
    // `ApplyError::BrokenInvariants` is unrepresentable for it (its `Invariant`
    // is `Infallible`).
    type InvalidOp = FakeError;
    type Invariant = std::convert::Infallible;

    fn annotate(&self, op: FakeOp) -> (FakeOp, ()) {
        (op, ())
    }

    fn apply(
        &mut self,
        op: &FakeOp,
    ) -> Result<FakeOp, ApplyError<FakeError, std::convert::Infallible>> {
        match op {
            FakeOp::Set { old, new } => {
                if self.value != *old {
                    return Err(ApplyError::InvalidOp(FakeError::ValueMismatch {
                        expected: *old,
                        found: self.value,
                    }));
                }
                self.value = *new;
                Ok(FakeOp::Set {
                    old: *new,
                    new: *old,
                })
            }
            FakeOp::Fail => Err(ApplyError::InvalidOp(FakeError::ApplyFailed)),
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

/// A minimal state *with* an invariant: every quote's author must exist.
///
/// Students are a set of ids; quotes are rows attributing a saying to a
/// student. Removing a student strands their quotes — the invariant break the
/// cascade exists to resolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuoteData {
    pub students: BTreeSet<u64>,
    /// quote id -> author student id
    pub quotes: BTreeMap<u64, u64>,
}

/// Operations on [QuoteData].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuoteOp {
    /// Adds a student. No-clobber: fails if the id already exists.
    AddStudent(u64),
    /// Removes a student. Fails if the id is unknown. Strands any quote the
    /// student authored — an invariant break, not a precheck failure.
    RemoveStudent(u64),
    /// Sets (or overwrites) a quote row. The author is *not* prechecked:
    /// a dangling author is an invariant break, which is the point.
    SetQuote { quote: u64, author: u64 },
    /// Removes a quote row. Removing an absent quote is a perfect no-op
    /// (G.2 precedent), whose inverse is itself.
    RemoveQuote(u64),
}

impl Operation for QuoteOp {}

/// The unresolvable tier for [QuoteData]: bad op input.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum QuoteInvalidOp {
    #[error("unknown student {0}")]
    UnknownStudent(u64),
    #[error("student {0} already exists")]
    StudentExists(u64),
}

/// The resolvable tier for [QuoteData]: one broken invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Error)]
pub enum QuoteInvariant {
    #[error("quote {0} has a dangling author")]
    DanglingQuoteAuthor(u64),
}

impl InMemoryData for QuoteData {
    type OriginalOperation = QuoteOp;
    type AnnotatedOperation = QuoteOp;
    type NewInfo = ();
    type InvalidOp = QuoteInvalidOp;
    type Invariant = QuoteInvariant;

    // Identity annotate: the toy's ops are complete, ids are caller-chosen.
    fn annotate(&self, op: QuoteOp) -> (QuoteOp, ()) {
        (op, ())
    }

    fn apply(
        &mut self,
        op: &QuoteOp,
    ) -> Result<QuoteOp, ApplyError<QuoteInvalidOp, QuoteInvariant>> {
        // Precheck: bad op input never touches the data (the gate's tier 1).
        match op {
            QuoteOp::AddStudent(s) if self.students.contains(s) => {
                return Err(ApplyError::InvalidOp(QuoteInvalidOp::StudentExists(*s)));
            }
            QuoteOp::RemoveStudent(s) if !self.students.contains(s) => {
                return Err(ApplyError::InvalidOp(QuoteInvalidOp::UnknownStudent(*s)));
            }
            _ => {}
        }

        // Force the op onto a clone and remember the inverse (computed from the
        // pre-op state, exactly as the real gate does).
        let mut next = self.clone();
        let inverse = match op {
            QuoteOp::AddStudent(s) => {
                next.students.insert(*s);
                QuoteOp::RemoveStudent(*s)
            }
            QuoteOp::RemoveStudent(s) => {
                next.students.remove(s);
                // Valid only when no quote the student authored survives — which
                // is exactly what the cascade removes first.
                QuoteOp::AddStudent(*s)
            }
            QuoteOp::SetQuote { quote, author } => match next.quotes.insert(*quote, *author) {
                Some(old) => QuoteOp::SetQuote {
                    quote: *quote,
                    author: old,
                },
                None => QuoteOp::RemoveQuote(*quote),
            },
            QuoteOp::RemoveQuote(quote) => match next.quotes.remove(quote) {
                Some(old) => QuoteOp::SetQuote {
                    quote: *quote,
                    author: old,
                },
                None => QuoteOp::RemoveQuote(*quote),
            },
        };

        // Check the whole state (the gate's tier 2), as the real checker does:
        // every quote's author must exist.
        let broken: BTreeSet<QuoteInvariant> = next
            .quotes
            .iter()
            .filter(|(_, author)| !next.students.contains(author))
            .map(|(quote, _)| QuoteInvariant::DanglingQuoteAuthor(*quote))
            .collect();
        if !broken.is_empty() {
            // Rollback = never commit `next`; `self` is untouched.
            return Err(ApplyError::BrokenInvariants(broken));
        }

        *self = next;
        Ok(inverse)
    }
}

impl Fixable for QuoteData {
    fn fix_invariant(&self, invariant: &QuoteInvariant) -> Option<QuoteOp> {
        match invariant {
            // Presence of the removable material (design doc §5): Some only if
            // the quote row actually exists in the current state.
            QuoteInvariant::DanglingQuoteAuthor(quote) => self
                .quotes
                .contains_key(quote)
                .then(|| QuoteOp::RemoveQuote(*quote)),
        }
    }
}

/// The way an [EvilQuoteData] map misbehaves.
///
/// There is no state-*growing* mode: without a round fuse that scenario is a
/// hang, not a test — it belongs to step 6.5's `PartialOrd` in-flight check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvilMode {
    /// Always "fixes" by removing the invariant's own quote, even when it is
    /// absent — a fix that lands as a perfect no-op.
    Blind,
    /// "Fixes" a dangling quote by removing some *other* existing quote,
    /// answering `None` only once no other quote remains.
    WrongTargetElseNone,
    /// Returns an op that fails the precheck (`RemoveStudent` of an unknown id).
    InvalidFix,
    /// "Fixes" by *creating* a fresh dangling quote, then disowns the invariant
    /// that fresh quote raises.
    CreateThenDisown { fresh_quote: u64, fresh_author: u64 },
}

/// A deliberately misbehaving resolution map, to drive the engine's panic and
/// restore paths. Delegates [InMemoryData] to the inner [QuoteData] and only
/// overrides [Fixable::fix_invariant] per [EvilMode].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvilQuoteData(pub QuoteData, pub EvilMode);

impl InMemoryData for EvilQuoteData {
    type OriginalOperation = QuoteOp;
    type AnnotatedOperation = QuoteOp;
    type NewInfo = ();
    type InvalidOp = QuoteInvalidOp;
    type Invariant = QuoteInvariant;

    fn annotate(&self, op: QuoteOp) -> (QuoteOp, ()) {
        self.0.annotate(op)
    }

    fn apply(
        &mut self,
        op: &QuoteOp,
    ) -> Result<QuoteOp, ApplyError<QuoteInvalidOp, QuoteInvariant>> {
        self.0.apply(op)
    }
}

impl Fixable for EvilQuoteData {
    fn fix_invariant(&self, invariant: &QuoteInvariant) -> Option<QuoteOp> {
        let QuoteInvariant::DanglingQuoteAuthor(quote) = invariant;
        match &self.1 {
            EvilMode::Blind => Some(QuoteOp::RemoveQuote(*quote)),
            EvilMode::WrongTargetElseNone => self
                .0
                .quotes
                .keys()
                .copied()
                .find(|&q| q != *quote)
                .map(QuoteOp::RemoveQuote),
            EvilMode::InvalidFix => Some(QuoteOp::RemoveStudent(u64::MAX)),
            EvilMode::CreateThenDisown {
                fresh_quote,
                fresh_author,
            } => (quote != fresh_quote).then(|| QuoteOp::SetQuote {
                quote: *fresh_quote,
                author: *fresh_author,
            }),
        }
    }
}
