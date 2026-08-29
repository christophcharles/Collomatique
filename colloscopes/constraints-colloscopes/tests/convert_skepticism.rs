//! `convert::build_complete_config` must answer, not panic, on a colloscope that
//! does not match the parameters.
//!
//! Every caller inside the application hands it a colloscope taken straight out of
//! the document it also takes the parameters from, so the mismatch cannot happen
//! there. A colloscope built outside — the Python `model.blame` — is another matter,
//! and the conversion is the place that knows enough to judge it.

use collomatique_constraints_colloscopes::convert::{ConvertError, build_complete_config};
use collomatique_constraints_colloscopes::tools;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;
use collomatique_state_colloscopes::ids::{GroupListId, Id, SlotId, StudentId, WeekId};
use collomatique_storage::deserialize_data;
use std::collections::{BTreeMap, BTreeSet};

const FIXTURE: &str = include_str!("fixtures/period_with_group_list_association.collomatique");

/// The first interrogation cell of the fixture whose slot has a group list on its
/// period — the coordinates `build_config` looks up.
fn a_real_cell(params: &Parameters) -> (SlotId, WeekId, GroupListId) {
    for (period_id, week_id, _week) in params.walk_weeks() {
        for (slot_id, _slot) in params.slots.all_slots() {
            if !params.is_interrogation_possible(*slot_id, week_id) {
                continue;
            }
            if let Some(group_list_id) = tools::group_list_for_slot(params, period_id, *slot_id) {
                return (*slot_id, week_id, group_list_id);
            }
        }
    }
    panic!("the fixture should have at least one interrogation cell");
}

fn a_student(params: &Parameters) -> StudentId {
    params
        .students
        .student_map
        .keys()
        .next()
        .expect("the fixture should have at least one student")
}

/// One student in group 0, one group in the first real interrogation cell.
fn intact_colloscope(params: &Parameters) -> Colloscope {
    let (slot_id, week_id, group_list_id) = a_real_cell(params);
    let mut colloscope = Colloscope::default();
    colloscope.set_interrogation(slot_id, week_id, BTreeSet::from([0]));
    colloscope.set_group_list(group_list_id, BTreeMap::from([(a_student(params), 0)]));
    colloscope
}

#[test]
fn an_intact_colloscope_converts() {
    let (inner, _caveats) = deserialize_data(FIXTURE).expect("fixture should decode");
    let colloscope = intact_colloscope(&inner.params);

    assert!(build_complete_config(&inner.params, &colloscope).is_ok());
}

#[test]
fn an_out_of_range_group_number_is_refused() {
    let (inner, _caveats) = deserialize_data(FIXTURE).expect("fixture should decode");
    let (_slot_id, _week_id, group_list_id) = a_real_cell(&inner.params);

    // The setter is a dumb upsert, so a group number no group list is that long
    // goes straight in. It used to reach `GroupNum::new(..).expect(..)`.
    let mut colloscope = intact_colloscope(&inner.params);
    colloscope.set_group_list(
        group_list_id,
        BTreeMap::from([(a_student(&inner.params), 9999)]),
    );

    assert_eq!(
        build_complete_config(&inner.params, &colloscope).err(),
        Some(ConvertError::ColloscopeNotCompatibleWithParams)
    );
}

#[test]
fn an_unknown_week_is_refused() {
    let (inner, _caveats) = deserialize_data(FIXTURE).expect("fixture should decode");
    let (slot_id, _week_id, _group_list_id) = a_real_cell(&inner.params);

    // A week id the parameters never issued: `week_position` finds nothing, where
    // the conversion used to `expect` a hit.
    let mut colloscope = intact_colloscope(&inner.params);
    colloscope.set_interrogation(
        slot_id,
        unsafe { WeekId::new(u64::MAX) },
        BTreeSet::from([0]),
    );

    assert_eq!(
        build_complete_config(&inner.params, &colloscope).err(),
        Some(ConvertError::ColloscopeNotCompatibleWithParams)
    );
}
