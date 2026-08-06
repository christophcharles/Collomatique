//! The variable environment and the base variable of the model.

use crate::specs::{GenerationPlan, GroupListSpec};
use collomatique_state_colloscopes::StudentId;
use std::collections::BTreeSet;

/// Pre-loaded data for variable enumeration: the deduplicated specs of a
/// [`GenerationPlan`], in plan order, together with the pairs already
/// grouped by the kept lists (the extras of piece 7 read them).
#[derive(Debug, Clone)]
pub struct VarEnv {
    specs: Vec<GroupListSpec>,
    pinned_pairs: BTreeSet<(StudentId, StudentId)>,
}

impl VarEnv {
    pub fn new(plan: &GenerationPlan) -> VarEnv {
        VarEnv {
            specs: plan.specs.iter().map(|(spec, _)| spec.clone()).collect(),
            pinned_pairs: plan.pinned_pairs.clone(),
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

    /// The pairs fixed to "already shared" by the kept lists (`a < b`).
    pub(crate) fn pinned_pairs(&self) -> &BTreeSet<(StudentId, StudentId)> {
        &self.pinned_pairs
    }
}

/// Index into the deduplicated spec vector of a [`GenerationPlan`].
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupListIdx(pub usize);

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, collomatique_ilp_modeler::DescribeVar,
)]
#[env(VarEnv)]
pub enum Var {
    /// The group the student sits in, as an integer in
    /// `0..=group_count-1`. Unlike the colloscope crate there is no −1
    /// value: every student of a spec is registered and must be placed, so
    /// the domain itself enforces "exactly one group". No variable is ever
    /// fixed in this crate, so no fix attribute: the derive's default
    /// `check_fix` returns `None` for in-range names and `Some(0.0)` for
    /// stale ones.
    #[var(Variable::integer().min(0.).max(Self::compute_max_group(env, list)))]
    StudentGroup {
        #[range(Self::compute_list_range(env))]
        list: GroupListIdx,
        #[range(Self::compute_student_range(env, list))]
        student: StudentId,
    },
}

impl Var {
    /// The group count is at least 1 (a spec always has a student), so the
    /// subtraction never underflows.
    fn compute_max_group(env: &VarEnv, list: &GroupListIdx) -> f64 {
        (env.group_count(*list) - 1) as f64
    }

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
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::specs::tests::{range, set, student};
    use collomatique_ilp_modeler::DescribeVar;
    use std::collections::BTreeSet;

    /// A plan of bare specs, with no covered pairs (the model never reads
    /// them). The specs must be feasible — an unsatisfiable one has no
    /// place in a plan at all.
    pub(crate) fn plan_of(specs: &[(&[u64], (u32, u32))]) -> GenerationPlan {
        GenerationPlan {
            specs: specs
                .iter()
                .map(|(students, (min, max))| {
                    (
                        GroupListSpec::new(set(students), range(*min, *max))
                            .expect("feasible test spec"),
                        BTreeSet::new(),
                    )
                })
                .collect(),
            skipped: BTreeSet::new(),
            pinned_pairs: BTreeSet::new(),
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
    fn enumeration_and_default_fix() {
        let plan = plan_of(&[(&[1, 2, 3, 4], (2, 3)), (&[5, 6, 7], (1, 2))]);
        let env = VarEnv::new(&plan);

        let vars = <Var as DescribeVar>::enumerate(&env);
        assert_eq!(vars.len(), 7); // 4 students + 3 students

        // A variable of the enumerated set is free: nothing is ever fixed.
        let free = Var::StudentGroup {
            list: GroupListIdx(0),
            student: student(1),
        };
        assert!(vars.contains_key(&free));
        assert_eq!(free.check_fix(&env), None);

        // A student that belongs to the other spec is a stale name and is
        // neutralized to 0.
        let stale_student = Var::StudentGroup {
            list: GroupListIdx(0),
            student: student(5),
        };
        assert!(!vars.contains_key(&stale_student));
        assert_eq!(stale_student.check_fix(&env), Some(0.0));

        // So is a list index beyond the plan.
        let stale_list = Var::StudentGroup {
            list: GroupListIdx(2),
            student: student(1),
        };
        assert_eq!(stale_list.check_fix(&env), Some(0.0));
    }
}
