//! Output side of the translation layer: from a solved base-variable
//! configuration back to prefilled `GroupList` values — and the way back in,
//! from prefilled lists to a complete configuration a solve can start from
//! ([`group_lists_to_warm_start`]).

use crate::pairs::{PairData, cross_tiers};
use crate::specs::GenerationPlan;
use crate::types::ExtraVarName;
use crate::vars::{GroupListIdx, Var, VarEnv};
use collomatique_ilp::ConfigData;
use collomatique_ilp_modeler::InternalVar;
use collomatique_state_colloscopes::group_lists::{
    GroupList, GroupListFilling, GroupListParameters, PrefilledGroup,
};
use collomatique_state_colloscopes::{PeriodId, StudentId, SubjectId};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod tests;

/// Convert a solved configuration (base variables only, after
/// `filter_transmute`) into one prefilled `GroupList` per spec, paired with
/// the (period, subject) pairs it must be associated to.
///
/// Panics on internal inconsistency: a missing `StudentInGroup` value, a
/// student sitting in no group at all, or a name-count mismatch are all
/// caller or solver bugs — every student of every spec must have a solved
/// value for each of its groups, exactly one of them being 1.
pub fn build_group_lists(
    plan: &GenerationPlan,
    names: &[String],
    config: &ConfigData<Var>,
) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)> {
    assert_eq!(
        names.len(),
        plan.specs.len(),
        "one name per spec is required"
    );

    let env = VarEnv::new(plan);

    plan.specs
        .iter()
        .enumerate()
        .map(|(i, (spec, covered))| {
            let list = GroupListIdx(i);
            let group_count = env.group_count(list) as usize;

            let mut slots: Vec<BTreeSet<StudentId>> = vec![BTreeSet::new(); group_count];
            for &student in spec.students() {
                // The first group whose binary is 1. "Exactly one" is a
                // constraint of the model, so a solved configuration has
                // exactly one — reading the first is only how the index is
                // recovered from the matrix.
                let slot = (0..group_count)
                    .find(|&group| {
                        let value = config
                            .get(Var::StudentInGroup {
                                list,
                                student,
                                group: group as u32,
                            })
                            .expect("every (student, group) pair must have a solved value");
                        value.round() as i64 == 1
                    })
                    .expect("every student must sit in a group");
                slots[slot].insert(student);
            }

            // Compact away empty slots and remap group indices. The
            // minimum-size constraint (piece 8) makes every group non-empty
            // in a solved configuration, but the conversion is also handed
            // arbitrary placements (the fuzz-build test), so it never
            // assumes it.
            let groups: Vec<PrefilledGroup> = slots
                .into_iter()
                .filter(|students| !students.is_empty())
                .map(|students| PrefilledGroup { students })
                .collect();

            let group_list_params = GroupListParameters {
                name: names[i].clone(),
                students_per_group: spec.students_per_group().clone(),
                group_names: vec![None; groups.len()],
            };
            let group_list =
                GroupList::new(group_list_params, GroupListFilling::Prefilled { groups })
                    .expect("generated lists satisfy the prefilled invariants by construction");

            (group_list, covered.clone())
        })
        .collect()
}

/// The way back in: one *complete* configuration — base variables and extras
/// — from a set of prefilled lists the model did not produce, so that a solve
/// can be handed it as a warm start.
///
/// `lists` is a [`build_group_lists`] output for the same plan, in plan order:
/// in practice a [`greedy_group_lists`](crate::greedy_group_lists) result,
/// whose group order *is* the model's group index — both are the closed
/// `⌈n / max⌉` groups of [`VarEnv::group_count`].
///
/// The extras are given their **tight** values, read off the same placement:
/// a `Together` is 1 exactly when the pair sits in that group of that list, a
/// `Coincide` exactly when the pair meets in both of its tiers. Both families
/// are one-sided and only ever pulled *down* (`crate::extras`), so the tight
/// values are feasible, and they are the ones the maximizing solve settles on
/// — the configuration therefore evaluates to exactly the collision score of
/// this placement, which is to say to
/// [`placement_objective`](crate::placement_objective) of the same lists, and
/// not to some inflated bound.
///
/// The variable set is the model's own: both families are enumerated from
/// [`PairData`], the very table that declares them, so valuation and
/// declaration cannot drift apart. A configuration missing a variable — or
/// carrying one the model does not have — is refused wholesale by
/// `Model::solution_from_complete_data`.
///
/// Panics on internal inconsistency, like [`build_group_lists`]: a list whose
/// count differs from the plan's, a student of a spec sitting in no group of
/// its list, or a group index beyond the list's closed group count.
pub fn group_lists_to_warm_start(
    plan: &GenerationPlan,
    lists: &[(GroupList, BTreeSet<(PeriodId, SubjectId)>)],
) -> ConfigData<InternalVar<Var, ExtraVarName>> {
    assert_eq!(
        lists.len(),
        plan.specs.len(),
        "one list per spec is required"
    );

    let env = VarEnv::new(plan);

    // Where everyone sits, read once: every value below — base binary, shared
    // pair, reference-group piece — is a question about this one placement.
    let placement: Vec<BTreeMap<StudentId, u32>> = env
        .lists()
        .map(|list| {
            let filling = lists[list.0].0.filling();
            let group_count = env.group_count(list);
            env.students(list)
                .iter()
                .map(|&student| {
                    let group = filling
                        .find_student_group(student)
                        .expect("every student of a spec sits in a group of its list");
                    let group = u32::try_from(group).expect("a group index fits in a u32");
                    assert!(
                        group < group_count,
                        "the list has more groups than the model gives it",
                    );
                    (student, group)
                })
                .collect()
        })
        .collect();

    let mut config = ConfigData::new();

    for list in env.lists() {
        for (&student, &slot) in &placement[list.0] {
            for group in 0..env.group_count(list) {
                config = config.set(
                    InternalVar::Base(Var::StudentInGroup {
                        list,
                        student,
                        group,
                    }),
                    if group == slot { 1.0 } else { 0.0 },
                );
            }
        }
    }

    let pairs = PairData::new(plan, &env);
    for ((a, b), table) in pairs.pairs() {
        // The group the two share in a list, if any. Both students belong to
        // every list of their tier table, so the two lookups are `Some`
        // whenever the pair is asked about at all.
        let shared_seat = |list: GroupListIdx| -> Option<u32> {
            let seats = &placement[list.0];
            match (seats.get(&a), seats.get(&b)) {
                (Some(&ga), Some(&gb)) if ga == gb => Some(ga),
                _ => None,
            }
        };
        // Whether they meet in one *tier* — the sum a `Coincide` is bounded
        // by, which the one group per list makes 0/1.
        let in_tier = |tier: &crate::pairs::Tier| {
            shared_seat(tier.list).is_some_and(|group| tier.groups.contains(&group))
        };

        for tier in table {
            for &group in &tier.groups {
                config = config.set(
                    InternalVar::Extra(ExtraVarName::Together {
                        a,
                        b,
                        list: tier.list,
                        group,
                    }),
                    if shared_seat(tier.list) == Some(group) {
                        1.0
                    } else {
                        0.0
                    },
                );
            }
        }

        for (first, second) in cross_tiers(table) {
            config = config.set(
                InternalVar::Extra(ExtraVarName::Coincide {
                    a,
                    b,
                    list1: first.list,
                    target1: first.target,
                    list2: second.list,
                    target2: second.target,
                }),
                if in_tier(first) && in_tier(second) {
                    1.0
                } else {
                    0.0
                },
            );
        }
    }

    config
}
