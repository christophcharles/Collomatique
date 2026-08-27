//! The variable environment and the base variable of the model.

use crate::ghost::GhostGrouping;
use crate::specs::{GenerationPlan, GroupListSpec};
use collomatique_state_colloscopes::{NonEmptyRangeInclusive, StudentId};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

/// Pre-loaded data for variable enumeration: the deduplicated specs of a
/// [`GenerationPlan`], in plan order, together with their group targets, the
/// size classes they fall into, the pairs already grouped by the kept lists
/// (the extras of piece 7 read them), the canonical group size the classes
/// are weighed against and the template grouping the objective measures
/// deviations from.
#[derive(Debug, Clone)]
pub struct VarEnv {
    specs: Vec<GroupListSpec>,
    /// Per list, in plan order, the group sizes of
    /// [`balanced_targets`](crate::targets): the shape the greedy fills and
    /// the model pins. Its length is the list's group count.
    targets: Vec<Vec<u32>>,
    /// The distinct `students_per_group` ranges of the specs, sorted.
    /// [`SizeClassIdx`] indexes into this vector.
    classes: Vec<NonEmptyRangeInclusive<NonZeroU32>>,
    /// Per class, the pairs pinned by a kept list of that same range. Same
    /// indexing as `classes`; a class no kept list matches gets an empty set.
    pinned_pairs: Vec<BTreeSet<(StudentId, StudentId)>>,
    canonical_range: Option<NonEmptyRangeInclusive<NonZeroU32>>,
    /// The template grouping. Deliberately *not* one of the `specs`: it is
    /// plan data rather than a list, so it gets no variable of its own, no
    /// `SharedPair`, and never becomes an output list. The model only reads
    /// it through [`VarEnv::ref_groups`] and [`VarEnv::ref_group`].
    ghost: Option<GhostGrouping>,
}

impl VarEnv {
    pub fn new(plan: &GenerationPlan) -> VarEnv {
        let specs: Vec<GroupListSpec> = plan.specs.iter().map(|(spec, _)| spec.clone()).collect();
        let targets: Vec<Vec<u32>> = specs
            .iter()
            .map(|spec| {
                let n = u32::try_from(spec.students().len()).unwrap_or(u32::MAX);
                crate::targets::balanced_targets(n, spec.students_per_group())
            })
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
            targets,
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
    /// feasible count — the length of the list's target table. It is exact,
    /// not an upper bound — every group holds between `min_size` and
    /// `max_size` students, so the model no longer has to optimize the count.
    ///
    /// [`GroupListSpec::new`] rejects a spec with no feasible count, so
    /// `count · min <= n <= count · max` always holds here, and `n >= 1`
    /// makes the count at least 1.
    ///
    /// Panics if `list` is not an index of the plan the env was built from.
    pub fn group_count(&self, list: GroupListIdx) -> u32 {
        self.targets[list.0].len() as u32
    }

    /// The group sizes of a list, indexed by group — the balanced targets of
    /// [`balanced_targets`](crate::targets), summing to the list's student
    /// count. Panics on a stale index, like [`VarEnv::group_count`].
    pub(crate) fn targets(&self, list: GroupListIdx) -> &[u32] {
        &self.targets[list.0]
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

    /// The largest allowed group size of a list's spec. Panics on a stale
    /// index, like [`VarEnv::group_count`].
    ///
    /// Nothing reads it since the objective swap — the group sizes are pinned
    /// at their targets, which is a finer question than the range — and it
    /// goes with the rest of the ILP-era env in the retirement commit.
    #[allow(dead_code)]
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

    /// Weight of a size class in the retired stability objective, by class
    /// index: [`crate::specs::class_weight`] applied to this env's canonical
    /// range. Only the build log still reports it.
    pub(crate) fn class_weight(&self, class: SizeClassIdx) -> f64 {
        crate::specs::class_weight(self.canonical_range.as_ref(), self.class_range(class))
    }

    /// The pairs fixed to "already shared" *in this class* by the kept lists
    /// (`a < b`). Panics on a stale class index.
    ///
    /// Dead since the objective swap: the kept lists enter the collision
    /// objective as constant mass, not as a discount on a pair variable. It
    /// goes with the rest of the ILP-era env in the retirement commit, as do
    /// [`VarEnv::ref_groups`] and [`VarEnv::ref_group`] just below.
    #[allow(dead_code)]
    pub(crate) fn pinned_pairs(&self, class: SizeClassIdx) -> &BTreeSet<(StudentId, StudentId)> {
        &self.pinned_pairs[class.0]
    }

    /// The reference groups of the template, in build order. Empty without a
    /// template, so every family that loops over them self-gates on the
    /// plan having one.
    #[allow(dead_code)]
    pub(crate) fn ref_groups(&self) -> impl Iterator<Item = RefGroupIdx> {
        (0..self.ghost_group_count() as usize).map(RefGroupIdx)
    }

    /// The students of one reference group. Panics without a template, or on
    /// a stale index — like [`VarEnv::group_count`], and for the same reason:
    /// the index can only have come from [`VarEnv::ref_groups`].
    #[allow(dead_code)]
    pub(crate) fn ref_group(&self, ref_group: RefGroupIdx) -> &BTreeSet<StudentId> {
        &self
            .ghost
            .as_ref()
            .expect("a reference group index implies a template")
            .groups()[ref_group.0]
    }

    /// Number of groups of the template: the length of its group vector,
    /// which [`build_ghost`](crate::ghost) built at the same closed count as
    /// [`VarEnv::group_count`]. 0 without a template, so
    /// [`VarEnv::ref_groups`] is the empty iterator there.
    pub(crate) fn ghost_group_count(&self) -> u32 {
        match &self.ghost {
            Some(ghost) => ghost.groups().len() as u32,
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

/// Index into the group vector of the template
/// ([`crate::GhostGrouping`]): one of the reference groups the
/// generated lists are asked to reuse. Unlike [`SizeClassIdx`] it names a
/// concrete set of students, since the template is computed rather than
/// solved.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RefGroupIdx(pub usize);

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
    /// `crate::constraints::student_in_one_group`. No variable is ever
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
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::specs::KeptList;
    use crate::specs::tests::{period_id, range, set, student, subject_id};
    use collomatique_ilp_modeler::DescribeVar;
    use collomatique_state_colloscopes::{PeriodId, SubjectId};
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
        let ghost = crate::ghost::build_ghost(&specs, &BTreeMap::new(), canonical_range.as_ref());
        GenerationPlan {
            specs,
            skipped: BTreeSet::new(),
            pinned_pairs: BTreeMap::new(),
            canonical_range,
            ghost,
            kept_lists: Vec::new(),
        }
    }

    /// `count` distinct (period, subject) pairs, so a spec can be given a
    /// multiplicity. Coverage is read honestly — a spec covering nothing
    /// weighs nothing in the objective — so anything that exercises scoring
    /// must supply pairs.
    fn covered(spec: usize, count: usize) -> BTreeSet<(PeriodId, SubjectId)> {
        (0..count as u64)
            .map(|i| (period_id(1), subject_id(spec as u64 * 100 + i + 1)))
            .collect()
    }

    /// A plan that carries coverage: `specs` are `(students, (min, max), use
    /// count)` and `kept` are `(groups, use count)`. Unlike [`plan_of`], whose
    /// specs cover nothing, this is what the collision objective needs — a
    /// multiplicity-0 list is filtered out of the pair enumeration entirely.
    ///
    /// The ILP-era fields stay empty; the collision objective never reads
    /// them.
    pub(crate) fn plan_with_uses(
        specs: &[(&[u64], (u32, u32), usize)],
        kept: &[(&[&[u64]], usize)],
    ) -> GenerationPlan {
        GenerationPlan {
            specs: specs
                .iter()
                .enumerate()
                .map(|(i, &(students, (min, max), uses))| {
                    let spec =
                        GroupListSpec::new(set(students), range(min, max)).expect("feasible spec");
                    (spec, covered(i, uses))
                })
                .collect(),
            skipped: BTreeSet::new(),
            pinned_pairs: BTreeMap::new(),
            canonical_range: None,
            ghost: None,
            kept_lists: kept
                .iter()
                .map(|&(groups, use_count)| KeptList {
                    groups: groups.iter().map(|group| set(group)).collect(),
                    use_count,
                })
                .collect(),
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
    fn targets_are_the_balanced_sizes() {
        let plan = plan_of(&[
            (&[1, 2, 3, 4, 5, 6], (2, 3)),
            (&[1, 2, 3, 4, 5, 6, 7], (2, 3)),
            (&[1, 2, 3], (3, 4)),
        ]);
        let env = VarEnv::new(&plan);

        // Balanced around n / k, descending: 6 = 3 + 3, 7 = 3 + 2 + 2, 3 = 3.
        assert_eq!(env.targets(GroupListIdx(0)), &[3, 3]);
        assert_eq!(env.targets(GroupListIdx(1)), &[3, 2, 2]);
        assert_eq!(env.targets(GroupListIdx(2)), &[3]);

        // The table length *is* the group count, for every list.
        for list in env.lists() {
            assert_eq!(env.targets(list).len(), env.group_count(list) as usize);
            let seated: u32 = env.targets(list).iter().sum();
            assert_eq!(seated, env.students(list).len() as u32);
        }
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
        // One binary per (list, student, group): 4 students × ceil(4/3)
        // groups, plus 3 students × ceil(3/2) groups. The template adds
        // none — it is plan data, not a matrix the solver decides.
        assert_eq!(vars.len(), 4 * 2 + 3 * 2);

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
}
