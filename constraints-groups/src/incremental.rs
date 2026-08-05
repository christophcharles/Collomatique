//! Inclusion-based incremental epochs (piece 10, §2.6 of the roadmap).
//!
//! The incremental strategy solves the model in stages. The stages are
//! ordered by strict inclusion of the specs' student sets: the
//! inclusion-minimal lists are solved first, and every larger list that
//! contains them is solved later, aligning its groups with the
//! already-fixed small lists through the pair objective.

use std::collections::HashMap;

use crate::specs::GenerationPlan;
use crate::vars::{GroupListIdx, Var};

/// Assign each base variable an epoch for the incremental (staggered)
/// solve: every `StudentGroup` variable gets its spec's epoch, which is the
/// height of the longest chain of strictly-included student sets below the
/// spec. A spec with no strict subset gets epoch 0. Equal student sets
/// never strictly include each other, so they never relate; the size
/// ranges play no role. The map names exactly the model's base variables.
pub fn build_incremental_epochs(plan: &GenerationPlan) -> HashMap<Var, u32> {
    // Process the specs by ascending student count: a strict subset always
    // has strictly fewer students, so every strict subset of a spec is
    // already computed when the spec's turn comes. The plan's own order is
    // lexicographic on the student sets, not sorted by size, so this sort
    // is what makes the single pass correct.
    let mut order: Vec<usize> = (0..plan.specs.len()).collect();
    order.sort_by_key(|&i| plan.specs[i].0.students.len());

    let mut spec_epochs = vec![0u32; plan.specs.len()];
    for (pos, &i) in order.iter().enumerate() {
        let students = &plan.specs[i].0.students;
        for &j in &order[..pos] {
            let candidate = &plan.specs[j].0.students;
            // The length test makes the inclusion strict: the processed
            // prefix may hold sets of the same size (equal sets included),
            // and those must never relate.
            if candidate.len() < students.len() && candidate.is_subset(students) {
                spec_epochs[i] = spec_epochs[i].max(spec_epochs[j] + 1);
            }
        }
    }

    let mut epochs = HashMap::new();
    for (i, (spec, _covered)) in plan.specs.iter().enumerate() {
        for &student in &spec.students {
            epochs.insert(
                Var::StudentGroup {
                    list: GroupListIdx(i),
                    student,
                },
                spec_epochs[i],
            );
        }
    }
    epochs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vars::VarEnv;
    use crate::vars::tests::plan_of;
    use collomatique_ilp_modeler::DescribeVar;
    use std::collections::BTreeSet;

    /// The epoch shared by every variable of a list, asserting on the way
    /// that the list's variables all agree on it.
    fn epoch_of(epochs: &HashMap<Var, u32>, plan: &GenerationPlan, list: usize) -> u32 {
        let values: BTreeSet<u32> = plan.specs[list]
            .0
            .students
            .iter()
            .map(|&student| {
                epochs[&Var::StudentGroup {
                    list: GroupListIdx(list),
                    student,
                }]
            })
            .collect();
        assert_eq!(values.len(), 1, "all variables of a spec share its epoch");
        values.into_iter().next().unwrap()
    }

    #[test]
    fn disjoint_sets_are_epoch_zero() {
        // Different sizes on purpose: the is_subset → true mutation check
        // needs the strictness test to not mask it.
        let plan = plan_of(&[(&[1, 2, 3], (1, 3)), (&[4, 5], (1, 2))]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 0);
    }

    #[test]
    fn nested_chain_counts_height() {
        // Deliberately superset-first: the result must not depend on plan
        // order, only on the ascending-size processing order.
        let plan = plan_of(&[
            (&[1, 2, 3, 4, 5, 6], (1, 6)),
            (&[1, 2, 3, 4], (1, 4)),
            (&[1, 2], (1, 2)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 2);
        assert_eq!(epoch_of(&epochs, &plan, 1), 1);
        assert_eq!(epoch_of(&epochs, &plan, 2), 0);
    }

    #[test]
    fn overlapping_incomparable_sets_share_their_epoch() {
        // {1,2} and {2,3,4} overlap but neither contains the other; the
        // sizes differ so the strictness test alone cannot save a broken
        // subset test.
        let plan = plan_of(&[(&[1, 2], (1, 2)), (&[2, 3, 4], (1, 3))]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 0);
    }

    #[test]
    fn equal_sets_never_relate() {
        // Same students, different ranges: two distinct specs (dedup only
        // merges identical (set, range) keys), and neither strictly
        // includes the other, so both sit in epoch 0.
        let plan = plan_of(&[(&[1, 2, 3], (1, 2)), (&[1, 2, 3], (2, 3))]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 0);
    }

    #[test]
    fn roadmap_example_heights() {
        // §2.6's worked example, shrunk: two disjoint LV2 lists, a
        // sciences list containing both, the whole class containing
        // everything. Epochs 0, 0, 1, 2.
        let plan = plan_of(&[
            (&[1, 2], (1, 2)),
            (&[3, 4], (1, 2)),
            (&[1, 2, 3, 4, 5], (1, 5)),
            (&[1, 2, 3, 4, 5, 6, 7], (1, 7)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 0);
        assert_eq!(epoch_of(&epochs, &plan, 2), 1);
        assert_eq!(epoch_of(&epochs, &plan, 3), 2);
    }

    #[test]
    fn height_is_a_max_over_all_strict_subsets() {
        // {1} ⊂ {1,2} is a chain of height 1. {3,4,5} is inclusion-minimal
        // but *larger* than {1,2}, so the ascending-size pass visits it
        // last among the top spec's subsets. The top spec must take the
        // max over all of them (epoch 2), not follow the last one visited
        // (which would give 1).
        let plan = plan_of(&[
            (&[1], (1, 1)),
            (&[1, 2], (1, 2)),
            (&[3, 4, 5], (1, 3)),
            (&[1, 2, 3, 4, 5], (1, 5)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 1);
        assert_eq!(epoch_of(&epochs, &plan, 2), 0);
        assert_eq!(epoch_of(&epochs, &plan, 3), 2);
    }

    #[test]
    fn map_names_exactly_the_base_variables() {
        // The strategy contract: entries are base variables; a base
        // variable absent from the map lands in the final epoch, so
        // covering all of them is what makes the epochs authoritative.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 3)), (&[3, 4, 5], (1, 2))]);
        let epochs = build_incremental_epochs(&plan);
        let env = VarEnv::new(&plan);
        let enumerated: BTreeSet<Var> = <Var as DescribeVar>::enumerate(&env).into_keys().collect();
        let named: BTreeSet<Var> = epochs.into_keys().collect();
        assert_eq!(named, enumerated);
    }
}
