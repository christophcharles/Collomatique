//! Integration tests for the `ContentOrd` and `ContentIdentity` derive macros
//!
//! These are integration tests (not unit tests) on purpose: the derives emit
//! absolute `::collomatique_state::…` paths, so the generated code only
//! compiles from outside the crate.
//!
//! Compile-*failure* cases are not asserted here — the crate has no
//! `trybuild` harness and this step adds no dependency (no `Cargo.lock`
//! churn). For the record, each of these is a spanned `syn::Error` at the
//! offending site: a generic type, a tuple struct, a tuple variant, a union,
//! two `#[ord(...)]` attributes on one field, an unknown attribute argument
//! (`#[ord(nonsense)]`), and `#[derive(ContentIdentity)]` over a field
//! carrying `#[ord(ignore)]` or `#[ord(with = ...)]`.
//!
//! Two more failures come from the type system rather than the macro, and
//! they are the decision-17 safety net: a `#[derive(ContentIdentity)]`
//! whose default field is not itself a `ContentIdentity` fails the emitted
//! static assert (E0277 pointing at the field's type), and a
//! `Vec<T>` field whose `T` lacks the marker does not dispatch at all
//! (E0277 pointing at the derive) — so *omitting* `#[ord(with =
//! vec_prefix)]` on a positional list is a compile error, not a silently
//! wrong order.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use collomatique_state::partial_order::{option_lift_discrete, prefix_pointwise, vec_prefix};
// One `use` brings in both namespaces, so `ContentOrd` here is at once the
// derive macro and the trait whose methods the assertions call.
use collomatique_state::{ContentIdentity, ContentOrd, OrderedTable, Table};

// ---------------------------------------------------------------------------
// Default dispatch through the container blankets
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct Containers {
    tag: Option<u64>,
    members: BTreeSet<u64>,
    rows: BTreeMap<u64, u64>,
    order: Vec<u64>,
    table: Table<u64, BTreeSet<u64>>,
    ordered: OrderedTable<u64, u64>,
}

fn containers() -> Containers {
    Containers {
        tag: Some(7),
        members: [1, 2, 3].into_iter().collect(),
        rows: [(1, 10), (2, 20)].into_iter().collect(),
        order: vec![1, 2, 3],
        table: [(1, [1, 2].into_iter().collect()), (2, BTreeSet::new())]
            .into_iter()
            .collect(),
        ordered: OrderedTable::try_from(vec![(1, 10), (2, 20), (3, 30)]).unwrap(),
    }
}

#[test]
fn default_dispatch_reaches_every_container_blanket() {
    let full = containers();

    assert_eq!(
        full.content_cmp(&full.clone()),
        Some(Ordering::Equal),
        "reflexivity through the whole product"
    );

    let mut option_cleared = containers();
    option_cleared.tag = None;
    assert_eq!(option_cleared.content_cmp(&full), Some(Ordering::Less));

    let mut set_shrunk = containers();
    set_shrunk.members.remove(&2);
    assert_eq!(set_shrunk.content_cmp(&full), Some(Ordering::Less));

    let mut map_row_removed = containers();
    map_row_removed.rows.remove(&2);
    assert_eq!(map_row_removed.content_cmp(&full), Some(Ordering::Less));

    let mut map_value_renumbered = containers();
    map_value_renumbered.rows.insert(2, 21);
    assert_eq!(map_value_renumbered.content_cmp(&full), None);

    let mut vec_shortened = containers();
    vec_shortened.order = vec![1, 3];
    assert_eq!(
        vec_shortened.content_cmp(&full),
        Some(Ordering::Less),
        "the Vec blanket is subsequence, so a middle deletion is below"
    );

    let mut table_row_removed = containers();
    table_row_removed.table.remove(&2);
    assert_eq!(table_row_removed.content_cmp(&full), Some(Ordering::Less));

    let mut ordered_reordered = containers();
    ordered_reordered.ordered = OrderedTable::try_from(vec![(3, 30), (1, 10), (2, 20)]).unwrap();
    assert_eq!(
        ordered_reordered.content_cmp(&full),
        None,
        "ordering is user-visible data: a reorder is incomparable"
    );
}

// ---------------------------------------------------------------------------
// `#[ord(atom)]`: a field whose type has no impls at all
// ---------------------------------------------------------------------------

/// Stands in for a foreign scalar leaf the orphan rule keeps out of the
/// trait (`SlotStart`, `NonZeroMinutes`): no `ContentOrd`, only `Eq`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ForeignScalar(u32);

#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct WithAtom {
    #[ord(atom)]
    leaf: ForeignScalar,
}

#[test]
fn atom_attribute_compares_discretely() {
    let a = WithAtom {
        leaf: ForeignScalar(1),
    };
    let b = WithAtom {
        leaf: ForeignScalar(1),
    };
    let c = WithAtom {
        leaf: ForeignScalar(2),
    };

    assert_eq!(a.content_cmp(&b), Some(Ordering::Equal));
    assert_eq!(a.content_cmp(&c), None, "different atoms are incomparable");
    assert_eq!(c.content_cmp(&a), None);
}

// ---------------------------------------------------------------------------
// `#[ord(ignore)]`: the equivalence-class pin
// ---------------------------------------------------------------------------

/// Stands in for the `Data` id issuer: not content, and with no impls.
#[derive(Clone, Debug, PartialEq, Eq)]
struct NotContent(u64);

#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct WithIgnore {
    #[ord(ignore)]
    bookkeeping: NotContent,
    members: BTreeSet<u64>,
}

#[test]
fn ignored_fields_create_equivalence_classes() {
    let a = WithIgnore {
        bookkeeping: NotContent(1),
        members: [1, 2].into_iter().collect(),
    };
    let b = WithIgnore {
        bookkeeping: NotContent(999),
        members: [1, 2].into_iter().collect(),
    };

    assert_ne!(a, b, "the two values really do differ");
    assert_eq!(
        a.content_cmp(&b),
        Some(Ordering::Equal),
        "the order does not see the ignored field"
    );
    assert!(a.content_eq(&b));
    assert!(
        !a.content_lt(&b),
        "content-equivalent values are not strictly below one another"
    );
}

// ---------------------------------------------------------------------------
// `#[ord(with = ...)]`: path form and closure form
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct WithCustom {
    /// Path form — the shape `PersonWithContact::tel` uses.
    #[ord(with = option_lift_discrete)]
    annotation: Option<String>,
    /// Closure form — the exact shape `GroupListParameters::group_names`
    /// uses: positional identity over a foreign element type.
    #[ord(with = |a, b| prefix_pointwise(a, b, option_lift_discrete))]
    names: Vec<Option<String>>,
}

fn with_custom(annotation: Option<&str>, names: &[Option<&str>]) -> WithCustom {
    WithCustom {
        annotation: annotation.map(String::from),
        names: names
            .iter()
            .map(|n| n.map(String::from))
            .collect::<Vec<_>>(),
    }
}

#[test]
fn with_attribute_path_form() {
    let full = with_custom(Some("note"), &[]);
    let cleared = with_custom(None, &[]);
    let renamed = with_custom(Some("other"), &[]);

    assert_eq!(cleared.content_cmp(&full), Some(Ordering::Less));
    assert_eq!(full.content_cmp(&cleared), Some(Ordering::Greater));
    assert_eq!(
        renamed.content_cmp(&full),
        None,
        "a rename is not a removal"
    );
}

#[test]
fn with_attribute_closure_form() {
    let full = with_custom(None, &[Some("a"), Some("b")]);
    let unnamed = with_custom(None, &[Some("a"), None]);
    let truncated = with_custom(None, &[Some("a")]);
    let middle_removed = with_custom(None, &[Some("b")]);

    assert_eq!(
        unnamed.content_cmp(&full),
        Some(Ordering::Less),
        "un-naming a group is removing content"
    );
    assert_eq!(
        truncated.content_cmp(&full),
        Some(Ordering::Less),
        "dropping the last group is removing content"
    );
    assert_eq!(
        middle_removed.content_cmp(&full),
        None,
        "removing a *first* group shifts every later index: incomparable"
    );
}

// ---------------------------------------------------------------------------
// `#[ord(total)]`: the one rule under which two different values compare
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct WithTotal {
    #[ord(total)]
    level: u32,
}

#[test]
fn total_attribute_uses_the_native_order() {
    let low = WithTotal { level: 3 };
    let high = WithTotal { level: 5 };

    assert_eq!(low.content_cmp(&high), Some(Ordering::Less));
    assert_eq!(high.content_cmp(&low), Some(Ordering::Greater));
    assert_eq!(low.content_cmp(&low.clone()), Some(Ordering::Equal));
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
enum Filling {
    Prefilled {
        #[ord(with = vec_prefix)]
        groups: Vec<BTreeSet<u64>>,
    },
    Automatic {
        excluded: BTreeSet<u64>,
    },
    Empty,
}

#[test]
fn enum_same_variant_compares_as_a_product() {
    let full = Filling::Prefilled {
        groups: vec![[1, 2].into_iter().collect(), [3].into_iter().collect()],
    };
    let shrunk = Filling::Prefilled {
        groups: vec![[1].into_iter().collect(), [3].into_iter().collect()],
    };
    assert_eq!(shrunk.content_cmp(&full), Some(Ordering::Less));

    let auto_full = Filling::Automatic {
        excluded: [1, 2].into_iter().collect(),
    };
    let auto_shrunk = Filling::Automatic {
        excluded: [1].into_iter().collect(),
    };
    assert_eq!(auto_shrunk.content_cmp(&auto_full), Some(Ordering::Less));
}

#[test]
fn enum_unit_variant_is_the_empty_product() {
    assert_eq!(
        Filling::Empty.content_cmp(&Filling::Empty),
        Some(Ordering::Equal)
    );
}

#[test]
fn enum_different_variants_are_incomparable() {
    let prefilled = Filling::Prefilled { groups: Vec::new() };
    let automatic = Filling::Automatic {
        excluded: BTreeSet::new(),
    };

    assert_eq!(prefilled.content_cmp(&automatic), None);
    assert_eq!(automatic.content_cmp(&prefilled), None);
    assert_eq!(
        prefilled.content_cmp(&Filling::Empty),
        None,
        "even against the emptiest variant: a variant switch is not a removal"
    );
}

// ---------------------------------------------------------------------------
// The product rule, and the degenerate shapes
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct TwoFields {
    left: BTreeSet<u64>,
    right: BTreeSet<u64>,
}

#[test]
fn product_mixes_field_directions() {
    let base = TwoFields {
        left: [1, 2].into_iter().collect(),
        right: [1, 2].into_iter().collect(),
    };
    let one_down = TwoFields {
        left: [1].into_iter().collect(),
        right: [1, 2].into_iter().collect(),
    };
    let one_down_one_up = TwoFields {
        left: [1].into_iter().collect(),
        right: [1, 2, 3].into_iter().collect(),
    };

    assert_eq!(
        one_down.content_cmp(&base),
        Some(Ordering::Less),
        "one field down, one equal"
    );
    assert_eq!(
        one_down_one_up.content_cmp(&base),
        None,
        "one field down and one up: the product is incomparable"
    );
}

#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct Empty {}

#[test]
fn an_empty_struct_is_its_own_equivalence_class() {
    assert_eq!(Empty {}.content_cmp(&Empty {}), Some(Ordering::Equal));
}

// ---------------------------------------------------------------------------
// The default methods
// ---------------------------------------------------------------------------

#[test]
fn default_methods_on_a_derived_type() {
    let base = TwoFields {
        left: [1, 2].into_iter().collect(),
        right: BTreeSet::new(),
    };
    let smaller = TwoFields {
        left: [1].into_iter().collect(),
        right: BTreeSet::new(),
    };

    assert!(smaller.content_lt(&base));
    assert!(smaller.content_le(&base));
    assert!(!smaller.content_eq(&base));

    assert!(base.content_eq(&base.clone()));
    assert!(base.content_le(&base.clone()));
    assert!(!base.content_lt(&base.clone()));

    assert!(!base.content_le(&smaller));
}

// ---------------------------------------------------------------------------
// `#[derive(ContentIdentity)]`
// ---------------------------------------------------------------------------

/// Every field rule the identity derive accepts: default (through an
/// enrolled type), atom (an `Eq` foreign leaf) and total (free by `Ord`'s
/// own contract). `PartialEq` is *derived* right here — that co-location is
/// the audit trail for the one premise the macro cannot check.
#[derive(Clone, Debug, PartialEq, Eq, ContentOrd, ContentIdentity)]
struct Marked {
    tag: u64,
    #[ord(atom)]
    leaf: ForeignScalar,
    #[ord(total)]
    level: u32,
}

/// The marker's whole purpose: `Marked` may now sit at a container matching
/// position. This struct compiling *is* the test — without the marker, the
/// `BTreeSet` blanket does not apply and the field fails to dispatch.
#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct HoldsMarked {
    marked: BTreeSet<Marked>,
}

impl PartialOrd for Marked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Marked {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.tag, self.leaf.0, self.level).cmp(&(other.tag, other.leaf.0, other.level))
    }
}

fn marked(tag: u64) -> Marked {
    Marked {
        tag,
        leaf: ForeignScalar(0),
        level: 0,
    }
}

#[test]
fn the_identity_marker_lets_a_derived_type_sit_in_a_container() {
    let full = HoldsMarked {
        marked: [marked(1), marked(2)].into_iter().collect(),
    };
    let shrunk = HoldsMarked {
        marked: [marked(1)].into_iter().collect(),
    };

    assert_eq!(
        shrunk.content_cmp(&full),
        Some(Ordering::Less),
        "the set of marked rows is compared by inclusion"
    );
    assert_eq!(full.content_cmp(&full.clone()), Some(Ordering::Equal));
}
