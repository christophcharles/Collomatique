//! Integration tests for the `EntityId` and `References` derive macros
//!
//! These are integration tests (not unit tests) on purpose: the derives emit
//! absolute `::collomatique_state::…` paths, so the generated code only
//! compiles from outside the crate.

use collomatique_state::{EntityId, Id, References};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, EntityId)]
struct AlphaId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, EntityId)]
struct BetaId(u64);

/// Toy equivalent of `state-colloscopes`' `NewId` union type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToyUnion {
    Alpha(AlphaId),
    Beta(BetaId),
}

impl From<AlphaId> for ToyUnion {
    fn from(value: AlphaId) -> Self {
        ToyUnion::Alpha(value)
    }
}

impl From<BetaId> for ToyUnion {
    fn from(value: BetaId) -> Self {
        ToyUnion::Beta(value)
    }
}

fn alpha(value: u64) -> AlphaId {
    unsafe { AlphaId::new(value) }
}

fn beta(value: u64) -> BetaId {
    unsafe { BetaId::new(value) }
}

fn collect<T: References<ToyUnion>>(value: &T) -> Vec<ToyUnion> {
    let mut out = Vec::new();
    value.for_each_ref(&mut |id| out.push(id));
    out
}

#[test]
fn entity_id_implements_id() {
    let id = alpha(42);
    assert_eq!(id.inner(), 42);
}

#[test]
fn leaf_id_reports_itself() {
    assert_eq!(collect(&alpha(1)), vec![ToyUnion::Alpha(alpha(1))]);
    assert_eq!(collect(&beta(2)), vec![ToyUnion::Beta(beta(2))]);
}

#[test]
fn option_lift_walks_some_and_skips_none() {
    assert_eq!(collect(&Some(beta(7))), vec![ToyUnion::Beta(beta(7))]);
    assert_eq!(collect(&None::<BetaId>), vec![]);
}

#[test]
fn vec_lift_preserves_order() {
    let ids = vec![alpha(3), alpha(1), alpha(2)];
    assert_eq!(
        collect(&ids),
        vec![
            ToyUnion::Alpha(alpha(3)),
            ToyUnion::Alpha(alpha(1)),
            ToyUnion::Alpha(alpha(2)),
        ]
    );
}

#[test]
fn btree_set_lift_walks_in_id_order() {
    let ids = BTreeSet::from([alpha(3), alpha(1), alpha(2)]);
    assert_eq!(
        collect(&ids),
        vec![
            ToyUnion::Alpha(alpha(1)),
            ToyUnion::Alpha(alpha(2)),
            ToyUnion::Alpha(alpha(3)),
        ]
    );
}

#[derive(References)]
struct ToyEntity {
    #[fk]
    alpha: AlphaId,
    _name: String,
    #[fk]
    maybe_beta: Option<BetaId>,
    #[fk]
    others: Vec<AlphaId>,
}

#[test]
fn derived_struct_walks_fk_fields_in_declaration_order() {
    let entity = ToyEntity {
        alpha: alpha(1),
        _name: "toy".to_string(),
        maybe_beta: Some(beta(2)),
        others: vec![alpha(4), alpha(3)],
    };
    assert_eq!(
        collect(&entity),
        vec![
            ToyUnion::Alpha(alpha(1)),
            ToyUnion::Beta(beta(2)),
            ToyUnion::Alpha(alpha(4)),
            ToyUnion::Alpha(alpha(3)),
        ]
    );
}

#[derive(References)]
struct ToyInner {
    #[fk]
    beta: BetaId,
}

#[derive(References)]
struct ToyOuter {
    #[fk]
    first: AlphaId,
    #[fk]
    inner: ToyInner,
    #[fk(name = renamed)]
    last: BetaId,
}

#[test]
fn nested_structs_compose_and_fk_name_argument_is_accepted() {
    let outer = ToyOuter {
        first: alpha(10),
        inner: ToyInner { beta: beta(20) },
        last: beta(30),
    };
    assert_eq!(
        collect(&outer),
        vec![
            ToyUnion::Alpha(alpha(10)),
            ToyUnion::Beta(beta(20)),
            ToyUnion::Beta(beta(30)),
        ]
    );
}
