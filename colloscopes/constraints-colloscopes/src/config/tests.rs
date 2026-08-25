use super::*;

use collomatique_ilp_modeler::MinimalBlame;
use collomatique_state_colloscopes::ids::{Id, StudentId, SubjectId};

use crate::ids::GroupNum;
use crate::types::{ProgressiveConstraint, QualityConstraint};

fn student(n: u64) -> StudentId {
    unsafe { StudentId::new(n) }
}
fn subject(n: u64) -> SubjectId {
    unsafe { SubjectId::new(n) }
}
fn group_list(n: u64) -> GroupListId {
    unsafe { GroupListId::new(n) }
}

/// A pin holding student `n` in group 0 of group list 0.
fn pin(n: u64) -> ConfiguredConstraintDesc {
    ConfiguredConstraintDesc::Fixed {
        var: Var::StudentInGroup {
            student: student(n),
            group_list: group_list(0),
            group: GroupNum::new_for_test(group_list(0), 0),
        },
        value: OrderedFloat(1.0),
    }
}

#[test]
fn distinct_pins_all_survive() {
    // No category, so nothing can suppress anything: two pins broken at once are
    // two things the user asked for and did not get.
    let blame: MinimalBlame<_> = [pin(1), pin(2)].into_iter().collect();
    assert_eq!(blame.len(), 2);
}

#[test]
fn the_same_pin_twice_is_one_violation() {
    let blame: MinimalBlame<_> = [pin(1), pin(1)].into_iter().collect();
    assert_eq!(blame.len(), 1);
}

#[test]
fn an_inner_violation_dedups_as_its_base_description_does() {
    // The `max_implies_exact_same_range_same_bound` pair from
    // `types::violation_order`'s tests: "at most 3" being violated over a range
    // implies "exactly 3" is violated over the same one, so only the implying
    // side is worth reporting.
    let max = ConfiguredConstraintDesc::Inner(ConstraintDesc::Level2(
        QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: GlobalWeek(0),
            last_week: GlobalWeek(5),
            max_count: 3,
        },
    ));
    let exact = ConfiguredConstraintDesc::Inner(ConstraintDesc::Level3(
        ProgressiveConstraint::PeriodicityInterrogationCountExact {
            student: student(1),
            subject: subject(1),
            first_week: GlobalWeek(0),
            last_week: GlobalWeek(5),
            count: 3,
        },
    ));

    let blame: MinimalBlame<_> = [exact, max.clone()].into_iter().collect();
    assert_eq!(blame.iter().collect::<Vec<_>>(), vec![&max]);
}

#[test]
fn a_pin_and_an_inner_violation_do_not_hide_each_other() {
    let inner = ConfiguredConstraintDesc::Inner(ConstraintDesc::Level1(
        StructuralConstraint::StudentHasGroup {
            student: student(1),
            group_list: group_list(0),
        },
    ));

    let blame: MinimalBlame<_> = [pin(1), inner].into_iter().collect();
    assert_eq!(blame.len(), 2);
}
