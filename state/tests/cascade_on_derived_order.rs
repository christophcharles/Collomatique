//! End-to-end tests: the cascade running on a *derived* document order
//!
//! `derive_content_ord.rs` checks what the macro generates. This file checks
//! that a derived order actually drives [apply_cascade] — the derive, the
//! container blankets, the `Fixable` bound and (once commit 5 lands) the
//! in-loop strictly-below check working together, with no
//! `state-colloscopes` involvement.
//!
//! The toy is deliberately isomorphic to `QuoteData` in
//! `state/src/test_utils.rs`, so the expected cascade behavior is already
//! understood; the difference is that its order is derived rather than
//! hand-written.

use std::collections::{BTreeMap, BTreeSet};

use collomatique_state::{
    ApplyError, CascadeReceipt, ContentOrd, FixOp, Fixable, InMemoryData, Operation, apply_cascade,
};

/// Authors and books: every book's author must exist.
///
/// The two container blankets give set inclusion over the authors and map
/// inclusion over the books, with atomic `u64` author ids as the values —
/// no field attribute is needed anywhere.
#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct LibraryData {
    authors: BTreeSet<u64>,
    /// book id -> author id
    books: BTreeMap<u64, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LibraryOp {
    /// Adds an author. No-clobber: fails if the id already exists.
    AddAuthor(u64),
    /// Removes an author. Fails if the id is unknown. Strands their books —
    /// an invariant break, not a precheck failure.
    RemoveAuthor(u64),
    /// Sets (or overwrites) a book row. The author is *not* prechecked.
    SetBook { book: u64, author: u64 },
    /// Removes a book row. Removing an absent book is a perfect no-op.
    RemoveBook(u64),
}

impl Operation for LibraryOp {}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
enum LibraryInvalidOp {
    #[error("unknown author {0}")]
    UnknownAuthor(u64),
    #[error("author {0} already exists")]
    AuthorExists(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
enum LibraryInvariant {
    #[error("book {0} has a dangling author")]
    DanglingBookAuthor(u64),
}

impl InMemoryData for LibraryData {
    type OriginalOperation = LibraryOp;
    type AnnotatedOperation = LibraryOp;
    type NewInfo = ();
    type InvalidOp = LibraryInvalidOp;
    type Invariant = LibraryInvariant;

    fn annotate(&mut self, op: LibraryOp) -> (LibraryOp, ()) {
        (op, ())
    }

    fn apply(
        &mut self,
        op: &LibraryOp,
    ) -> Result<LibraryOp, ApplyError<LibraryInvalidOp, LibraryInvariant>> {
        // Precheck: bad op input never touches the data (the gate's tier 1).
        match op {
            LibraryOp::AddAuthor(a) if self.authors.contains(a) => {
                return Err(ApplyError::InvalidOp(LibraryInvalidOp::AuthorExists(*a)));
            }
            LibraryOp::RemoveAuthor(a) if !self.authors.contains(a) => {
                return Err(ApplyError::InvalidOp(LibraryInvalidOp::UnknownAuthor(*a)));
            }
            _ => {}
        }

        let mut next = self.clone();
        let inverse = match op {
            LibraryOp::AddAuthor(a) => {
                next.authors.insert(*a);
                LibraryOp::RemoveAuthor(*a)
            }
            LibraryOp::RemoveAuthor(a) => {
                next.authors.remove(a);
                LibraryOp::AddAuthor(*a)
            }
            LibraryOp::SetBook { book, author } => match next.books.insert(*book, *author) {
                Some(old) => LibraryOp::SetBook {
                    book: *book,
                    author: old,
                },
                None => LibraryOp::RemoveBook(*book),
            },
            LibraryOp::RemoveBook(book) => match next.books.remove(book) {
                Some(old) => LibraryOp::SetBook {
                    book: *book,
                    author: old,
                },
                None => LibraryOp::RemoveBook(*book),
            },
        };

        // Check the whole state (the gate's tier 2).
        let broken: BTreeSet<LibraryInvariant> = next
            .books
            .iter()
            .filter(|(_, author)| !next.authors.contains(author))
            .map(|(book, _)| LibraryInvariant::DanglingBookAuthor(*book))
            .collect();
        if !broken.is_empty() {
            return Err(ApplyError::BrokenInvariants(broken));
        }

        *self = next;
        Ok(inverse)
    }
}

/// The toy's repair vocabulary. `Grow` is the one the honest map never
/// answers: it belongs to [GrowingLibraryData] below, and lives in the same
/// enum because both maps repair the same invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LibraryFix {
    /// Drop a book whose author is gone.
    RemoveBook(u64),
    /// "Repair" a dangling author by creating them — a contract violation, and
    /// the point of [GrowingLibraryData].
    AddAuthor(u64),
}

impl FixOp for LibraryFix {
    type Op = LibraryOp;

    fn to_annotated_op(&self) -> LibraryOp {
        match self {
            LibraryFix::RemoveBook(book) => LibraryOp::RemoveBook(*book),
            LibraryFix::AddAuthor(author) => LibraryOp::AddAuthor(*author),
        }
    }
}

impl Fixable for LibraryData {
    type Fix = LibraryFix;

    fn fix_invariant(&self, invariant: &LibraryInvariant) -> Option<LibraryFix> {
        match invariant {
            // Presence of the removable material: `Some` only when the book
            // row actually exists in the current state.
            LibraryInvariant::DanglingBookAuthor(book) => self
                .books
                .contains_key(book)
                .then(|| LibraryFix::RemoveBook(*book)),
        }
    }
}

/// The same data with a *growing* resolution map: it "fixes" a dangling book
/// by inventing the missing author. Every op it returns lands strictly
/// **above** the pre-fix state, which is exactly the violation step 6.5
/// exists to catch — without the in-loop check it is an infinite loop, not a
/// failure.
///
/// Commit 5 adds the panic test that drives it; the type lives here from
/// commit 2 so both commits touch one file.
#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct GrowingLibraryData {
    inner: LibraryData,
}

impl InMemoryData for GrowingLibraryData {
    type OriginalOperation = LibraryOp;
    type AnnotatedOperation = LibraryOp;
    type NewInfo = ();
    type InvalidOp = LibraryInvalidOp;
    type Invariant = LibraryInvariant;

    fn annotate(&mut self, op: LibraryOp) -> (LibraryOp, ()) {
        self.inner.annotate(op)
    }

    fn apply(
        &mut self,
        op: &LibraryOp,
    ) -> Result<LibraryOp, ApplyError<LibraryInvalidOp, LibraryInvariant>> {
        self.inner.apply(op)
    }
}

impl Fixable for GrowingLibraryData {
    type Fix = LibraryFix;

    fn fix_invariant(&self, invariant: &LibraryInvariant) -> Option<LibraryFix> {
        let LibraryInvariant::DanglingBookAuthor(book) = invariant;
        self.inner.books.get(book).map(|author| {
            // Adding the missing author repairs the invariant and grows the
            // document: a contract violation, not a fix.
            LibraryFix::AddAuthor(*author)
        })
    }
}

fn library(authors: &[u64], books: &[(u64, u64)]) -> LibraryData {
    LibraryData {
        authors: authors.iter().copied().collect(),
        books: books.iter().copied().collect(),
    }
}

/// The forward op of every landed step, in order — fixes first, target last.
fn forward_ops(receipt: CascadeReceipt<LibraryData>) -> Vec<LibraryOp> {
    receipt
        .into_aggregated_op()
        .inner()
        .iter()
        .map(|r| r.inner().clone())
        .collect()
}

#[test]
fn a_cascade_repairs_through_a_derived_order() {
    let mut data = library(&[1], &[(10, 1), (20, 1)]);
    let (target, ()) = data.annotate(LibraryOp::RemoveAuthor(1));

    let receipt = apply_cascade(&mut data, target).expect("cascade resolves");

    assert_eq!(
        forward_ops(receipt),
        vec![
            LibraryOp::RemoveBook(10),
            LibraryOp::RemoveBook(20),
            LibraryOp::RemoveAuthor(1),
        ],
    );
    assert_eq!(data, library(&[], &[]));
}

#[test]
fn every_landed_fix_is_strictly_below_its_pre_fix_state() {
    // The obligation the engine will assert in-flight (commit 5), checked
    // here by hand on the states the honest map actually walks through.
    let start = library(&[1], &[(10, 1), (20, 1)]);
    let after_first_fix = library(&[1], &[(20, 1)]);
    let after_second_fix = library(&[1], &[]);

    assert!(after_first_fix.content_lt(&start));
    assert!(after_second_fix.content_lt(&after_first_fix));
}

#[test]
fn the_growing_maps_answer_lands_strictly_above() {
    // A state the gate would never commit — built by hand, because
    // `fix_invariant` only reads it. The growing map answers `AddAuthor`,
    // and applying that lands *above* the pre-fix state: the violation the
    // in-loop check catches once commit 5 lands.
    let dangling = GrowingLibraryData {
        inner: library(&[], &[(10, 1)]),
    };
    let fix = dangling
        .fix_invariant(&LibraryInvariant::DanglingBookAuthor(10))
        .expect("the growing map always answers");
    assert_eq!(fix, LibraryFix::AddAuthor(1));
    assert_eq!(fix.to_annotated_op(), LibraryOp::AddAuthor(1));

    let mut after = dangling.clone();
    after.inner.authors.insert(1);
    assert!(!after.content_lt(&dangling), "the fix is not below");
    assert!(
        dangling.content_lt(&after),
        "it is strictly above: the state grew"
    );
}

#[test]
#[should_panic(expected = "did not land strictly below")]
fn a_growing_fix_through_a_derived_order_panics() {
    // The same growing map, now driven through the engine. The starting state
    // is built by hand (the gate would never commit a dangling book), so that
    // the map has a book row to read its invented author from.
    let mut data = GrowingLibraryData {
        inner: library(&[], &[(10, 1)]),
    };
    // Any target at all re-raises DanglingBookAuthor(10): the checker scans
    // the whole state, not just what the op touched.
    let (target, ()) = data.annotate(LibraryOp::AddAuthor(0));

    // The fix `AddAuthor(1)` applies cleanly and repairs the invariant — but
    // it lands *above* the pre-fix state, and the derived order says so.
    let _ = apply_cascade(&mut data, target);
}

/// A tiny deterministic op-walk: a linear congruential step over a `u64`
/// seed selects the op kind and the ids from its bits. No new dependency,
/// so no `Cargo.lock`/`cargoHash` churn.
struct Walk(u64);

impl Walk {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn op(&mut self) -> LibraryOp {
        let bits = self.next();
        // Small id spaces, so books and authors keep colliding — which is
        // what makes the cascade fire.
        let a = (bits >> 8) % 4;
        let b = (bits >> 24) % 6;
        match bits % 4 {
            0 => LibraryOp::AddAuthor(a),
            1 => LibraryOp::RemoveAuthor(a),
            2 => LibraryOp::SetBook { book: b, author: a },
            _ => LibraryOp::RemoveBook(b),
        }
    }
}

#[test]
fn a_deterministic_walk_never_panics_and_errors_are_atomic() {
    let mut data = library(&[], &[]);
    let mut walk = Walk(0x5eed);
    let mut cascaded = 0usize;
    let mut errored = 0usize;

    for _ in 0..500 {
        let before = data.clone();
        let (target, ()) = data.annotate(walk.op());
        let expected_target = target.clone();

        match apply_cascade(&mut data, target) {
            Ok(receipt) => {
                let ops = forward_ops(receipt);
                assert_eq!(
                    ops.last(),
                    Some(&expected_target),
                    "the target op lands last, after its repairs"
                );
                if ops.len() > 1 {
                    cascaded += 1;
                }
            }
            Err(_) => {
                errored += 1;
                assert_eq!(data, before, "a rejected cascade restores the state");
            }
        }
    }

    // The commit-8 lesson: a walk that never cascades proves nothing.
    assert!(cascaded > 0, "no landing ever needed a fix");
    assert!(errored > 0, "no landing was ever rejected");
}
