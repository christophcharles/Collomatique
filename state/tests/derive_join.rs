//! Integration tests for the join machinery (`Joinable`/`Join`/`Lookup`,
//! the `Join` derive macro and the `#[entity(Type)]` attribute of `EntityId`)
//!
//! Same rationale as `derive_refs.rs`: the derives emit absolute
//! `::collomatique_state::…` paths, so the generated code only compiles
//! from outside the crate.

use collomatique_state::{EntityId, Id, Join, Lookup};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Alpha {
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Beta {
    value: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, EntityId)]
#[entity(Alpha)]
struct AlphaId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, EntityId)]
#[entity(Beta)]
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

struct ToyContext {
    alphas: BTreeMap<AlphaId, Alpha>,
    betas: BTreeMap<BetaId, Beta>,
}

impl Lookup<AlphaId> for ToyContext {
    type Entity = Alpha;

    fn lookup(&self, id: AlphaId) -> Option<&Alpha> {
        self.alphas.get(&id)
    }
}

impl Lookup<BetaId> for ToyContext {
    type Entity = Beta;

    fn lookup(&self, id: BetaId) -> Option<&Beta> {
        self.betas.get(&id)
    }
}

fn alpha(value: u64) -> AlphaId {
    unsafe { AlphaId::new(value) }
}

fn beta(value: u64) -> BetaId {
    unsafe { BetaId::new(value) }
}

fn toy_context() -> ToyContext {
    ToyContext {
        alphas: BTreeMap::from([
            (
                alpha(1),
                Alpha {
                    name: "one".to_string(),
                },
            ),
            (
                alpha(2),
                Alpha {
                    name: "two".to_string(),
                },
            ),
            (
                alpha(3),
                Alpha {
                    name: "three".to_string(),
                },
            ),
        ]),
        betas: BTreeMap::from([
            (beta(10), Beta { value: 10 }),
            (beta(20), Beta { value: 20 }),
        ]),
    }
}

#[test]
fn leaf_join_borrows_from_the_context() {
    let ctx = toy_context();
    let id = alpha(1);
    let joined = id.join(&ctx).unwrap();
    assert!(std::ptr::eq(joined, ctx.alphas.get(&alpha(1)).unwrap()));
}

#[test]
fn dangling_leaf_reports_the_id() {
    let ctx = toy_context();
    let id = alpha(99);
    assert_eq!(id.join(&ctx), Err(alpha(99)));
}

#[derive(Join)]
#[join(error = ToyUnion)]
struct ToyEntity {
    #[fk]
    alpha: AlphaId,
    name: String,
    #[fk]
    maybe_beta: Option<BetaId>,
    #[fk]
    others: Vec<AlphaId>,
    #[fk]
    beta_set: BTreeSet<BetaId>,
}

fn toy_entity() -> ToyEntity {
    ToyEntity {
        alpha: alpha(1),
        name: "toy".to_string(),
        maybe_beta: Some(beta(20)),
        others: vec![alpha(3), alpha(2)],
        beta_set: BTreeSet::from([beta(20), beta(10)]),
    }
}

#[test]
fn derived_struct_joins_every_fk_field_and_borrows_the_rest() {
    let ctx = toy_context();
    let entity = toy_entity();
    let joined: JoinedToyEntity = entity.join(&ctx).unwrap();

    assert!(std::ptr::eq(
        joined.alpha,
        ctx.alphas.get(&alpha(1)).unwrap()
    ));
    assert!(std::ptr::eq(joined.name, &entity.name));
    assert_eq!(joined.maybe_beta, Some(&Beta { value: 20 }));
}

#[test]
fn option_none_passes_through() {
    let ctx = toy_context();
    let entity = ToyEntity {
        maybe_beta: None,
        ..toy_entity()
    };
    assert_eq!(entity.join(&ctx).unwrap().maybe_beta, None);
}

#[test]
fn vec_join_preserves_order() {
    let ctx = toy_context();
    let entity = toy_entity();
    let joined = entity.join(&ctx).unwrap();
    let names: Vec<_> = joined.others.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["three", "two"]);
}

#[test]
fn btree_set_joins_to_id_sorted_vec() {
    let ctx = toy_context();
    let entity = toy_entity();
    let joined = entity.join(&ctx).unwrap();
    let values: Vec<_> = joined.beta_set.iter().map(|b| b.value).collect();
    assert_eq!(values, vec![10, 20]);
}

#[test]
fn dangling_fk_is_converted_into_the_union_error() {
    let ctx = toy_context();
    let entity = ToyEntity {
        others: vec![alpha(3), alpha(99)],
        ..toy_entity()
    };
    assert!(matches!(
        entity.join(&ctx),
        Err(ToyUnion::Alpha(id)) if id == alpha(99)
    ));

    let entity = ToyEntity {
        maybe_beta: Some(beta(99)),
        ..toy_entity()
    };
    assert!(matches!(
        entity.join(&ctx),
        Err(ToyUnion::Beta(id)) if id == beta(99)
    ));
}

#[derive(Join)]
#[join(error = ToyUnion, output = InnerView)]
struct ToyInner {
    #[fk]
    beta: BetaId,
}

#[derive(Join)]
#[join(error = ToyUnion)]
struct ToyOuter {
    #[fk]
    inner: ToyInner,
    #[fk(name = renamed)]
    last: AlphaId,
}

#[test]
fn nested_structs_compose_and_fk_name_renames_the_joined_field() {
    let ctx = toy_context();
    let outer = ToyOuter {
        inner: ToyInner { beta: beta(10) },
        last: alpha(2),
    };
    let joined: JoinedToyOuter = outer.join(&ctx).unwrap();
    let inner: &InnerView = &joined.inner;
    assert_eq!(inner.beta, &Beta { value: 10 });
    assert_eq!(joined.renamed.name, "two");
}

#[test]
fn nested_error_converts_through_both_levels() {
    let ctx = toy_context();
    let outer = ToyOuter {
        inner: ToyInner { beta: beta(99) },
        last: alpha(2),
    };
    assert!(matches!(
        outer.join(&ctx),
        Err(ToyUnion::Beta(id)) if id == beta(99)
    ));
}
