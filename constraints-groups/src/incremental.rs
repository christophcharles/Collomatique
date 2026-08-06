//! Inclusion-based incremental epochs (pieces 10 and 12bis, §2.6 and §5 of
//! the roadmap).
//!
//! The incremental strategy solves the model in stages. The stages are
//! ordered by strict inclusion of the specs' student sets: the
//! inclusion-minimal lists are solved first, and every larger list that
//! contains them is solved later, aligning its groups with the
//! already-fixed small lists through the pair objective. That gives every
//! spec an inclusion *level*.
//!
//! A level, though, still holds several lists, and branch and bound does
//! not decompose a model into independent blocks by itself. Piece 12 split
//! a level into the connected components of the "share a student" relation,
//! but real documents overlap too much: the components fuse into one big
//! block and the level solves as a single large model anyway. So piece 12bis
//! gives **every spec an epoch of its own**, and inside a level runs the
//! least-entangled lists first — those sharing the fewest students with the
//! other lists of the level.

use std::collections::{BTreeMap, HashMap};

use crate::specs::GenerationPlan;
use crate::vars::{GroupListIdx, Var, VarEnv};

/// Assign each base variable an epoch for the incremental (staggered)
/// solve: every `StudentInGroup` binary of a spec gets that spec's epoch.
/// The epochs are built in two passes.
///
/// The first pass gives each spec its inclusion *level*: the height of the
/// longest chain of strictly-included student sets below it. A spec with no
/// strict subset sits at level 0. Equal student sets never strictly include
/// each other, so they never relate; the size ranges play no role.
///
/// The second pass gives every spec an epoch of its own. The levels keep
/// their order — every spec of a level is numbered before any spec of the
/// next — so the inclusion ordering is untouched. Inside a level the specs
/// are ordered by the number of distinct students they share with the other
/// specs *of that level*, then by student count (small lists first), then by
/// spec index for determinism. Sharing is counted inside the level only:
/// against the whole plan, a single whole-class list would make every spec's
/// count equal its size and collapse the ordering into plain size ordering.
///
/// The resulting epoch numbers are contiguous from 0 (each level occupying a
/// contiguous run), and the map names exactly the model's base variables.
pub fn build_incremental_epochs(plan: &GenerationPlan) -> HashMap<Var, u32> {
    // Pass 1 — inclusion levels. Process the specs by ascending student
    // count: a strict subset always has strictly fewer students, so every
    // strict subset of a spec is already computed when the spec's turn
    // comes. The plan's own order is lexicographic on the student sets, not
    // sorted by size, so this sort is what makes the single pass correct.
    let mut order: Vec<usize> = (0..plan.specs.len()).collect();
    order.sort_by_key(|&i| plan.specs[i].0.students().len());

    let mut heights = vec![0u32; plan.specs.len()];
    for (pos, &i) in order.iter().enumerate() {
        let students = plan.specs[i].0.students();
        for &j in &order[..pos] {
            let candidate = plan.specs[j].0.students();
            // The length test makes the inclusion strict: the processed
            // prefix may hold sets of the same size (equal sets included),
            // and those must never relate.
            if candidate.len() < students.len() && candidate.is_subset(students) {
                heights[i] = heights[i].max(heights[j] + 1);
            }
        }
    }

    // Pass 2 — one epoch per spec (piece 12bis). Real documents showed too
    // much overlap for piece 12's connected components: a level fuses into
    // one big block and solves as a single large model anyway. So every
    // spec of a level gets an epoch of its own, and inside the level the
    // least-entangled specs run first: sorted by the number of distinct
    // students shared with the other specs of the level, then by student
    // count (small lists first), then by spec index for determinism. A
    // barely-shared list is nearly independent — its solo optimum is close
    // to the joint one — while a heavily-shared list solves later, when
    // the lists it is entangled with are already anchored and it can align
    // to them. Sharing is counted inside the level only: against the whole
    // plan, a whole-class list would make every spec's count equal its
    // size and collapse the ordering into plain size ordering.
    let mut levels: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, &h) in heights.iter().enumerate() {
        levels.entry(h).or_default().push(i);
    }

    let mut spec_epochs = vec![0u32; plan.specs.len()];
    let mut next_epoch = 0u32;
    for level_specs in levels.into_values() {
        let mut keyed: Vec<(usize, usize, usize)> = level_specs
            .iter()
            .map(|&i| {
                let students = plan.specs[i].0.students();
                let shared = students
                    .iter()
                    .filter(|student| {
                        level_specs
                            .iter()
                            .any(|&j| j != i && plan.specs[j].0.students().contains(student))
                    })
                    .count();
                (shared, students.len(), i)
            })
            .collect();
        keyed.sort_unstable();
        for (_shared, _size, i) in keyed {
            spec_epochs[i] = next_epoch;
            next_epoch += 1;
        }
    }

    // The base variable is the whole assignment matrix, so a spec's epoch
    // covers every (student, group) binary of its list — the group count
    // comes from a local `VarEnv`, which only needs the plan.
    let env = VarEnv::new(plan);
    let mut epochs = HashMap::new();
    for (i, (spec, _covered)) in plan.specs.iter().enumerate() {
        let list = GroupListIdx(i);
        for &student in spec.students() {
            for group in 0..env.group_count(list) {
                epochs.insert(
                    Var::StudentInGroup {
                        list,
                        student,
                        group,
                    },
                    spec_epochs[i],
                );
            }
        }
    }
    epochs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vars::tests::plan_of;
    use collomatique_ilp_modeler::DescribeVar;
    use std::collections::BTreeSet;

    /// The epoch shared by every variable of a list, asserting on the way
    /// that the list's variables all agree on it.
    fn epoch_of(epochs: &HashMap<Var, u32>, plan: &GenerationPlan, list: usize) -> u32 {
        let env = VarEnv::new(plan);
        let idx = GroupListIdx(list);
        let group_count = env.group_count(idx);
        let values: BTreeSet<u32> = plan.specs[list]
            .0
            .students()
            .iter()
            .flat_map(|&student| {
                (0..group_count).map(move |group| Var::StudentInGroup {
                    list: idx,
                    student,
                    group,
                })
            })
            .map(|var| epochs[&var])
            .collect();
        assert_eq!(values.len(), 1, "all variables of a spec share its epoch");
        values.into_iter().next().unwrap()
    }

    #[test]
    fn disjoint_sets_of_a_level_split_smaller_first() {
        // Both specs are inclusion-minimal (level 0), and they share no
        // student, so the shared counts are both 0 and the sort key falls
        // through to the size: the smaller list ({4,5}) solves first.
        // Different sizes on purpose:
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
    fn overlapping_incomparable_sets_get_their_own_epochs() {
        // {1,2} and {2,3,4} overlap but neither contains the other; the
        // sizes differ so the strictness test alone cannot save a broken
        // subset test. Both sit at level 0. Piece 12 solved them jointly
        // (one component); piece 12bis gives each its own epoch. Each
        // shares exactly one student (2) with the other, so the size
        // tie-break orders {1,2} first.
        let plan = plan_of(&[(&[1, 2], (1, 2)), (&[2, 3, 4], (1, 3))]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 1);
    }

    #[test]
    fn equal_sets_never_relate() {
        // Same students, different ranges: two distinct specs (dedup only
        // merges identical (set, range) keys), and neither strictly
        // includes the other, so both sit at level 0 — neither waits for
        // the other at a higher level, which is what "never relate" pins.
        // They then split into two epochs of the same level, fully tied on
        // shared count (each shares all 3 students) and on size, so the
        // spec index breaks the tie.
        let plan = plan_of(&[(&[1, 2, 3], (1, 2)), (&[1, 2, 3], (2, 3))]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 1);
    }

    #[test]
    fn roadmap_example_heights() {
        // §2.6's worked example, shrunk: two disjoint LV2 lists, a
        // sciences list containing both, the whole class containing
        // everything. Levels 0, 0, 1, 2 — and the two LV2 lists are
        // disjoint (shared count 0) and of equal size, so the index
        // tie-break orders them. Epochs 0, 1, 2, 3: the inclusion levels
        // stay contiguous runs of epoch numbers.
        //
        // (That sharing is counted per level is pinned by the dedicated
        // `sharing_is_counted_within_the_level_only` test.)
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
        // holds two disjoint specs, ordered by size ({1} first), so the
        // epochs run 0, 1, 2, 3. Under the last-wins mutation the top
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
        // in scrambled order: all shared counts are 0, so the size
        // tie-break of the sort key decides — the epochs follow the sizes,
        // not the plan order.
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
    fn least_shared_specs_solve_first_within_a_level() {
        // All four specs are level 0. Shared students within the level:
        // {1,2,3,4,5} shares none, {6,7} shares one (7), {8,9,10} shares
        // one (8), {7,8} shares two (7 with the first, 8 with the second).
        // The epochs follow the shared counts, NOT the sizes: the biggest
        // list solves first because it is untangled, and the smallest list
        // solves last because it is the most entangled. Within shared
        // count 1, size orders {6,7} before {8,9,10}.
        let plan = plan_of(&[
            (&[1, 2, 3, 4, 5], (1, 5)),
            (&[6, 7], (1, 2)),
            (&[7, 8], (1, 2)),
            (&[8, 9, 10], (1, 3)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0); // shared 0
        assert_eq!(epoch_of(&epochs, &plan, 1), 1); // shared 1, 2 students
        assert_eq!(epoch_of(&epochs, &plan, 3), 2); // shared 1, 3 students
        assert_eq!(epoch_of(&epochs, &plan, 2), 3); // shared 2
    }

    #[test]
    fn sharing_is_counted_within_the_level_only() {
        // The whole-class list {1..6} contains everything, so it shares
        // students with every level-0 spec. Counted globally, {1,2,3}
        // would score 3 and the overlapping pair {4,5}/{5,6} would score 2
        // each, reordering the level. Counted within level 0 as required,
        // {1,2,3} shares nothing and solves first despite being the
        // biggest.
        let plan = plan_of(&[
            (&[1, 2, 3], (1, 3)),
            (&[4, 5], (1, 2)),
            (&[5, 6], (1, 2)),
            (&[1, 2, 3, 4, 5, 6], (1, 6)),
        ]);
        let epochs = build_incremental_epochs(&plan);
        assert_eq!(epoch_of(&epochs, &plan, 0), 0);
        assert_eq!(epoch_of(&epochs, &plan, 1), 1);
        assert_eq!(epoch_of(&epochs, &plan, 2), 2);
        assert_eq!(epoch_of(&epochs, &plan, 3), 3);
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
