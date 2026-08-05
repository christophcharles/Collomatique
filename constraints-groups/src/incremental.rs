//! Inclusion-based incremental epochs (pieces 10 and 12, §2.6 and §5 of
//! the roadmap).
//!
//! The incremental strategy solves the model in stages. The stages are
//! ordered by strict inclusion of the specs' student sets: the
//! inclusion-minimal lists are solved first, and every larger list that
//! contains them is solved later, aligning its groups with the
//! already-fixed small lists through the pair objective. That gives every
//! spec an inclusion *level*.
//!
//! A level, though, still holds lists that have nothing to do with each
//! other (the German LV2 list and the Spanish LV2 list, say). Branch and
//! bound does not decompose a model into independent blocks by itself, so
//! piece 12 splits each level into its connected components — two specs of
//! a level are joined when their student sets intersect — and gives every
//! component an epoch of its own.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use collomatique_state_colloscopes::StudentId;

use crate::specs::GenerationPlan;
use crate::vars::{GroupListIdx, Var};

/// Assign each base variable an epoch for the incremental (staggered)
/// solve: every `StudentGroup` variable gets its spec's epoch. The epochs
/// are built in two passes.
///
/// The first pass gives each spec its inclusion *level*: the height of the
/// longest chain of strictly-included student sets below it. A spec with no
/// strict subset sits at level 0. Equal student sets never strictly include
/// each other, so they never relate; the size ranges play no role.
///
/// The second pass splits each level into the connected components of the
/// "share a student" relation, and numbers one epoch per component. The
/// levels keep their order — every component of a level is numbered before
/// any component of the next — so the inclusion ordering is untouched.
/// Connectivity is computed inside a level only: over the whole spec set
/// almost everything is one component as soon as a whole-class list exists.
/// Inside a level the smaller blocks (fewer distinct students) run first.
///
/// The resulting epoch numbers are contiguous from 0, and the map names
/// exactly the model's base variables.
pub fn build_incremental_epochs(plan: &GenerationPlan) -> HashMap<Var, u32> {
    // Pass 1 — inclusion levels. Process the specs by ascending student
    // count: a strict subset always has strictly fewer students, so every
    // strict subset of a spec is already computed when the spec's turn
    // comes. The plan's own order is lexicographic on the student sets, not
    // sorted by size, so this sort is what makes the single pass correct.
    let mut order: Vec<usize> = (0..plan.specs.len()).collect();
    order.sort_by_key(|&i| plan.specs[i].0.students.len());

    let mut heights = vec![0u32; plan.specs.len()];
    for (pos, &i) in order.iter().enumerate() {
        let students = &plan.specs[i].0.students;
        for &j in &order[..pos] {
            let candidate = &plan.specs[j].0.students;
            // The length test makes the inclusion strict: the processed
            // prefix may hold sets of the same size (equal sets included),
            // and those must never relate.
            if candidate.len() < students.len() && candidate.is_subset(students) {
                heights[i] = heights[i].max(heights[j] + 1);
            }
        }
    }

    // Pass 2 — split each level into its connected components. The
    // components of a level are numbered before those of the next level, so
    // the inclusion ordering of pass 1 is preserved exactly; inside a level
    // the smaller blocks come first. The tie-break on the smallest member
    // index is for determinism only: equal-sized blocks of one level share
    // no student, so their relative order is semantically indifferent.
    let mut levels: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, &h) in heights.iter().enumerate() {
        levels.entry(h).or_default().push(i);
    }

    let mut spec_epochs = vec![0u32; plan.specs.len()];
    let mut next_epoch = 0u32;
    for level_specs in levels.into_values() {
        // Grow the components in spec-index order. A spec that touches
        // several existing components merges them all: intersecting a
        // component's union set is the same as intersecting one of its
        // members' sets, since the union is exactly their union.
        let mut components: Vec<(BTreeSet<StudentId>, Vec<usize>)> = Vec::new();
        for &i in &level_specs {
            let students = &plan.specs[i].0.students;
            let (touching, mut rest): (Vec<_>, Vec<_>) = components
                .into_iter()
                .partition(|(union, _members)| !union.is_disjoint(students));
            let mut union = students.clone();
            let mut members = vec![i];
            for (u, m) in touching {
                union.extend(u);
                members.extend(m);
            }
            rest.push((union, members));
            components = rest;
        }

        components.sort_by_key(|(union, members)| {
            (
                union.len(),
                members
                    .iter()
                    .copied()
                    .min()
                    .expect("a component always holds the spec that created it"),
            )
        });
        for (_union, members) in components {
            for i in members {
                spec_epochs[i] = next_epoch;
            }
            next_epoch += 1;
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
    fn disjoint_sets_of_a_level_split_smaller_first() {
        // Both specs are inclusion-minimal (level 0), but they share no
        // student, so each is its own component and its own epoch. The
        // smaller block ({4,5}) solves first. Different sizes on purpose:
        // the is_subset → true mutation check needs the strictness test to
        // not mask it, and the size order needs a strict size difference.
        let plan = plan_of(&[(&[1, 2, 3], (1, 3)), (&[4, 5], (1, 2))]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 1), 0);
        assert_eq!(epoch_of(&epochs, &plan, 0), 1);
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
        // subset test. They are both at level 0, and they share student 2,
        // so they form one component: sharing a height is not enough, they
        // share an epoch *because* they share a student.
        let plan = plan_of(&[(&[1, 2], (1, 2)), (&[2, 3, 4], (1, 3))]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 0);
    }

    #[test]
    fn equal_sets_never_relate() {
        // Same students, different ranges: two distinct specs (dedup only
        // merges identical (set, range) keys), and neither strictly
        // includes the other, so both sit at level 0. Equal sets intersect,
        // so they also form a single component: both stay in epoch 0.
        let plan = plan_of(&[(&[1, 2, 3], (1, 2)), (&[1, 2, 3], (2, 3))]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 0);
    }

    #[test]
    fn roadmap_example_heights() {
        // §2.6's worked example, shrunk: two disjoint LV2 lists, a
        // sciences list containing both, the whole class containing
        // everything. Levels 0, 0, 1, 2 — and the two LV2 lists are
        // disjoint, so level 0 splits into two components of equal size,
        // ordered by the deterministic tie-break. Epochs 0, 1, 2, 3.
        //
        // This also pins that connectivity is computed per level: the
        // whole-class list intersects both LV2 lists, but it sits at
        // another level and must not merge their components.
        let plan = plan_of(&[
            (&[1, 2], (1, 2)),
            (&[3, 4], (1, 2)),
            (&[1, 2, 3, 4, 5], (1, 5)),
            (&[1, 2, 3, 4, 5, 6, 7], (1, 7)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 1);
        assert_eq!(epoch_of(&epochs, &plan, 2), 2);
        assert_eq!(epoch_of(&epochs, &plan, 3), 3);
    }

    #[test]
    fn height_is_a_max_over_all_strict_subsets() {
        // {1} ⊂ {1,2} is a chain of height 1. {3,4,5} is inclusion-minimal
        // but *larger* than {1,2}, so the ascending-size pass visits it
        // last among the top spec's subsets. The top spec must take the
        // max over all of them (level 2), not follow the last one visited
        // (which would give 1).
        //
        // Levels: {1} and {3,4,5} at 0, {1,2} at 1, the top at 2. Level 0
        // holds two disjoint components, ordered by size ({1} first), so
        // the epochs run 0, 1, 2, 3. Under the last-wins mutation the top
        // spec's level degrades to 1, it joins {1,2}'s level, and these
        // assertions fail.
        let plan = plan_of(&[
            (&[1], (1, 1)),
            (&[1, 2], (1, 2)),
            (&[3, 4, 5], (1, 3)),
            (&[1, 2, 3, 4, 5], (1, 5)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 2), 1);
        assert_eq!(epoch_of(&epochs, &plan, 1), 2);
        assert_eq!(epoch_of(&epochs, &plan, 3), 3);
    }

    #[test]
    fn smaller_blocks_solve_first_within_a_level() {
        // Three disjoint level-0 blocks of pairwise distinct sizes, given
        // in scrambled order: the epochs follow the sizes, not the plan
        // order.
        let plan = plan_of(&[
            (&[1, 2, 3, 4], (1, 4)),
            (&[5, 6], (1, 2)),
            (&[7, 8, 9], (1, 3)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 1), 0); // 2 students
        assert_eq!(epoch_of(&epochs, &plan, 2), 1); // 3 students
        assert_eq!(epoch_of(&epochs, &plan, 0), 2); // 4 students
    }

    #[test]
    fn connectivity_is_transitive_within_a_level() {
        // {1,2} and {3,4} are disjoint, but {2,3} touches both, so the
        // three specs form ONE component — and {2,3} arrives last in index
        // order, so it must merge two already-separate components, not
        // just join the first one it touches. The singleton {9} stays its
        // own block; it is smaller (1 student against the merged block's
        // 4), so it solves first.
        let plan = plan_of(&[
            (&[1, 2], (1, 2)),
            (&[3, 4], (1, 2)),
            (&[2, 3], (1, 2)),
            (&[9], (1, 1)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 3), 0);
        assert_eq!(epoch_of(&epochs, &plan, 0), 1);
        assert_eq!(epoch_of(&epochs, &plan, 1), 1);
        assert_eq!(epoch_of(&epochs, &plan, 2), 1);
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
