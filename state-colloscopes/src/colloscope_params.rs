//! Colloscope parameters submodule
//!
//! This module defines the relevant types to describes the full set of parameters for colloscopes

use crate::ids::{
    GroupListId, IncompatId, NewId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekId, WeekPatternId,
};

use super::*;

use collomatique_state::Lookup;

/// Full set of parameters to describe the constraints for colloscopes
///
/// This structure contains all the parameters we might want to adjust
/// to define the constraints for a colloscope.
///
/// This structure is used in two ways:
/// - a main version is used in [InnerData] to represent the currently edited parameters
/// - another version is used for each colloscope to store the parameters used for its generation
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parameters {
    pub periods: periods::Periods,
    pub weeks: weeks::Weeks,
    pub subjects: subjects::Subjects,
    pub teachers: teachers::Teachers,
    pub students: students::Students,
    pub assignments: assignments::Assignments,
    pub week_patterns: week_patterns::WeekPatterns,
    pub slots: slots::Slots,
    pub incompats: incompats::Incompats,
    pub group_lists: group_lists::GroupLists,
    pub settings: settings::Settings,
    pub pairings: pairings::Pairings,
    pub slot_pairings: slot_pairings::SlotPairings,
    pub balancing: balancing::Balancing,
}

impl Parameters {
    /// The single definition of "a slot can carry an interrogation on `week_id`":
    /// the week runs interrogations and is not excluded by the slot's pattern
    /// (or the slot has no pattern). Shared by the colloscope, the constraints
    /// layer and the Python glue.
    pub fn is_week_active(&self, week_id: WeekId, pattern: Option<WeekPatternId>) -> bool {
        self.week_patterns
            .is_week_active(&self.weeks, week_id, pattern)
    }

    /// The single definition of "slot `slot` can carry an interrogation on week
    /// `week`": the slot's subject runs interrogations, is not excluded on that
    /// week's period, and the week is active for the slot's pattern.
    ///
    /// This mirrors exactly the dense skeleton's Some-cell rule
    /// (`ColloscopePeriod::new_empty_from_params` +
    /// `ColloscopeSlot::new_empty_from_params`): on validated data the predicate
    /// is true iff the dense cell exists and is `Some`. It is the possibility
    /// oracle behind the sparse colloscope surface and its consumers.
    pub fn is_interrogation_possible(&self, slot: SlotId, week: WeekId) -> bool {
        let Some((period_id, _pos)) = self.weeks.week_position(week) else {
            return false;
        };
        let Some((subject_id, slot_desc)) = self.slots.find_slot_with_subject(slot) else {
            return false;
        };
        let Some(subject) = self.subjects.find_subject(subject_id) else {
            return false;
        };
        if subject.parameters.interrogation_parameters.is_none() {
            return false;
        }
        if subject.excluded_periods.contains(&period_id) {
            return false;
        }
        self.is_week_active(week, slot_desc.week_pattern)
    }
}

impl Parameters {
    /// The canonical global week order: every week of every period, in
    /// period-then-position order, each with its identity (delegates to
    /// [`weeks::Weeks::walk`], passing the sibling periods for display order).
    pub fn walk_weeks(&self) -> impl Iterator<Item = (PeriodId, WeekId, &weeks::Week)> + '_ {
        self.weeks.walk(&self.periods)
    }

    /// All week ids, in global week order (delegates to
    /// [`weeks::Weeks::week_ids`]).
    pub fn week_ids(&self) -> impl Iterator<Item = WeekId> + '_ {
        self.weeks.week_ids(&self.periods)
    }

    /// Total number of weeks across all periods (delegates to
    /// [`weeks::Weeks::count_weeks`]).
    ///
    /// This reads the week *table*, whereas [`Self::walk_weeks`] and
    /// [`Self::week_ids`] are period-keyed (they walk the ordering sidecar under
    /// the period display order). On a valid state the two conventions agree, but
    /// on a broken (dangling) state they disagree — an orphan week is counted
    /// here yet never walked. Never mix a `count_weeks`-derived total with a
    /// `walk_weeks`-derived index outside a validated state.
    pub fn count_weeks(&self) -> usize {
        self.weeks.count_weeks()
    }
}

impl Parameters {
    /// Promotes an u64 to a [PeriodId] if it is valid
    pub fn validate_period_id(&self, id: u64) -> Option<PeriodId> {
        for period_id in self.periods.period_ids() {
            if period_id.inner() == id {
                return Some(period_id);
            }
        }

        None
    }

    /// Promotes an u64 to a [StudentId] if it is valid
    pub fn validate_student_id(&self, id: u64) -> Option<StudentId> {
        let student_id = unsafe { StudentId::new(id) };

        if !self.students.student_map.contains(&student_id) {
            return None;
        }

        Some(student_id)
    }

    /// Promotes an u64 to a [SubjectId] if it is valid
    pub fn validate_subject_id(&self, id: u64) -> Option<SubjectId> {
        for (subject_id, _) in self.subjects.ordered_subject_list.iter() {
            if subject_id.inner() == id {
                return Some(subject_id);
            }
        }

        None
    }

    /// Promotes an u64 to a [TeacherId] if it is valid
    pub fn validate_teacher_id(&self, id: u64) -> Option<TeacherId> {
        let temp_teacher_id = unsafe { TeacherId::new(id) };
        if self.teachers.teacher_map.contains(&temp_teacher_id) {
            return Some(temp_teacher_id);
        }

        None
    }

    /// Promotes an u64 to a [WeekPatternId] if it is valid
    pub fn validate_week_pattern_id(&self, id: u64) -> Option<WeekPatternId> {
        let temp_week_pattern_id = unsafe { WeekPatternId::new(id) };
        if self
            .week_patterns
            .week_pattern_map
            .contains(&temp_week_pattern_id)
        {
            return Some(temp_week_pattern_id);
        }

        None
    }

    /// Promotes an u64 to a [SlotId] if it is valid
    pub fn validate_slot_id(&self, id: u64) -> Option<SlotId> {
        let slot_id = unsafe { SlotId::new(id) };
        if self.slots.find_slot(slot_id).is_some() {
            Some(slot_id)
        } else {
            None
        }
    }

    /// Promotes an u64 to a [IncompatId] if it is valid
    pub fn validate_incompat_id(&self, id: u64) -> Option<IncompatId> {
        let temp_incompat_id = unsafe { IncompatId::new(id) };
        if self.incompats.incompat_map.contains(&temp_incompat_id) {
            return Some(temp_incompat_id);
        }

        None
    }

    /// Promotes an u64 to a [GroupListId] if it is valid
    pub fn validate_group_list_id(&self, id: u64) -> Option<GroupListId> {
        let temp_group_list_id = unsafe { GroupListId::new(id) };
        if self
            .group_lists
            .group_list_map
            .contains(&temp_group_list_id)
        {
            return Some(temp_group_list_id);
        }

        None
    }
}

// --- Keyed read interface (SQL-like lookup) ---
//
// One [`Lookup`] impl per entity kind, keyed on the matching typed id and
// resolving to the entity type declared in that id's `#[entity(…)]` attribute
// (`ids.rs`). Each delegates to the container accessor already used elsewhere
// in this module, so lookup borrows straight out of the table — no clone, no
// rebuild. These are the context impls the `Join` derives resolve against.

impl Lookup<PeriodId> for Parameters {
    type Entity = ();
    fn lookup(&self, id: PeriodId) -> Option<&()> {
        self.periods.ordered_period_list.get(&id)
    }
}

impl Lookup<WeekId> for Parameters {
    type Entity = weeks::Week;
    fn lookup(&self, id: WeekId) -> Option<&weeks::Week> {
        self.weeks.find_week(id)
    }
}

impl Lookup<SubjectId> for Parameters {
    type Entity = subjects::Subject;
    fn lookup(&self, id: SubjectId) -> Option<&subjects::Subject> {
        self.subjects.find_subject(id)
    }
}

impl Lookup<TeacherId> for Parameters {
    type Entity = teachers::Teacher;
    fn lookup(&self, id: TeacherId) -> Option<&teachers::Teacher> {
        self.teachers.teacher_map.get(&id)
    }
}

impl Lookup<StudentId> for Parameters {
    type Entity = students::Student;
    fn lookup(&self, id: StudentId) -> Option<&students::Student> {
        self.students.student_map.get(&id)
    }
}

impl Lookup<WeekPatternId> for Parameters {
    type Entity = week_patterns::WeekPattern;
    fn lookup(&self, id: WeekPatternId) -> Option<&week_patterns::WeekPattern> {
        self.week_patterns.week_pattern_map.get(&id)
    }
}

impl Lookup<SlotId> for Parameters {
    type Entity = slots::Slot;
    fn lookup(&self, id: SlotId) -> Option<&slots::Slot> {
        self.slots.find_slot(id)
    }
}

impl Lookup<IncompatId> for Parameters {
    type Entity = incompats::Incompatibility;
    fn lookup(&self, id: IncompatId) -> Option<&incompats::Incompatibility> {
        self.incompats.incompat_map.get(&id)
    }
}

impl Lookup<GroupListId> for Parameters {
    type Entity = group_lists::GroupList;
    fn lookup(&self, id: GroupListId) -> Option<&group_lists::GroupList> {
        self.group_lists.group_list_map.get(&id)
    }
}

impl Lookup<PairingRuleId> for Parameters {
    type Entity = pairings::PairingRule;
    fn lookup(&self, id: PairingRuleId) -> Option<&pairings::PairingRule> {
        self.pairings.pairing_rule_map.get(&id)
    }
}

impl Lookup<SlotPairingRuleId> for Parameters {
    type Entity = slot_pairings::SlotPairingRule;
    fn lookup(&self, id: SlotPairingRuleId) -> Option<&slot_pairings::SlotPairingRule> {
        self.slot_pairings.slot_pairing_rule_map.get(&id)
    }
}

impl Parameters {
    /// Typed keyed lookup — the fallible entry point.
    ///
    /// Resolves any typed id against its table, returning `None` when the id
    /// dangles. Use this for candidate/unvalidated data where a missing target
    /// is a legitimate outcome. The concrete entity type is inferred from the
    /// id kind through the [`Lookup`] impls above.
    pub fn lookup<I>(&self, id: I) -> Option<&<Self as Lookup<I>>::Entity>
    where
        Self: Lookup<I>,
    {
        <Self as Lookup<I>>::lookup(self, id)
    }

    /// Infallible resolution for already-validated data.
    ///
    /// The invariant checks guarantee no reference dangles once a document is
    /// committed, so on that data every id resolves. This variant unwraps that
    /// guarantee and **panics** (printing the offending id) if it is ever
    /// violated — a dangling id here is a bug, not an expected input.
    pub fn resolve<I: Id>(&self, id: I) -> &<Self as Lookup<I>>::Entity
    where
        Self: Lookup<I>,
    {
        <Self as Lookup<I>>::lookup(self, id)
            .unwrap_or_else(|| panic!("dangling {id:?} in validated data"))
    }
}

impl Parameters {
    /// Every primary-key id in the document, typed as [`NewId`], in the
    /// canonical table order.
    ///
    /// This is the single declared enumeration of the ten entity tables. The
    /// order — students, periods, subjects, teachers, week patterns, slots,
    /// incompats, group lists, pairing rules, slot pairing rules — is kept
    /// identical to the historical [`Parameters::ids`] chain, which now defers
    /// to this method.
    pub fn all_ids(&self) -> impl Iterator<Item = NewId> + '_ {
        self.students
            .student_map
            .keys()
            .map(NewId::from)
            .chain(self.periods.period_ids().map(NewId::from))
            .chain(self.week_ids().map(NewId::from))
            .chain(self.subjects.ordered_subject_list.keys().map(NewId::from))
            .chain(self.teachers.teacher_map.keys().map(NewId::from))
            .chain(self.week_patterns.week_pattern_map.keys().map(NewId::from))
            .chain(self.slots.slot_ids().map(NewId::from))
            .chain(self.incompats.incompat_map.keys().map(NewId::from))
            .chain(self.group_lists.group_list_map.keys().map(NewId::from))
            .chain(self.pairings.pairing_rule_map.keys().map(NewId::from))
            .chain(
                self.slot_pairings
                    .slot_pairing_rule_map
                    .keys()
                    .map(NewId::from),
            )
    }

    /// USED INTERNALLY
    ///
    /// Returns an iterator on all ids that appear in the colloscope params, as
    /// raw `u64`. A thin numeric adapter over [`Parameters::all_ids`].
    pub(crate) fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.all_ids().map(|id| id.inner())
    }

    /// Promotes an u64 to a [PairingRuleId] if it is valid
    pub fn validate_pairing_rule_id(&self, id: u64) -> Option<PairingRuleId> {
        let temp_id = unsafe { PairingRuleId::new(id) };
        if self.pairings.pairing_rule_map.contains(&temp_id) {
            return Some(temp_id);
        }

        None
    }

    /// Promotes an u64 to a [SlotPairingRuleId] if it is valid
    pub fn validate_slot_pairing_rule_id(&self, id: u64) -> Option<SlotPairingRuleId> {
        let id = unsafe { SlotPairingRuleId::new(id) };
        if self.slot_pairings.slot_pairing_rule_map.contains(&id) {
            Some(id)
        } else {
            None
        }
    }
}
