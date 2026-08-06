//! `Σ_g StudentInGroup(list, student, g)` rows for every non-excluded student
//! of an Automatic group list: `>= 1` (blamable, `StudentHasGroup` — the
//! student must be placed) and `<= 1` (structural, `StudentInAtMostOneGroup`
//! — what the retired integer variable's domain used to say; the all-zeros
//! row is the old -1 sentinel and decodes as "unassigned").
//!
//! Prefilled lists are skipped: their binaries are all fixed by the fixer, so
//! the rows would reduce to variable-free constants at build. A zero-group
//! Automatic list deliberately emits `0 >= 1` (an empty sum): infeasible but
//! blamed on `StudentHasGroup`, matching the old `x >= 0` over a [-1,-1]
//! domain.

use crate::extras::{MyBundle, V, base_var};
use crate::ids::GroupNum;
use crate::types::StructuralConstraint;
use crate::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::group_lists::GroupListFilling;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (group_list, gl) in env.group_lists.group_list_map.iter() {
        let GroupListFilling::Automatic { excluded_students } = gl.filling() else {
            continue;
        };
        let groups: Vec<GroupNum> = GroupNum::enumerate(env, group_list).collect();
        for student in env.students.student_map.keys() {
            if excluded_students.contains(&student) {
                continue;
            }
            let sum: IntLinExpr<V> = groups
                .iter()
                .map(|&group| {
                    IntLinExpr::var(base_var(Var::StudentInGroup {
                        group_list,
                        student,
                        group,
                    }))
                })
                .sum();
            if !groups.is_empty() {
                bundle = bundle.with_constraint(
                    sum.clone().leq(&IntLinExpr::constant(1)),
                    StructuralConstraint::StudentInAtMostOneGroup {
                        student,
                        group_list,
                    }
                    .into(),
                );
            }
            bundle = bundle.with_constraint(
                sum.geq(&IntLinExpr::constant(1)),
                StructuralConstraint::StudentHasGroup {
                    student,
                    group_list,
                }
                .into(),
            );
        }
    }
    bundle
}
