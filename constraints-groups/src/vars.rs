//! The variable environment and the base variable of the model.

use crate::specs::{GenerationPlan, GroupListSpec};
use collomatique_state_colloscopes::{NonEmptyRangeInclusive, StudentId};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

/// Pre-loaded data for variable enumeration: the deduplicated specs of a
/// [`GenerationPlan`], in plan order, together with the size classes they
/// fall into, the pairs already grouped by the kept lists (the extras of
/// piece 7 read them), the canonical group size the classes are weighed
/// against and the template grouping the objective measures deviations from.
#[derive(Debug, Clone)]
pub struct VarEnv {
    specs: Vec<GroupListSpec>,
    /// Per list, how many (period, subject) slots its spec covers, floored
    /// at 1 — how many times the grouping is actually used. Same indexing as
    /// `specs`; same floor as the canonical vote's weight.
    multiplicities: Vec<u64>,
    /// The distinct `students_per_group` ranges of the specs, sorted.
    /// [`SizeClassIdx`] indexes into this vector.
    classes: Vec<NonEmptyRangeInclusive<NonZeroU32>>,
    /// Per class, the pairs pinned by a kept list of that same range. Same
    /// indexing as `classes`; a class no kept list matches gets an empty set.
    pinned_pairs: Vec<BTreeSet<(StudentId, StudentId)>>,
    canonical_range: Option<NonEmptyRangeInclusive<NonZeroU32>>,
    /// The template grouping. Deliberately *not* one of the `specs`: it has
    /// its own variable, gets no `SharedPair` and never becomes an output
    /// list.
    ghost: Option<GroupListSpec>,
}

impl VarEnv {
    pub fn new(plan: &GenerationPlan) -> VarEnv {
        let specs: Vec<GroupListSpec> = plan.specs.iter().map(|(spec, _)| spec.clone()).collect();
        let multiplicities: Vec<u64> = plan
            .specs
            .iter()
            .map(|(_, covered)| covered.len().max(1) as u64)
            .collect();
        let classes: Vec<NonEmptyRangeInclusive<NonZeroU32>> = specs
            .iter()
            .map(|spec| spec.students_per_group().clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        // Pins of a range no spec uses are dropped here: their pairs have no
        // variable to discount.
        let pinned_pairs = classes
            .iter()
            .map(|range| plan.pinned_pairs.get(range).cloned().unwrap_or_default())
            .collect();
        VarEnv {
            specs,
            multiplicities,
            classes,
            pinned_pairs,
            canonical_range: plan
                .canonical_range
                .as_ref()
                .map(|(range, _source)| range.clone()),
            ghost: plan.ghost.clone(),
        }
    }

    /// Number of groups for a list: `ceil(n / max_size)`, the smallest
    /// feasible count. It is exact, not an upper bound — every group holds
    /// between `min_size` and `max_size` students, so the model no longer
    /// has to optimize the count.
    ///
    /// [`GroupListSpec::new`] rejects a spec with no feasible count, so
    /// `count · min <= n <= count · max` always holds here, and `n >= 1`
    /// makes the count at least 1.
    ///
    /// Panics if `list` is not an index of the plan the env was built from.
    pub fn group_count(&self, list: GroupListIdx) -> u32 {
        let spec = &self.specs[list.0];
        let n = spec.students().len() as u32;
        let max = spec.students_per_group().end().get();
        n.div_ceil(max)
    }

    /// How many (period, subject) slots `list`'s spec covers (at least 1):
    /// the weight of that list when the objective asks how much two students
    /// share. Panics on a stale index, like [`VarEnv::group_count`].
    pub(crate) fn multiplicity(&self, list: GroupListIdx) -> u64 {
        self.multiplicities[list.0]
    }

    /// The list indices of the plan, in order.
    pub(crate) fn lists(&self) -> impl Iterator<Item = GroupListIdx> {
        (0..self.specs.len()).map(GroupListIdx)
    }

    /// The students of a list's spec. Panics on a stale index, like
    /// [`VarEnv::group_count`].
    pub(crate) fn students(&self, list: GroupListIdx) -> &BTreeSet<StudentId> {
        self.specs[list.0].students()
    }

    /// The smallest allowed group size of a list's spec. Panics on a stale
    /// index, like [`VarEnv::group_count`].
    pub(crate) fn min_size(&self, list: GroupListIdx) -> u32 {
        self.specs[list.0].students_per_group().start().get()
    }

    /// The largest allowed group size of a list's spec. Panics on a stale
    /// index, like [`VarEnv::group_count`].
    pub(crate) fn max_size(&self, list: GroupListIdx) -> u32 {
        self.specs[list.0].students_per_group().end().get()
    }

    /// The size classes of the plan, in table order (sorted by range).
    pub(crate) fn classes(&self) -> impl Iterator<Item = SizeClassIdx> {
        (0..self.classes.len()).map(SizeClassIdx)
    }

    /// The size class of a list: the position of its `students_per_group`
    /// range in the class table. Panics on a stale index, like
    /// [`VarEnv::group_count`].
    pub(crate) fn class_of(&self, list: GroupListIdx) -> SizeClassIdx {
        let range = self.specs[list.0].students_per_group();
        SizeClassIdx(
            self.classes
                .binary_search(range)
                .expect("every spec's range is a class of the table"),
        )
    }

    /// The size range a class stands for. Panics on a stale class index.
    pub(crate) fn class_range(&self, class: SizeClassIdx) -> &NonEmptyRangeInclusive<NonZeroU32> {
        &self.classes[class.0]
    }

    /// Weight of a size class in the stability objective: how much a pair
    /// meeting in a group of this class matters, relative to a meeting at
    /// the canonical size. 1 for the canonical class and anything tighter,
    /// then decaying like `(canonical_max − 1) / (class_max − 1)` — in a
    /// tutorial group of 20 every student meets 19 others whatever the model
    /// does, so such a meeting is a far weaker tie than one in a group of 3,
    /// and pricing them alike lets tutorials pre-pay (and thereby free) every
    /// colle pair.
    ///
    /// Without a canonical range (a plan with no specs) every class weighs 1.
    /// A class of maximum size 1 never meets at all and is skipped upstream
    /// ([`co_occurrences`](crate::extras::co_occurrences)), so the divisor is
    /// never 0; a *canonical* size of 1 is read as 2 instead, so that a
    /// document whose typical subject takes students one at a time still
    /// ranks its real groups by size rather than zeroing the objective.
    pub(crate) fn class_weight(&self, class: SizeClassIdx) -> f64 {
        let Some(canonical) = &self.canonical_range else {
            return 1.0;
        };
        let canon_max = canonical.end().get().max(2);
        let class_max = self.class_range(class).end().get();
        (f64::from(canon_max - 1) / f64::from(class_max - 1)).min(1.0)
    }

    /// The pairs fixed to "already shared" *in this class* by the kept lists
    /// (`a < b`). Panics on a stale class index.
    pub(crate) fn pinned_pairs(&self, class: SizeClassIdx) -> &BTreeSet<(StudentId, StudentId)> {
        &self.pinned_pairs[class.0]
    }

    /// The template grouping, or `None` when the plan has none. It is not
    /// one of the [`lists`](VarEnv::lists): the objective measures the real
    /// lists *against* it, so it must be decided by the solver like a list
    /// but must never be counted as one.
    pub(crate) fn ghost(&self) -> Option<&GroupListSpec> {
        self.ghost.as_ref()
    }

    /// Number of groups of the template, by the same closed form as
    /// [`VarEnv::group_count`]. 0 without a template, so `0..count` is the
    /// empty loop everywhere the ghost pass runs.
    pub(crate) fn ghost_group_count(&self) -> u32 {
        match &self.ghost {
            Some(ghost) => {
                let n = ghost.students().len() as u32;
                n.div_ceil(ghost.students_per_group().end().get())
            }
            None => 0,
        }
    }
}

/// Index into the deduplicated spec vector of a [`GenerationPlan`].
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupListIdx(pub usize);

/// Index into the deduplicated size-class table of a [`VarEnv`]: the distinct
/// `students_per_group` ranges of the plan's specs, sorted. Two lists of the
/// same class are groupings of the same shape, and only inside a class does
/// reusing a pair mean anything.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SizeClassIdx(pub usize);

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, collomatique_ilp_modeler::DescribeVar,
)]
#[env(VarEnv)]
pub enum Var {
    /// 1 ⟺ `student` sits in group `group` of `list`. Binary (the derive
    /// default): the assignment matrix *is* the base variable, so no
    /// channeling is needed to reach it from the constraints.
    ///
    /// "Exactly one group per student" is not a property of the domain any
    /// more — it is the constraint family of
    /// [`crate::constraints`]`::student_in_one_group`. No variable is ever
    /// fixed in this crate, so no fix attribute: the derive's default
    /// `check_fix` returns `None` for in-range names and `Some(0.0)` for
    /// stale ones.
    StudentInGroup {
        #[range(Self::compute_list_range(env))]
        list: GroupListIdx,
        #[range(Self::compute_student_range(env, list))]
        student: StudentId,
        #[range(Self::compute_group_range(env, list))]
        group: u32,
    },
    /// 1 ⟺ `student` sits in group `group` of the *template* grouping — the
    /// one grouping of the whole student body the objective asks the real
    /// lists to resemble. Decided by the solver like any list (the shape
    /// constraints partition it too), but never converted to output: it is a
    /// yardstick, not a group list.
    ///
    /// Enumerated only when the plan has a template
    /// ([`GenerationPlan::ghost`](crate::GenerationPlan::ghost)); without
    /// one, both range helpers are empty and every such name is stale, hence
    /// neutralized to 0 by the derive's default `check_fix`.
    StudentInGhostGroup {
        #[range(Self::compute_ghost_student_range(env))]
        student: StudentId,
        #[range(Self::compute_ghost_group_range(env))]
        group: u32,
    },
}

impl Var {
    fn compute_list_range(env: &VarEnv) -> Vec<GroupListIdx> {
        (0..env.specs.len()).map(GroupListIdx).collect()
    }

    /// Defensive against a stale `list` (as the sibling crate's range
    /// helpers are): `check_fix` checks the fields in declaration order and
    /// bails out on a stale `list` before reaching here, but this helper is
    /// callable on its own.
    fn compute_student_range(env: &VarEnv, list: &GroupListIdx) -> Vec<StudentId> {
        match env.specs.get(list.0) {
            Some(spec) => spec.students().iter().copied().collect(),
            None => Vec::new(),
        }
    }

    /// Defensive against a stale `list` too — [`VarEnv::group_count`] would
    /// panic on one.
    fn compute_group_range(env: &VarEnv, list: &GroupListIdx) -> Vec<u32> {
        if list.0 < env.specs.len() {
            (0..env.group_count(*list)).collect()
        } else {
            Vec::new()
        }
    }

    fn compute_ghost_student_range(env: &VarEnv) -> Vec<StudentId> {
        match env.ghost() {
            Some(ghost) => ghost.students().iter().copied().collect(),
            None => Vec::new(),
        }
    }

    /// Empty without a template, since [`VarEnv::ghost_group_count`] is 0
    /// there — the whole variant then enumerates to nothing.
    fn compute_ghost_group_range(env: &VarEnv) -> Vec<u32> {
        (0..env.ghost_group_count()).collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::specs::tests::{range, set, student};
    use collomatique_ilp_modeler::DescribeVar;
    use std::collections::{BTreeMap, BTreeSet};

    /// A plan of bare specs, with no covered pairs (the model never reads
    /// them). The specs must be feasible — an unsatisfiable one has no
    /// place in a plan at all. The canonical range and the template are
    /// resolved from the specs by the production functions, so a test plan
    /// weighs its size classes and templates its lists exactly as a real one
    /// would.
    pub(crate) fn plan_of(specs: &[(&[u64], (u32, u32))]) -> GenerationPlan {
        let specs: Vec<_> = specs
            .iter()
            .map(|(students, (min, max))| {
                (
                    GroupListSpec::new(set(students), range(*min, *max))
                        .expect("feasible test spec"),
                    BTreeSet::new(),
                )
            })
            .collect();
        let canonical_range = crate::specs::elect_canonical_range(&specs)
            .map(|range| (range, crate::specs::RangeSource::Automatic));
        let ghost = crate::specs::build_ghost(&specs, canonical_range.as_ref());
        GenerationPlan {
            specs,
            skipped: BTreeSet::new(),
            pinned_pairs: BTreeMap::new(),
            canonical_range,
            ghost,
        }
    }

    #[test]
    fn group_count_formula() {
        let plan = plan_of(&[
            (&[1, 2, 3, 4, 5, 6], (2, 3)),
            (&[1, 2, 3, 4, 5, 6, 7], (2, 3)),
            (&[1, 2, 3], (3, 4)),
        ]);
        let env = VarEnv::new(&plan);

        assert_eq!(env.group_count(GroupListIdx(0)), 2); // ceil(6 / 3)
        assert_eq!(env.group_count(GroupListIdx(1)), 3); // ceil(7 / 3)
        assert_eq!(env.group_count(GroupListIdx(2)), 1); // ceil(3 / 4)
    }

    #[test]
    fn size_classes_dedup_and_weigh_by_distance_to_the_canonical_size() {
        // Two lists of 2..=3 (four students each, so 8 votes) against one
        // tutorial list of 6..=6 (6 votes): the small range is canonical.
        let plan = plan_of(&[
            (&[1, 2, 3, 4], (2, 3)),
            (&[3, 4, 5, 6], (2, 3)),
            (&[1, 2, 3, 4, 5, 6], (6, 6)),
        ]);
        let env = VarEnv::new(&plan);

        // The table is the *distinct* ranges, sorted: 2..=3 then 6..=6.
        let small = env.class_of(GroupListIdx(0));
        assert_eq!(small, SizeClassIdx(0));
        assert_eq!(env.class_of(GroupListIdx(1)), small);
        let big = env.class_of(GroupListIdx(2));
        assert_eq!(big, SizeClassIdx(1));
        assert_eq!(*env.class_range(big), range(6, 6));

        // The canonical class weighs 1; a group of 6 ties its members five
        // times as loosely as a group of 3.
        assert_eq!(env.class_weight(small), 1.0);
        assert_eq!(env.class_weight(big), 2.0 / 5.0);
    }

    #[test]
    fn classes_tighter_than_canonical_weigh_one() {
        // 6..=6 carries 6 votes against 4, so the tutorial size is canonical
        // here. The formula would give the small class a weight of 5; the
        // clamp keeps a meeting worth at most one meeting.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4, 5, 6], (6, 6))]);
        let env = VarEnv::new(&plan);

        assert_eq!(env.class_weight(env.class_of(GroupListIdx(0))), 1.0);
        assert_eq!(env.class_weight(env.class_of(GroupListIdx(1))), 1.0);
    }

    #[test]
    fn pins_are_read_per_class() {
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[1, 2, 3, 4], (2, 3))]);
        plan.pinned_pairs = [
            (
                range(2, 2),
                [(student(1), student(2))].into_iter().collect(),
            ),
            // A range no spec uses: dropped, since its pairs have no
            // variable in any class.
            (
                range(4, 4),
                [(student(3), student(4))].into_iter().collect(),
            ),
        ]
        .into_iter()
        .collect();
        let env = VarEnv::new(&plan);

        let tight = env.class_of(GroupListIdx(0));
        let loose = env.class_of(GroupListIdx(1));
        assert_eq!(
            *env.pinned_pairs(tight),
            BTreeSet::from([(student(1), student(2))])
        );
        assert!(env.pinned_pairs(loose).is_empty());
    }

    #[test]
    fn enumeration_and_default_fix() {
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 3)), (&[5, 6, 7], (1, 2))]);
        let env = VarEnv::new(&plan);

        let vars = <Var as DescribeVar>::enumerate(&env);
        // One binary per (student, group): 4 students × ceil(4/3) groups,
        // plus 3 students × ceil(3/2) groups — and the template's own
        // matrix, 7 students in ceil(7/3) groups at the canonical 2..=3.
        assert_eq!(vars.len(), 4 * 2 + 3 * 2 + 7 * 3);

        // A variable of the enumerated set is free: nothing is ever fixed.
        let free = Var::StudentInGroup {
            list: GroupListIdx(0),
            student: student(1),
            group: 1,
        };
        assert!(vars.contains_key(&free));
        assert_eq!(free.check_fix(&env), None);

        // A student that belongs to the other spec is a stale name and is
        // neutralized to 0.
        let stale_student = Var::StudentInGroup {
            list: GroupListIdx(0),
            student: student(5),
            group: 0,
        };
        assert!(!vars.contains_key(&stale_student));
        assert_eq!(stale_student.check_fix(&env), Some(0.0));

        // So is a list index beyond the plan.
        let stale_list = Var::StudentInGroup {
            list: GroupListIdx(2),
            student: student(1),
            group: 0,
        };
        assert_eq!(stale_list.check_fix(&env), Some(0.0));

        // And so is a group index beyond the list's group count.
        let stale_group = Var::StudentInGroup {
            list: GroupListIdx(0),
            student: student(1),
            group: 2,
        };
        assert!(!vars.contains_key(&stale_group));
        assert_eq!(stale_group.check_fix(&env), Some(0.0));
    }

    #[test]
    fn the_template_gets_its_own_assignment_matrix() {
        // Two overlapping lists of the same range: the template spans their
        // union — six students in ceil(6 / 2) = 3 groups.
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 2)), (&[3, 4, 5, 6], (2, 2))]);
        let env = VarEnv::new(&plan);
        assert_eq!(env.ghost_group_count(), 3);

        let vars = <Var as DescribeVar>::enumerate(&env);
        assert_eq!(vars.len(), 4 * 2 + 4 * 2 + 6 * 3);

        let free = Var::StudentInGhostGroup {
            student: student(1),
            group: 2,
        };
        assert!(vars.contains_key(&free));
        assert_eq!(free.check_fix(&env), None);

        // Stale template names are neutralized exactly like stale list ones.
        let stale_group = Var::StudentInGhostGroup {
            student: student(1),
            group: 3,
        };
        assert!(!vars.contains_key(&stale_group));
        assert_eq!(stale_group.check_fix(&env), Some(0.0));
        let stale_student = Var::StudentInGhostGroup {
            student: student(9),
            group: 0,
        };
        assert!(!vars.contains_key(&stale_student));
        assert_eq!(stale_student.check_fix(&env), Some(0.0));
    }

    #[test]
    fn a_plan_without_a_template_has_no_template_variables() {
        // A plan whose canonical size cannot split the union has no ghost,
        // and then the whole variant enumerates to nothing.
        let mut plan = plan_of(&[(&[1, 2, 3, 4], (2, 2))]);
        plan.ghost = None;
        let env = VarEnv::new(&plan);
        assert_eq!(env.ghost_group_count(), 0);

        let vars = <Var as DescribeVar>::enumerate(&env);
        assert_eq!(vars.len(), 4 * 2);
        assert_eq!(
            Var::StudentInGhostGroup {
                student: student(1),
                group: 0,
            }
            .check_fix(&env),
            Some(0.0)
        );
    }
}
