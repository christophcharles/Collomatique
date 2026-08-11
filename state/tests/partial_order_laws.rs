//! The [ContentOrd] laws, checked over generated universes
//!
//! `partial_order.rs`'s own unit tests are example-tested: each one names a
//! pair and the answer it expects. That catches a wrong *arm*, but not a rule
//! that is subtly not a partial order — and the compositions where such a
//! break would be least obvious are exactly the ones with two things going on
//! at once: `prefix_pointwise` (a length comparison combined with a pointwise
//! one) and the [OrderedTable] order (a key subsequence combined with a
//! pointwise value comparison).
//!
//! So this file enumerates a small universe per shape and checks the laws
//! stated on the trait (`partial_order.rs`, "# Laws") on every pair and
//! triple:
//!
//! * **reflexivity** — `a.content_cmp(&a) == Some(Equal)`;
//! * **mutual inverse** — `content_cmp(b, a)` is the reverse of
//!   `content_cmp(a, b)` (the symmetry half of antisymmetry, and what makes
//!   `Greater` mean anything at all);
//! * **transitivity** — `a ≤ b` and `b ≤ c` imply `a ≤ c`;
//! * **antisymmetry up to equivalence** — `a ≤ b` and `b ≤ a` imply
//!   `content_eq(a, b)`.
//!
//! The universes stay small (a few dozen values each) because the triple loop
//! is cubic; that is still tens of thousands of comparisons per shape, which
//! runs in well under a second. Nothing is printed.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use collomatique_state::partial_order::vec_prefix;
use collomatique_state::{ContentIdentity, ContentOrd, OrderedTable};

/// Checks the four laws over every pair and triple of `universe`.
///
/// `shape` names the universe in the failure message: with six shapes going
/// through the same body, the assertion text alone would not say which one
/// broke.
fn check_laws<T: ContentOrd + std::fmt::Debug>(shape: &str, universe: &[T]) {
    assert!(
        universe.len() >= 8,
        "{shape}: the universe is too small to say anything"
    );

    for a in universe {
        assert_eq!(
            a.content_cmp(a),
            Some(Ordering::Equal),
            "{shape}: reflexivity on {a:?}"
        );
    }

    for a in universe {
        for b in universe {
            assert_eq!(
                b.content_cmp(a),
                a.content_cmp(b).map(Ordering::reverse),
                "{shape}: mutual inverse on {a:?} and {b:?}"
            );
            if a.content_le(b) && b.content_le(a) {
                assert!(
                    a.content_eq(b),
                    "{shape}: antisymmetry up to equivalence on {a:?} and {b:?}"
                );
            }
        }
    }

    for a in universe {
        for b in universe {
            if !a.content_le(b) {
                continue;
            }
            for c in universe {
                if !b.content_le(c) {
                    continue;
                }
                assert!(
                    a.content_le(c),
                    "{shape}: transitivity on {a:?} ≤ {b:?} ≤ {c:?}"
                );
            }
        }
    }
}

/// A tiny deterministic generator: a linear congruential step over a `u64`
/// seed, whose bits drive every choice below. Same pattern as
/// `cascade_on_derived_order.rs` — no new dependency, so no `Cargo.lock` or
/// `cargoHash` churn.
struct Walk(u64);

impl Walk {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    /// A value in `0..n`.
    fn upto(&mut self, n: u64) -> u64 {
        (self.next() >> 16) % n
    }

    /// A tiny value space on purpose: keys and elements must keep colliding
    /// across the universe, or nearly every pair comes out incomparable and
    /// the laws hold vacuously.
    fn small(&mut self) -> u8 {
        self.upto(4) as u8
    }
}

/// The id type for the [OrderedTable] shape.
///
/// Hand-written impls: the derives do not accept tuple structs, and this is
/// exactly what the atom macros emit for an entity id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TestId(u8);

impl ContentOrd for TestId {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        (self == other).then_some(Ordering::Equal)
    }
}
impl ContentIdentity for TestId {}

/// A two-field derived struct: the product rule over two different container
/// orders, which is where `combine`'s "opposite directions are incomparable"
/// arm has to hold the laws together.
#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct Record {
    tags: BTreeSet<u8>,
    slots: BTreeMap<u8, Option<u8>>,
}

/// A `Vec` whose element identity is *positional*, i.e. `vec_prefix`.
#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct Positional {
    #[ord(with = vec_prefix)]
    cells: Vec<Option<u8>>,
}

fn gen_set(walk: &mut Walk) -> BTreeSet<u8> {
    let len = walk.upto(4);
    (0..len).map(|_| walk.small()).collect()
}

fn gen_map(walk: &mut Walk) -> BTreeMap<u8, Option<u8>> {
    let len = walk.upto(4);
    (0..len)
        .map(|_| {
            let key = walk.small();
            // A `None` value is strictly below a `Some`, so the map shape
            // exercises `map_inclusion` with a non-discrete value rule —
            // keys and values can disagree, which is the interesting case.
            let value = (walk.upto(3) != 0).then(|| walk.small());
            (key, value)
        })
        .collect()
}

fn gen_vec(walk: &mut Walk) -> Vec<u8> {
    let len = walk.upto(4);
    (0..len).map(|_| walk.small()).collect()
}

fn gen_cells(walk: &mut Walk) -> Positional {
    let len = walk.upto(4);
    Positional {
        cells: (0..len)
            .map(|_| (walk.upto(3) != 0).then(|| walk.small()))
            .collect(),
    }
}

fn gen_table(walk: &mut Walk) -> OrderedTable<TestId, BTreeSet<u8>> {
    let len = walk.upto(4);
    let mut rows: Vec<(TestId, BTreeSet<u8>)> = Vec::new();
    for _ in 0..len {
        let id = TestId(walk.small());
        // `OrderedTable::try_from` rejects duplicate keys, and the small id
        // space makes collisions common; skipping keeps the generator honest
        // (an `unwrap` here would just be a flaky panic).
        if rows.iter().all(|(existing, _)| *existing != id) {
            rows.push((id, gen_set(walk)));
        }
    }
    OrderedTable::try_from(rows).expect("the ids were deduplicated above")
}

/// Builds a universe of `count` values, then drops the duplicates that the
/// small value space inevitably produces (they cost cubic time and prove
/// nothing extra).
fn universe<T: PartialEq>(count: usize, mut make: impl FnMut(&mut Walk) -> T) -> Vec<T> {
    let mut walk = Walk(0x0dd_ba11);
    let mut values: Vec<T> = Vec::new();
    for _ in 0..count {
        let value = make(&mut walk);
        if !values.contains(&value) {
            values.push(value);
        }
    }
    values
}

#[test]
fn set_inclusion_is_a_partial_order() {
    check_laws("BTreeSet<u8>", &universe(400, gen_set));
}

#[test]
fn map_inclusion_with_option_values_is_a_partial_order() {
    check_laws("BTreeMap<u8, Option<u8>>", &universe(400, gen_map));
}

#[test]
fn vec_subsequence_is_a_partial_order() {
    check_laws("Vec<u8>", &universe(400, gen_vec));
}

#[test]
fn vec_prefix_is_a_partial_order() {
    check_laws(
        "Vec<Option<u8>> under vec_prefix",
        &universe(400, gen_cells),
    );
}

#[test]
fn the_ordered_table_order_is_a_partial_order() {
    check_laws(
        "OrderedTable<TestId, BTreeSet<u8>>",
        &universe(400, gen_table),
    );
}

#[test]
fn a_derived_product_is_a_partial_order() {
    check_laws(
        "Record",
        &universe(400, |walk| Record {
            tags: gen_set(walk),
            slots: gen_map(walk),
        }),
    );
}

/// The law checks above are only worth their runtime if the universes really
/// contain all four answers *and* a strict three-element chain. A universe of
/// pairwise-incomparable values satisfies every law vacuously, and one with no
/// `a < b < c` satisfies transitivity vacuously.
#[test]
fn the_universes_are_not_vacuous() {
    fn census<T: ContentOrd>(values: &[T]) -> (usize, usize, usize, usize) {
        let mut equal = 0;
        let mut strict = 0;
        let mut incomparable = 0;
        let mut chains = 0;
        for a in values {
            for b in values {
                match a.content_cmp(b) {
                    Some(Ordering::Equal) => equal += 1,
                    Some(_) => strict += 1,
                    None => incomparable += 1,
                }
                if !a.content_lt(b) {
                    continue;
                }
                chains += values.iter().filter(|c| b.content_lt(c)).count();
            }
        }
        (equal, strict, incomparable, chains)
    }

    for (shape, (equal, strict, incomparable, chains)) in [
        ("BTreeSet<u8>", census(&universe(400, gen_set))),
        ("BTreeMap<u8, Option<u8>>", census(&universe(400, gen_map))),
        ("Vec<u8>", census(&universe(400, gen_vec))),
        ("Vec<Option<u8>>", census(&universe(400, gen_cells))),
        ("OrderedTable", census(&universe(400, gen_table))),
        (
            "Record",
            census(&universe(400, |walk| Record {
                tags: gen_set(walk),
                slots: gen_map(walk),
            })),
        ),
    ] {
        assert!(equal > 0, "{shape}: no pair ever compared equivalent");
        assert!(strict > 0, "{shape}: no pair ever compared strictly");
        assert!(
            incomparable > 0,
            "{shape}: no pair was ever incomparable — the shape is a total order?"
        );
        assert!(
            chains > 0,
            "{shape}: no strict three-element chain, so transitivity held vacuously"
        );
    }
}
