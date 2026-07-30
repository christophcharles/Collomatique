//! Test utilities
//!
//! Minimal [InMemoryData] implementations used to exercise the state machinery.
//!
//! [FakeData] is a single integer with state-dependent ops — enough for
//! history-pointer logic, rollback of aggregated operations and session
//! commit/cancel semantics. It has no invariants.
//!
//! [QuoteData] is the smallest state *with* invariants (every quote's author
//! must exist, and every note's quote must exist — three levels, so a fix can
//! itself need a fix), so it can drive the cascade ([crate::cascade]); [EvilQuoteData]
//! is a resolution-map wrapper whose deliberately misbehaving fixes exercise the
//! engine's panic and restore paths.

use std::collections::{BTreeMap, BTreeSet};

use crate::cascade::Fixable;
use crate::history::ReversibleOp;
use crate::partial_order::ContentOrd;
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

/// A minimal state *with* invariants: every quote's author must exist, and
/// every note's quote must exist.
///
/// Students are a set of ids; quotes are rows attributing a saying to a
/// student; notes are rows commenting on a quote. Removing a student strands
/// their quotes — the invariant break the cascade exists to resolve. The
/// third level exists so a *fix* can itself break an invariant: removing a
/// stranded quote strands its notes, which makes the depth-first unwinding
/// (deepest fix first, target last) reachable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuoteData {
    pub students: BTreeSet<u64>,
    /// quote id -> author student id
    pub quotes: BTreeMap<u64, u64>,
    /// note id -> commented quote id
    pub notes: BTreeMap<u64, u64>,
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
    /// Rewrites an existing quote's author. Unlike [QuoteOp::SetQuote] the
    /// quote must exist (precheck) — which is what lets a cascade fix consume
    /// the target's own target and drive the retried target into `InvalidOp`
    /// with a remembered break (the D4 conviction route).
    UpdateQuote { quote: u64, author: u64 },
    /// Sets (or overwrites) a note row. The commented quote is *not*
    /// prechecked — the [QuoteOp::SetQuote] mirror one level down.
    SetNote { note: u64, quote: u64 },
    /// Removes a note row. Removing an absent note is a perfect no-op,
    /// whose inverse is itself.
    RemoveNote(u64),
}

impl Operation for QuoteOp {}

/// The unresolvable tier for [QuoteData]: bad op input.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum QuoteInvalidOp {
    #[error("unknown student {0}")]
    UnknownStudent(u64),
    #[error("student {0} already exists")]
    StudentExists(u64),
    #[error("unknown quote {0}")]
    UnknownQuote(u64),
}

/// The resolvable tier for [QuoteData]: one broken invariant.
///
/// Declaration order is load-bearing: the derived `Ord` is what the engine's
/// canonical pick (`set.first()`) follows, so quote invariants sort before
/// note invariants. Keeping the levels in that order means a cascade repairs
/// the quote level first, and the note breakage a quote removal creates is
/// discovered as a *fresh* failure of that fix — which is exactly the
/// depth-3 shape the toy exists to exercise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Error)]
pub enum QuoteInvariant {
    #[error("quote {0} has a dangling author")]
    DanglingQuoteAuthor(u64),
    #[error("note {0} comments a quote that does not exist")]
    DanglingNoteQuote(u64),
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
            QuoteOp::UpdateQuote { quote, .. } if !self.quotes.contains_key(quote) => {
                return Err(ApplyError::InvalidOp(QuoteInvalidOp::UnknownQuote(*quote)));
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
            QuoteOp::UpdateQuote { quote, author } => {
                let old = next
                    .quotes
                    .insert(*quote, *author)
                    .expect("prechecked: the quote exists");
                QuoteOp::UpdateQuote {
                    quote: *quote,
                    author: old,
                }
            }
            QuoteOp::SetNote { note, quote } => match next.notes.insert(*note, *quote) {
                Some(old) => QuoteOp::SetNote {
                    note: *note,
                    quote: old,
                },
                None => QuoteOp::RemoveNote(*note),
            },
            QuoteOp::RemoveNote(note) => match next.notes.remove(note) {
                Some(old) => QuoteOp::SetNote {
                    note: *note,
                    quote: old,
                },
                None => QuoteOp::RemoveNote(*note),
            },
        };

        // Check the whole state (the gate's tier 2), as the real checker does:
        // every quote's author must exist, and every note's quote must exist.
        let broken: BTreeSet<QuoteInvariant> = next
            .quotes
            .iter()
            .filter(|(_, author)| !next.students.contains(author))
            .map(|(quote, _)| QuoteInvariant::DanglingQuoteAuthor(*quote))
            .chain(
                next.notes
                    .iter()
                    .filter(|(_, quote)| !next.quotes.contains_key(quote))
                    .map(|(note, _)| QuoteInvariant::DanglingNoteQuote(*note)),
            )
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
            QuoteInvariant::DanglingNoteQuote(note) => self
                .notes
                .contains_key(note)
                .then(|| QuoteOp::RemoveNote(*note)),
        }
    }
}

/// The document order on the toy: students by set inclusion, quotes and notes
/// by map inclusion with atomic values (re-authoring a quote, or re-pointing
/// a note, is incomparable; removing one is strictly below).
///
/// Hand-written on purpose: it exercises the combinators and the engine with
/// zero dependence on `#[derive(ContentOrd)]`, and doubles as documentation
/// of what that derive expands to.
impl ContentOrd for QuoteData {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let QuoteData {
            students,
            quotes,
            notes,
        } = self;
        crate::partial_order::combine([
            crate::partial_order::set_inclusion(students, &other.students),
            crate::partial_order::map_inclusion(quotes, &other.quotes, |a, b| {
                crate::partial_order::discrete(a, b)
            }),
            crate::partial_order::map_inclusion(notes, &other.notes, |a, b| {
                crate::partial_order::discrete(a, b)
            }),
        ])
    }
}

/// The way an [EvilQuoteData] map misbehaves.
///
/// The two step-6.5 modes ([EvilMode::CreateAuthor] and
/// [EvilMode::ReauthorExisting]) violate the contract while *resolving* the
/// invariant: before the in-flight document-order check they would have led
/// the cascade to a quiet, creative `Ok`. Now they panic.
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
    /// "Fixes" a dangling author by *creating* the missing student — the
    /// state grows, which is exactly what a strictly-decreasing map may
    /// never do, however helpful it looks.
    ///
    /// The author id is carried by the mode because the map cannot read it
    /// from the state: at fix time the dangling quote of the invariant was
    /// rolled back and does not exist in `self`.
    CreateAuthor { author: u64 },
    /// "Fixes" a dangling author by re-authoring some existing quote to an
    /// existing student — the state moves *sideways* (incomparable),
    /// neither shrinking nor growing.
    ReauthorExisting { quote: u64, author: u64 },
    /// Answers the dangling invariant of quote `a` by "fixing" quote `b` and
    /// vice versa, each fix creating the other dangling quote — a two-cycle
    /// of never-landing fixes. Without the no-progress ledger this hangs the
    /// cascade forever.
    PingPong { a: u64, b: u64, author: u64 },
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
        // Every mode misbehaves on the quote level only; the note level is
        // answered honestly, so the existing evil tests read unchanged.
        let quote = match invariant {
            QuoteInvariant::DanglingQuoteAuthor(quote) => quote,
            QuoteInvariant::DanglingNoteQuote(_) => return self.0.fix_invariant(invariant),
        };
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
            EvilMode::CreateAuthor { author } => Some(QuoteOp::AddStudent(*author)),
            EvilMode::ReauthorExisting { quote, author } => Some(QuoteOp::SetQuote {
                quote: *quote,
                author: *author,
            }),
            EvilMode::PingPong { a, b, author } => {
                let other = if quote == a { *b } else { *a };
                Some(QuoteOp::SetQuote {
                    quote: other,
                    author: *author,
                })
            }
        }
    }
}

/// The document order sees only the inner data: the mode is test-harness
/// configuration, not content. Two [EvilQuoteData] with equal data but
/// different modes are content-equivalent while `!=` — the toy mirror of the
/// colloscope `Data`'s ignored id issuer, and legal because no law relates
/// [ContentOrd] to [PartialEq].
impl ContentOrd for EvilQuoteData {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.content_cmp(&other.0)
    }
}

#[cfg(test)]
mod partial_order_tests {
    use super::*;
    use std::cmp::Ordering;

    fn quote_data(students: &[u64], quotes: &[(u64, u64)]) -> QuoteData {
        QuoteData {
            students: students.iter().copied().collect(),
            quotes: quotes.iter().copied().collect(),
            notes: BTreeMap::new(),
        }
    }

    #[test]
    fn equal_documents_are_equivalent() {
        let a = quote_data(&[1, 2], &[(10, 1), (11, 2)]);
        let b = quote_data(&[1, 2], &[(10, 1), (11, 2)]);
        assert_eq!(a.content_cmp(&b), Some(Ordering::Equal));
    }

    #[test]
    fn removing_a_quote_is_strictly_below() {
        let full = quote_data(&[1, 2], &[(10, 1), (11, 2)]);
        let trimmed = quote_data(&[1, 2], &[(10, 1)]);
        assert_eq!(trimmed.content_cmp(&full), Some(Ordering::Less));
        assert_eq!(full.content_cmp(&trimmed), Some(Ordering::Greater));
    }

    #[test]
    fn adding_a_student_is_strictly_above() {
        let small = quote_data(&[1], &[(10, 1)]);
        let grown = quote_data(&[1, 2], &[(10, 1)]);
        assert_eq!(grown.content_cmp(&small), Some(Ordering::Greater));
    }

    #[test]
    fn re_authoring_a_quote_is_incomparable() {
        // Same keys on both sides, so the key sets say `Equal`; the shared
        // row's author is an atom, so it says `None` — and `None` wins.
        let by_one = quote_data(&[1, 2], &[(10, 1)]);
        let by_two = quote_data(&[1, 2], &[(10, 2)]);
        assert_eq!(by_one.content_cmp(&by_two), None);
        assert_eq!(by_two.content_cmp(&by_one), None);
    }

    #[test]
    fn mixed_directions_are_incomparable() {
        // One field goes down while the other goes up.
        let fewer_students = quote_data(&[1], &[(10, 1), (11, 1)]);
        let more_quotes = quote_data(&[1, 2], &[(10, 1)]);
        assert_eq!(fewer_students.content_cmp(&more_quotes), None);

        // Not vacuously `None`: each half really is comparable on its own.
        let baseline = quote_data(&[1], &[(10, 1)]);
        assert_eq!(baseline.content_cmp(&fewer_students), Some(Ordering::Less));
        assert_eq!(baseline.content_cmp(&more_quotes), Some(Ordering::Less));
    }

    #[test]
    fn the_evil_mode_is_not_content() {
        let data = quote_data(&[1], &[(10, 1)]);
        let blind = EvilQuoteData(data.clone(), EvilMode::Blind);
        let invalid = EvilQuoteData(data, EvilMode::InvalidFix);

        assert!(blind.content_eq(&invalid));
        assert_ne!(
            blind, invalid,
            "the modes really do differ, so the quotient is doing work"
        );
    }
}
