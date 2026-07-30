//! The document order: building blocks, the [ContentOrd] trait and the
//! [ContentIdentity] marker (design doc §8, step 6.5).
//!
//! The standard library's `PartialOrd`/`Ord` on containers is lexicographic
//! and is NOT what a removal-shaped order needs (removing an element can
//! make a set sort *later*). The document order therefore lives on its own
//! trait, with hand-picked container semantics: sets by inclusion, maps by
//! key-and-value inclusion, sequences by embedding or prefix.
//! `#[derive(ContentOrd)]` (from `collomatique-state-derive`) implements it
//! for regular structs and enums as the product of their fields.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU32, NonZeroU64};

use crate::tables::{Key, OrderedKey, OrderedTable, Table};

/// The document order: a partial order over *content* (design doc §8,
/// step 6.5, and ruling D5.1 — content, not the meaning it denotes).
///
/// This is the order of the cascade's monotonicity contract: the order is
/// well-founded, and every resolution-map fix must land strictly below the
/// pre-fix state. [crate::apply_cascade] checks that in-flight. The order
/// is intrinsic to the data type — it is defined from the structure alone,
/// and the resolution map is *held to it*, never the other way around.
///
/// # Laws (self-contained: no law mentions `PartialEq`)
///
/// * `Some(Ordering::Equal)` is an equivalence relation — **content
///   equivalence** (see [ContentOrd::content_eq]). It may be coarser than a
///   type's `==`: a type may quotient away non-content fields (an id
///   issuer, a test-harness mode). No law relates it to `PartialEq` —
///   `PartialEq` never promises structural equality, so nothing here may
///   lean on it. The provided building blocks ([discrete] and the blankets)
///   *define* content equivalence from `==` for the types they cover; a
///   type assembled from them inherits whatever `==` means on its leaves.
/// * `content_cmp` is a partial order up to that equivalence: reflexive
///   (`x.content_cmp(&x) == Some(Equal)`), transitive, and antisymmetric up
///   to equivalence.
/// * **Well-foundedness on document data**: every strict decrease happens in
///   a well-founded coordinate — removing an element from a finite container,
///   moving an `Option` from `Some` to `None`, or stepping down a field whose
///   own order admits no infinite descending chain — so there is no infinite
///   strictly-decreasing chain, and strict monotonicity of fixes is a
///   termination proof.
///
/// This is deliberately *not* `PartialOrd`: the standard library implements
/// `PartialOrd` lexicographically on containers (under which removing an
/// element can make a set sort *later*), and the typed ids must keep their
/// numeric `Ord` for use as map keys. A distinct trait keeps both worlds
/// intact and unambiguous.
pub trait ContentOrd {
    /// Compares two values in the document order.
    fn content_cmp(&self, other: &Self) -> Option<Ordering>;

    /// `self` and `other` are content-equivalent. Use this, never `==`,
    /// when the question is about the document order.
    fn content_eq(&self, other: &Self) -> bool {
        self.content_cmp(other) == Some(Ordering::Equal)
    }

    /// `self` is below or equal to `other` in the document order.
    fn content_le(&self, other: &Self) -> bool {
        matches!(
            self.content_cmp(other),
            Some(Ordering::Less | Ordering::Equal)
        )
    }

    /// `self` is strictly below `other` in the document order: below and
    /// not equivalent. This is the fix obligation.
    fn content_lt(&self, other: &Self) -> bool {
        self.content_cmp(other) == Some(Ordering::Less)
    }
}

/// Marker: `==` coincides with content equivalence — this type carries no
/// content quotient, so containers may match it by `==`/`Ord`.
///
/// Inside a container, "the same element/row" can only mean `==` (for
/// `Ord`-backed storage, `Ord`'s own contract ties its `Equal` to `==`), and
/// that is sound exactly when `==` is content identity for the element/key
/// type. This requirement is *positional* — it must hold at container
/// matching positions and nowhere else (a global law would outlaw quotients
/// like `ContentOrd` on `Data`, which ignores the id issuer). The
/// container blanket impls require this marker at every matching position,
/// so a quotiented type inside a container is a compile error.
///
/// Deliberately opt-in: entity structs whose equivalence happens to equal
/// `==` today still do not get the marker unless they need it — "safe to
/// match by `==`" stays an explicit, auditable assertion. Enrollment paths:
/// the atom macros emit it together with [ContentOrd] (an atom's
/// equivalence *is* `==` by construction), tuples of markers are markers,
/// and composite types use `#[derive(ContentIdentity)]` or a hand-written
/// impl.
pub trait ContentIdentity: Eq {}

/// Product order: combines per-field comparisons. `Equal` is neutral; two
/// fields pulling in opposite directions, or any incomparable field, make
/// the whole product incomparable (`None`).
pub fn combine(fields: impl IntoIterator<Item = Option<Ordering>>) -> Option<Ordering> {
    let mut acc = Ordering::Equal;
    for field in fields {
        match field? {
            Ordering::Equal => {}
            ord if acc == Ordering::Equal => acc = ord,
            ord if ord == acc => {}
            _ => return None,
        }
    }
    Some(acc)
}

/// Discrete order: comparable only when equal.
///
/// `Eq`, not `PartialEq`: the document order's reflexivity law rests on the
/// leaf's `==` being reflexive — a `PartialEq`-only type like `f64` would
/// break it through `NaN != NaN`. The bound puts that obligation in the
/// type system.
pub fn discrete<T: Eq + ?Sized>(a: &T, b: &T) -> Option<Ordering> {
    (a == b).then_some(Ordering::Equal)
}

/// Option lift: `None` is the bottom, two `Some` values compare by `inner`.
pub fn option_lift<T>(
    a: &Option<T>,
    b: &Option<T>,
    inner: impl FnOnce(&T, &T) -> Option<Ordering>,
) -> Option<Ordering> {
    match (a, b) {
        (None, None) => Some(Ordering::Equal),
        (None, Some(_)) => Some(Ordering::Less),
        (Some(_), None) => Some(Ordering::Greater),
        (Some(x), Some(y)) => inner(x, y),
    }
}

/// Set inclusion: strict subset is strictly below.
pub fn set_inclusion<T: Ord>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> Option<Ordering> {
    match (a.is_subset(b), b.is_subset(a)) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

/// Map inclusion with a value rule: `a` is below `b` iff `a`'s keys are
/// included in `b`'s and every shared key's value is below or equal.
pub fn map_inclusion<K: Ord, V>(
    a: &BTreeMap<K, V>,
    b: &BTreeMap<K, V>,
    value_cmp: impl Fn(&V, &V) -> Option<Ordering>,
) -> Option<Ordering> {
    let a_in_b = a.keys().all(|k| b.contains_key(k));
    let b_in_a = b.keys().all(|k| a.contains_key(k));
    let keys = match (a_in_b, b_in_a) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => return None,
    };
    combine(
        std::iter::once(Some(keys)).chain(
            a.iter()
                .filter_map(|(k, va)| b.get(k).map(|vb| value_cmp(va, vb))),
        ),
    )
}

/// Sequence embedding: a strict subsequence — obtainable by *deleting*
/// elements, the survivors keeping their relative order; contiguity is NOT
/// required, so `[1,3]` is a subsequence of `[1,2,3]` and compares `Less`.
/// A reordering is incomparable. Elements are matched by `==` (`Eq` for the
/// same reflexivity reason as [discrete]).
pub fn subsequence<T: Eq>(a: &[T], b: &[T]) -> Option<Ordering> {
    fn embeds<T: Eq>(small: &[T], big: &[T]) -> bool {
        let mut rest = big.iter();
        small.iter().all(|x| rest.any(|y| y == x))
    }
    match (embeds(a, b), embeds(b, a)) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

/// Prefix-pointwise order: the rule for sequences whose *positions* carry
/// the identity — the `Vec` read as a map from an initial segment of
/// indices, i.e. [map_inclusion] specialized. The length comparison plays
/// the key-set role (appending is adding content, the empty vector is the
/// minimum) and shared indices compare pointwise; removing a middle element
/// shifts every later element's identity and comes out incomparable.
pub fn prefix_pointwise<T>(
    a: &[T],
    b: &[T],
    cmp: impl Fn(&T, &T) -> Option<Ordering>,
) -> Option<Ordering> {
    combine(
        std::iter::once(Some(a.len().cmp(&b.len()))).chain(a.iter().zip(b).map(|(x, y)| cmp(x, y))),
    )
}

/// `Option` lift with a discrete inner comparison — for
/// `#[ord(with = option_lift_discrete)]` on an `Option` whose inner type is
/// foreign (`Option<NonEmptyString>`, `Option<WeekStart>`, …).
pub fn option_lift_discrete<T: Eq>(a: &Option<T>, b: &Option<T>) -> Option<Ordering> {
    option_lift(a, b, |x, y| discrete(x, y))
}

/// Sequence embedding with elements matched discretely (by `==`) — the `Vec`
/// analogue of [option_lift_discrete], for `#[ord(with = vec_subsequence)]`
/// on a `Vec` whose element type is foreign. (For an *enrolled*
/// [ContentIdentity] element type the blanket `Vec` impl already gives
/// exactly this behavior.)
pub fn vec_subsequence<T: Eq>(a: &Vec<T>, b: &Vec<T>) -> Option<Ordering> {
    subsequence(a, b)
}

/// Prefix-pointwise comparison through [ContentOrd] — for
/// `#[ord(with = vec_prefix)]` where element identity is positional
/// (prefilled groups). Positional elements are never matched by `==`, so no
/// [ContentIdentity] is required of them — deliberately: a positional
/// element type *without* the marker also keeps the `Vec` blanket from
/// applying, which turns a forgotten attribute into a compile error.
pub fn vec_prefix<T: ContentOrd>(a: &Vec<T>, b: &Vec<T>) -> Option<Ordering> {
    prefix_pointwise(a, b, ContentOrd::content_cmp)
}

macro_rules! impl_atoms {
    ($($t:ty),* $(,)?) => { $(
        impl ContentOrd for $t {
            fn content_cmp(&self, other: &Self) -> Option<Ordering> {
                discrete(self, other)
            }
        }
        // An atom's content equivalence IS `==`, so `==` is content
        // identity and the type may be matched inside containers.
        impl ContentIdentity for $t {}
    )* };
}

impl_atoms!(
    (),
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    String,
    NonZeroU32,
    NonZeroU64,
);

/// Enrolls local types into the document order as atoms (discretely
/// compared: equal or incomparable), together with
/// [ContentIdentity][crate::ContentIdentity] — an atom's content
/// equivalence is `==` by construction, so the marker is always truthful
/// for it. For foreign types use `#[ord(atom)]` on the field instead; for
/// generic types write the impls by hand.
#[macro_export]
macro_rules! impl_content_ord_atom {
    ($($t:ty),* $(,)?) => { $(
        impl $crate::partial_order::ContentOrd for $t {
            fn content_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                $crate::partial_order::discrete(self, other)
            }
        }
        impl $crate::partial_order::ContentIdentity for $t {}
    )* };
}

// A tuple of content identities is a content identity: tuple `==` is
// component-wise `==`, which coincides with component-wise content
// equivalence by the components' own markers.
impl<A: ContentIdentity, B: ContentIdentity> ContentIdentity for (A, B) {}
impl<A: ContentIdentity, B: ContentIdentity, C: ContentIdentity> ContentIdentity for (A, B, C) {}

impl<T: ContentOrd> ContentOrd for Option<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        option_lift(self, other, ContentOrd::content_cmp)
    }
}

// Set elements are matched by `Ord`, whose `Equal` is `==` by Rust's own
// contract — sound exactly when `==` is content identity for the element
// type, hence the marker bound. A quotiented element type does not compile.
impl<T: Ord + ContentIdentity> ContentOrd for BTreeSet<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        set_inclusion(self, other)
    }
}

// Keys are row identity; the marker bound is the same argument as for sets.
impl<K: Ord + ContentIdentity, V: ContentOrd> ContentOrd for BTreeMap<K, V> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        map_inclusion(self, other, ContentOrd::content_cmp)
    }
}

/// Sequence embedding — the value-borne-identity reading, which is the
/// common case reached through trait dispatch (id lists inside table
/// values). Elements are matched by `==`, hence the [ContentIdentity]
/// bound; it doubles as a safety net: a structured element type without
/// the marker (e.g. `PrefilledGroup`) does not dispatch at all, so the
/// field demands an explicit `#[ord(...)]` attribute instead of silently
/// getting a wrong rule. Where identity is positional, use
/// `#[ord(with = vec_prefix)]`; where the list is a relational chain,
/// `#[ord(atom)]` (the identity criterion of the step-6.5 plan, §2).
impl<T: ContentOrd + ContentIdentity> ContentOrd for Vec<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        subsequence(self, other)
    }
}

// Table keys are row identity — same marker argument as `BTreeMap`.
impl<I: Key + ContentIdentity, T: ContentOrd> ContentOrd for Table<I, T> {
    // Same order as `map_inclusion` with a `combine`d value rule, expressed
    // through `Table`'s public surface (`keys`/`contains`/`get`/`iter`) — keep
    // the two in step if either changes.
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_in_other = self.keys().all(|k| other.contains(&k));
        let other_in_self = other.keys().all(|k| self.contains(&k));
        let keys = match (self_in_other, other_in_self) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => return None,
        };
        combine(
            std::iter::once(Some(keys)).chain(
                self.iter()
                    .filter_map(|(k, v)| other.get(&k).map(|w| v.content_cmp(w))),
            ),
        )
    }
}

impl<I: OrderedKey + ContentIdentity, T: ContentOrd> ContentOrd for OrderedTable<I, T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_keys: Vec<I> = self.keys().collect();
        let other_keys: Vec<I> = other.keys().collect();
        let keys = subsequence(&self_keys, &other_keys)?;
        combine(
            std::iter::once(Some(keys)).chain(
                self.iter()
                    .filter_map(|(k, v)| other.get(&k).map(|w| v.content_cmp(w))),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A local id type for the [Table] / [OrderedTable] dispatch tests.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct TestId(u64);

    impl ContentOrd for TestId {
        fn content_cmp(&self, other: &Self) -> Option<Ordering> {
            discrete(self, other)
        }
    }
    impl ContentIdentity for TestId {}

    fn set(values: impl IntoIterator<Item = u64>) -> BTreeSet<u64> {
        values.into_iter().collect()
    }

    fn map(entries: impl IntoIterator<Item = (u64, u32)>) -> BTreeMap<u64, u32> {
        entries.into_iter().collect()
    }

    // --- combine ---------------------------------------------------------

    #[test]
    fn combine_empty_is_equal() {
        assert_eq!(combine(std::iter::empty()), Some(Ordering::Equal));
    }

    #[test]
    fn combine_all_equal_is_equal() {
        let fields = [Some(Ordering::Equal), Some(Ordering::Equal)];
        assert_eq!(combine(fields), Some(Ordering::Equal));
    }

    #[test]
    fn combine_one_less_among_equals_is_less() {
        let fields = [
            Some(Ordering::Equal),
            Some(Ordering::Less),
            Some(Ordering::Equal),
        ];
        assert_eq!(combine(fields), Some(Ordering::Less));
    }

    #[test]
    fn combine_two_less_is_less() {
        let fields = [Some(Ordering::Less), Some(Ordering::Less)];
        assert_eq!(combine(fields), Some(Ordering::Less));
    }

    #[test]
    fn combine_mixed_directions_is_incomparable() {
        let fields = [Some(Ordering::Less), Some(Ordering::Greater)];
        assert_eq!(combine(fields), None);
    }

    #[test]
    fn combine_any_incomparable_field_is_incomparable() {
        let fields = [Some(Ordering::Less), None, Some(Ordering::Less)];
        assert_eq!(combine(fields), None);
    }

    // --- discrete and option_lift ----------------------------------------

    #[test]
    fn discrete_is_equal_or_incomparable() {
        assert_eq!(discrete(&3u64, &3u64), Some(Ordering::Equal));
        assert_eq!(discrete(&3u64, &4u64), None);
        assert_eq!(discrete(&4u64, &3u64), None);
    }

    #[test]
    fn option_lift_all_four_arms() {
        let none: Option<u64> = None;
        assert_eq!(
            option_lift(&none, &none, discrete),
            Some(Ordering::Equal),
            "None vs None"
        );
        assert_eq!(
            option_lift(&none, &Some(1u64), discrete),
            Some(Ordering::Less),
            "None is the bottom"
        );
        assert_eq!(
            option_lift(&Some(1u64), &none, discrete),
            Some(Ordering::Greater),
            "Some is above None"
        );
        assert_eq!(
            option_lift(&Some(1u64), &Some(1u64), discrete),
            Some(Ordering::Equal),
            "two Some values compare by the inner rule"
        );
        assert_eq!(
            option_lift(&Some(1u64), &Some(2u64), discrete),
            None,
            "two Some values compare by the inner rule"
        );
    }

    // --- set inclusion ---------------------------------------------------

    #[test]
    fn set_inclusion_arms() {
        assert_eq!(
            set_inclusion(&set([1, 2]), &set([1, 2])),
            Some(Ordering::Equal)
        );
        assert_eq!(
            set_inclusion(&set([1, 2]), &set([1, 2, 3])),
            Some(Ordering::Less)
        );
        assert_eq!(
            set_inclusion(&set([1, 2, 3]), &set([1, 2])),
            Some(Ordering::Greater)
        );
        assert_eq!(set_inclusion(&set([1, 2]), &set([2, 3])), None);
    }

    /// The trap that motivates a dedicated trait: the standard library's
    /// `Ord` on sets is lexicographic, so removing an element can make a set
    /// sort *later*. The document order says the opposite, which is the
    /// whole point.
    #[test]
    fn set_inclusion_contradicts_std_lexicographic_order() {
        let smaller = set([1, 3]);
        let bigger = set([1, 2, 3]);

        assert_eq!(set_inclusion(&smaller, &bigger), Some(Ordering::Less));
        assert_eq!(smaller.cmp(&bigger), Ordering::Greater);
    }

    // --- map inclusion ---------------------------------------------------

    #[test]
    fn map_inclusion_key_subset_with_equal_values_is_less() {
        let a = map([(1, 10), (2, 20)]);
        let b = map([(1, 10), (2, 20), (3, 30)]);
        assert_eq!(map_inclusion(&a, &b, discrete), Some(Ordering::Less));
    }

    #[test]
    fn map_inclusion_same_keys_one_value_less_is_less() {
        let a = map([(1, 10), (2, 20)]);
        let b = map([(1, 10), (2, 20)]);
        let value_cmp = |x: &u32, y: &u32| if x == y { Some(Ordering::Equal) } else { None };
        assert_eq!(map_inclusion(&a, &b, value_cmp), Some(Ordering::Equal));

        // one shared value strictly below, keys equal
        let with_less = |x: &u32, y: &u32| Some(x.cmp(y));
        let c = map([(1, 10), (2, 19)]);
        assert_eq!(map_inclusion(&c, &b, with_less), Some(Ordering::Less));
    }

    #[test]
    fn map_inclusion_key_subset_but_greater_value_is_incomparable() {
        let a = map([(1, 11)]);
        let b = map([(1, 10), (2, 20)]);
        let with_less = |x: &u32, y: &u32| Some(x.cmp(y));
        assert_eq!(map_inclusion(&a, &b, with_less), None);
    }

    #[test]
    fn map_inclusion_crossed_keys_is_incomparable() {
        let a = map([(1, 10)]);
        let b = map([(2, 20)]);
        assert_eq!(map_inclusion(&a, &b, discrete), None);
    }

    // --- subsequence -----------------------------------------------------

    #[test]
    fn subsequence_is_not_contiguous() {
        assert_eq!(
            subsequence(&[1u64, 3], &[1u64, 2, 3]),
            Some(Ordering::Less),
            "deleting a middle element keeps a subsequence"
        );
    }

    #[test]
    fn subsequence_reorder_is_incomparable() {
        assert_eq!(subsequence(&[2u64, 1], &[1u64, 2]), None);
    }

    #[test]
    fn subsequence_equal_and_empty() {
        assert_eq!(subsequence(&[1u64, 2], &[1u64, 2]), Some(Ordering::Equal));
        assert_eq!(subsequence::<u64>(&[], &[]), Some(Ordering::Equal));
        assert_eq!(subsequence(&[], &[1u64, 2]), Some(Ordering::Less));
        assert_eq!(subsequence(&[1u64, 2], &[]), Some(Ordering::Greater));
    }

    // --- prefix pointwise ------------------------------------------------

    #[test]
    fn prefix_pointwise_truncation_is_less() {
        assert_eq!(
            prefix_pointwise(&[1u64], &[1u64, 2], discrete),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn prefix_pointwise_equal_length_with_one_decrease_is_less() {
        let with_less = |x: &u64, y: &u64| Some(x.cmp(y));
        assert_eq!(
            prefix_pointwise(&[1u64, 1], &[1u64, 2], with_less),
            Some(Ordering::Less)
        );
    }

    /// The rule that separates positional identity from value identity:
    /// removing a *middle* element shifts every later element's index, so
    /// the two lists are incomparable — while [subsequence] calls the very
    /// same pair `Less`.
    #[test]
    fn prefix_pointwise_middle_removal_is_incomparable() {
        assert_eq!(prefix_pointwise(&[1u64, 3], &[1u64, 2, 3], discrete), None);
        assert_eq!(subsequence(&[1u64, 3], &[1u64, 2, 3]), Some(Ordering::Less));
    }

    #[test]
    fn prefix_pointwise_mixed_directions_is_incomparable() {
        let with_less = |x: &u64, y: &u64| Some(x.cmp(y));
        assert_eq!(prefix_pointwise(&[0u64, 5], &[1u64, 4], with_less), None);
    }

    #[test]
    fn prefix_pointwise_empty_is_below_anything() {
        assert_eq!(
            prefix_pointwise(&[], &[1u64, 2], discrete),
            Some(Ordering::Less)
        );
        assert_eq!(
            prefix_pointwise::<u64>(&[], &[], discrete),
            Some(Ordering::Equal)
        );
    }

    // --- the with = helpers ----------------------------------------------

    #[test]
    fn option_lift_discrete_helper() {
        assert_eq!(
            option_lift_discrete(&None, &Some(String::from("a"))),
            Some(Ordering::Less)
        );
        assert_eq!(
            option_lift_discrete(&Some(String::from("a")), &Some(String::from("b"))),
            None
        );
    }

    #[test]
    fn vec_subsequence_helper() {
        assert_eq!(
            vec_subsequence(&vec![1u64, 3], &vec![1u64, 2, 3]),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn vec_prefix_helper_dispatches_into_content_ord() {
        // `Option<u64>` is a ContentOrd but *not* a ContentIdentity, so this
        // is exactly the positional shape: un-setting a slot is a decrease,
        // truncating is a decrease, a middle removal is incomparable.
        let full = vec![Some(1u64), Some(2u64)];
        let cleared = vec![Some(1u64), None];
        let shorter = vec![Some(1u64)];

        assert_eq!(vec_prefix(&cleared, &full), Some(Ordering::Less));
        assert_eq!(vec_prefix(&shorter, &full), Some(Ordering::Less));
        assert_eq!(vec_prefix(&full, &full), Some(Ordering::Equal));
        assert_eq!(vec_prefix(&vec![Some(2u64)], &full), None);
    }

    // --- the trait's default methods -------------------------------------

    #[test]
    fn default_methods() {
        let a = set([1u64, 2]);
        let b = set([1u64, 2, 3]);

        assert!(a.content_lt(&b));
        assert!(a.content_le(&b));
        assert!(!a.content_eq(&b));

        assert!(a.content_eq(&a.clone()));
        assert!(a.content_le(&a.clone()));
        assert!(
            !a.content_lt(&a.clone()),
            "strictly below means below AND not equivalent"
        );

        assert!(!b.content_lt(&a));
        assert!(!b.content_le(&a));
    }

    #[test]
    fn reflexivity_on_a_compound_value() {
        let x: BTreeMap<u64, BTreeSet<u64>> =
            [(1, set([1, 2])), (2, set([3]))].into_iter().collect();
        assert_eq!(x.content_cmp(&x.clone()), Some(Ordering::Equal));
    }

    // --- blanket dispatch smoke tests ------------------------------------

    #[test]
    fn option_blanket() {
        let none: Option<u32> = None;
        assert_eq!(none.content_cmp(&Some(3)), Some(Ordering::Less));
        assert_eq!(Some(3u32).content_cmp(&Some(4u32)), None);
        assert_eq!(Some(3u32).content_cmp(&Some(3u32)), Some(Ordering::Equal));
    }

    #[test]
    fn vec_blanket_is_subsequence() {
        assert_eq!(
            vec![1u32, 3].content_cmp(&vec![1u32, 2, 3]),
            Some(Ordering::Less)
        );
        assert_eq!(vec![3u32, 1].content_cmp(&vec![1u32, 3]), None);
    }

    #[test]
    fn btree_map_blanket_renumbering_a_value_is_incomparable() {
        let a: BTreeMap<u64, u32> = map([(1, 10)]);
        let b: BTreeMap<u64, u32> = map([(1, 11)]);
        assert_eq!(a.content_cmp(&b), None);

        let c: BTreeMap<u64, u32> = map([(1, 10), (2, 20)]);
        assert_eq!(a.content_cmp(&c), Some(Ordering::Less));
    }

    #[test]
    fn table_blanket() {
        let full: Table<TestId, BTreeSet<u64>> = [(TestId(1), set([1, 2])), (TestId(2), set([3]))]
            .into_iter()
            .collect();
        let row_removed: Table<TestId, BTreeSet<u64>> =
            [(TestId(1), set([1, 2]))].into_iter().collect();
        let value_shrunk: Table<TestId, BTreeSet<u64>> =
            [(TestId(1), set([1])), (TestId(2), set([3]))]
                .into_iter()
                .collect();
        let other_id: Table<TestId, BTreeSet<u64>> =
            [(TestId(3), set([1, 2]))].into_iter().collect();

        assert_eq!(row_removed.content_cmp(&full), Some(Ordering::Less));
        assert_eq!(
            value_shrunk.content_cmp(&full),
            Some(Ordering::Less),
            "the value comparison dispatches into the value's own impl"
        );
        assert_eq!(full.content_cmp(&full.clone()), Some(Ordering::Equal));
        assert_eq!(
            other_id.content_cmp(&row_removed),
            None,
            "the same content under a different id is incomparable"
        );
    }

    #[test]
    fn ordered_table_blanket() {
        let full: OrderedTable<TestId, u32> =
            OrderedTable::try_from(vec![(TestId(1), 10), (TestId(2), 20), (TestId(3), 30)])
                .unwrap();
        let removed: OrderedTable<TestId, u32> =
            OrderedTable::try_from(vec![(TestId(1), 10), (TestId(3), 30)]).unwrap();
        let reordered: OrderedTable<TestId, u32> =
            OrderedTable::try_from(vec![(TestId(3), 30), (TestId(1), 10), (TestId(2), 20)])
                .unwrap();
        let value_changed: OrderedTable<TestId, u32> =
            OrderedTable::try_from(vec![(TestId(1), 11), (TestId(2), 20), (TestId(3), 30)])
                .unwrap();

        assert_eq!(
            removed.content_cmp(&full),
            Some(Ordering::Less),
            "a middle row may leave: the key list stays a subsequence"
        );
        assert_eq!(full.content_cmp(&full.clone()), Some(Ordering::Equal));
        assert_eq!(
            reordered.content_cmp(&full),
            None,
            "ordering is user-visible data: a reorder is incomparable"
        );
        assert_eq!(
            value_changed.content_cmp(&full),
            None,
            "the value comparison dispatches into the value's own impl"
        );
    }

    #[test]
    fn composite_keys_via_the_tuple_marker() {
        // The assignments / colloscope shape: a table keyed by a pair.
        let full: Table<(u64, u64), BTreeSet<u64>> = [((1, 1), set([1, 2])), ((1, 2), set([3]))]
            .into_iter()
            .collect();
        let row_removed: Table<(u64, u64), BTreeSet<u64>> =
            [((1, 1), set([1, 2]))].into_iter().collect();

        assert_eq!(row_removed.content_cmp(&full), Some(Ordering::Less));
        assert_eq!(full.content_cmp(&full.clone()), Some(Ordering::Equal));
    }
}
