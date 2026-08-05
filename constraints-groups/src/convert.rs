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
/// Panics on internal inconsistency: a missing `StudentGroup` value, an
/// out-of-domain group index, or a name-count mismatch are all caller or
/// solver bugs — every student of every spec must have a solved value.
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
            let slot_count = env.slot_count(list) as usize;

            let mut slots: Vec<BTreeSet<StudentId>> = vec![BTreeSet::new(); slot_count];
            for &student in &spec.students {
                let value = config
                    .get(Var::StudentGroup { list, student })
                    .expect("every spec student must have a solved StudentGroup value");
                let slot = value.round() as usize;
                assert!(slot < slot_count, "solved group index out of domain");
                slots[slot].insert(student);
            }

            // Compact away empty slots and remap group indices. The
            // ascending-fill constraint (piece 8) will make empties a
            // suffix, but the conversion never assumes it.
            let groups: Vec<PrefilledGroup> = slots
                .into_iter()
                .filter(|students| !students.is_empty())
                .map(|students| PrefilledGroup { students })
                .collect();

            let group_list_params = GroupListParameters {
                name: names[i].clone(),
                students_per_group: spec.students_per_group.clone(),
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
        // Minimum size 1 over 4 students → 4 slots, of which 2 stay empty.
        let plan = plan_of(&[(&[1, 2, 3, 4], (1, 4))]);
        let list = GroupListIdx(0);

        let config = ConfigData::new()
            .set(
                Var::StudentGroup {
                    list,
                    student: student(1),
                },
                0.0,
            )
            .set(
                Var::StudentGroup {
                    list,
                    student: student(2),
                },
                0.0,
            )
            .set(
                Var::StudentGroup {
                    list,
                    student: student(3),
                },
                2.0,
            )
            .set(
                Var::StudentGroup {
                    list,
                    student: student(4),
                },
                2.0,
            );

        let lists = build_group_lists(&plan, &[String::from("Liste")], &config);
        assert_eq!(lists.len(), 1);
        let (group_list, _covered) = &lists[0];

        assert_eq!(group_list.params().group_names.len(), 2);
        assert_eq!(group_list.filling().find_student_group(student(1)), Some(0));
        // Slot 2 was compacted down to group 1.
        assert_eq!(group_list.filling().find_student_group(student(3)), Some(1));
    }
}
