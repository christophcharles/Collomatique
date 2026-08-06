//! Output side of the translation layer: from a solved base-variable
//! configuration back to prefilled `GroupList` values.

use crate::specs::GenerationPlan;
use crate::vars::{GroupListIdx, Var, VarEnv};
use collomatique_ilp::ConfigData;
use collomatique_state_colloscopes::group_lists::{
    GroupList, GroupListFilling, GroupListParameters, PrefilledGroup,
};
use collomatique_state_colloscopes::{PeriodId, StudentId, SubjectId};
use std::collections::BTreeSet;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specs::tests::student;
    use crate::vars::tests::plan_of;

    #[test]
    fn compaction_remaps_group_indices() {
        // 6 students in groups of 1 to 2 → 3 groups. The configuration is
        // in-domain but leaves the middle group empty, which the conversion
        // must compact away rather than emit.
        let plan = plan_of(&[(&[1, 2, 3, 4, 5, 6], (1, 2))]);
        let list = GroupListIdx(0);

        let mut config = ConfigData::new();
        for (s, slot) in [(1, 0), (2, 0), (3, 2), (4, 2), (5, 2), (6, 2)] {
            for group in 0..3 {
                config = config.set(
                    Var::StudentInGroup {
                        list,
                        student: student(s),
                        group,
                    },
                    if group == slot { 1.0 } else { 0.0 },
                );
            }
        }

        let lists = build_group_lists(&plan, &[String::from("Liste")], &config);
        assert_eq!(lists.len(), 1);
        let (group_list, _covered) = &lists[0];

        assert_eq!(group_list.params().group_names.len(), 2);
        assert_eq!(group_list.filling().find_student_group(student(1)), Some(0));
        // Slot 2 was compacted down to group 1.
        assert_eq!(group_list.filling().find_student_group(student(3)), Some(1));
    }
}
