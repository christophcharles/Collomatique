//! What a [crate::CascadeWarning] says to the user.
//!
//! One French sentence per [Fix] variant, phrased as the **effect** — « Le
//! créneau X sera supprimé », never « … car son colleur a été supprimé ». The
//! user has just performed the action, so they know the cause; what they cannot
//! know is what else it takes with it.
//!
//! [render] is exhaustive over [Fix] with **no wildcard arm**, which is the
//! whole point of the vocabulary being a closed enum: a new repair shape is a
//! compile error here. And because a [Fix] variant is one *rendered meaning*
//! (and not one invariant, nor one elementary op), the renderer never has to ask
//! what broke — the same op can arrive under two variants precisely when the two
//! sentences differ ([Fix::DeleteSlot] and [Fix::DeleteOverflowingSlot]).
//!
//! Entities are named through [`collomatique_ui_text::rendering`], the
//! vocabulary gtk4 and the python module share, so a warning and the window
//! behind it describe the same thing with the same words. The renderer owns the
//! noun, the article and the agreement; the renderers are noun-less on purpose.
//!
//! # Conventions
//!
//! - « » around free-form names that cannot be read without them (group lists,
//!   week patterns, incompatibility names, both rule notations); bare for
//!   proper-noun-like names (subjects, people) and for numbers.
//! - A period joins a sentence as « sur la période {…} », never as a
//!   parenthetical: [`collomatique_ui_text::rendering::render_period`]'s own
//!   output carries parentheses and nesting them reads badly.
//! - A coordinate that *every* producer of a variant implies is dropped, but
//!   only from the complement position — the grammatical subject always stays.
//!   The four period-exclusion sentences name no period because only the death
//!   of a period produces them, and the user knows which one they just deleted.
//!
//! # Failure
//!
//! A miss is not an ordinary outcome: a warning names material that is present
//! in the document it is rendered against — the frame rule's rendering corollary
//! (see [crate::cascade]). `Err(MissingId)` is the instrument that surfaces a
//! violation of it, and the callers that must not fail panic on it.

use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::{Fix, GroupListId, PeriodId, SlotId, SubjectId, WeekId};

use collomatique_ui_text::rendering::{
    MissingId, join_french, render_group, render_group_list, render_incompat, render_pairing_rule,
    render_period, render_slot, render_slot_pairing_rule, render_student, render_subject,
    render_teacher, render_week, render_week_pattern,
};

/// The French, effect-phrased description of one repair, read against `params`.
pub(crate) fn render(params: &Parameters, fix: &Fix) -> Result<String, MissingId> {
    Ok(match fix {
        Fix::DeleteWeek { week } => {
            format!(
                "La semaine {} sera supprimée",
                render_week(&params.periods, &params.weeks, *week)?
            )
        }
        Fix::RemoveSubjectPeriodExclusion { subject, .. } => {
            format!(
                "{} : l'exclusion de période sera levée",
                render_subject(&params.subjects, *subject)?
            )
        }
        Fix::RemoveStudentPeriodExclusion { student, .. } => {
            format!(
                "{} : l'exclusion de période sera levée",
                render_student(&params.students, *student)?
            )
        }
        Fix::RemovePairingRulePeriodExclusion { rule, .. } => {
            format!(
                "Règle « {} » : l'exclusion de période sera levée",
                render_pairing_rule(&params.subjects, &params.pairings, *rule)?
            )
        }
        Fix::RemoveSlotPairingRulePeriodExclusion { rule, .. } => {
            format!(
                "Règle de créneaux « {} » : l'exclusion de période sera levée",
                render_slot_pairing_rule(
                    &params.subjects,
                    &params.teachers,
                    &params.slots,
                    &params.slot_pairings,
                    *rule
                )?
            )
        }
        Fix::ClearAssignmentRow { period, subject } => {
            format!(
                "Les inscriptions en {} sur la période {} seront supprimées",
                render_subject(&params.subjects, *subject)?,
                render_period(&params.periods, &params.weeks, *period)?,
            )
        }
        Fix::UnassignGroupList { period, subject } => {
            // The list is not in the fix — it is what the entry the fix clears
            // holds, and that entry is present in the pre-state by definition.
            let group_list = association(params, *period, *subject)?;
            format!(
                "L'association de la liste « {} » en {} sur la période {} sera supprimée",
                render_group_list(&params.group_lists, group_list)?,
                render_subject(&params.subjects, *subject)?,
                render_period(&params.periods, &params.weeks, *period)?,
            )
        }
        Fix::RemoveWeekPatternExclusion { pattern, week, .. } => {
            format!(
                "Motif « {} » : l'exclusion de la semaine {} sera levée",
                render_week_pattern(&params.week_patterns, *pattern)?,
                render_week(&params.periods, &params.weeks, *week)?,
            )
        }
        Fix::ClearInterrogationCell { slot, week } => {
            format!(
                "La colle du créneau {} en semaine {} sera supprimée",
                render_slot(&params.subjects, &params.teachers, &params.slots, *slot)?,
                render_week(&params.periods, &params.weeks, *week)?,
            )
        }
        Fix::RemoveTeacherSubject {
            teacher, subject, ..
        } => {
            format!(
                "{} n'interviendra plus en {}",
                render_teacher(&params.teachers, *teacher)?,
                render_subject(&params.subjects, *subject)?,
            )
        }
        Fix::DeleteSlot { slot } => {
            format!(
                "Le créneau {} sera supprimé",
                render_slot(&params.subjects, &params.teachers, &params.slots, *slot)?
            )
        }
        Fix::DeleteOverflowingSlot { slot } => {
            format!(
                "Le créneau {} sera supprimé (il déborderait sur le jour suivant)",
                render_slot(&params.subjects, &params.teachers, &params.slots, *slot)?
            )
        }
        Fix::DeleteIncompat { incompat } => {
            format!(
                "L'incompatibilité « {} » sera supprimée",
                render_incompat(&params.incompats, *incompat)?
            )
        }
        Fix::DeletePairingRule { rule } => {
            format!(
                "La règle « {} » sera supprimée",
                render_pairing_rule(&params.subjects, &params.pairings, *rule)?
            )
        }
        Fix::ClearSubjectBalancing { subject } => {
            format!(
                "L'équilibrage de {} prendra les valeurs par défaut",
                render_subject(&params.subjects, *subject)?
            )
        }
        Fix::RemoveStudentFromGroupListPrefill {
            group_list,
            student,
            ..
        } => {
            format!(
                "{} sera retiré(e) des groupes préremplis de « {} »",
                render_student(&params.students, *student)?,
                render_group_list(&params.group_lists, *group_list)?,
            )
        }
        Fix::RemoveStudentGroupListExclusion {
            group_list,
            student,
            ..
        } => {
            format!(
                "{} : l'exclusion de la liste « {} » sera levée",
                render_student(&params.students, *student)?,
                render_group_list(&params.group_lists, *group_list)?,
            )
        }
        Fix::ClearStudentSettings { student } => {
            format!(
                "Les paramètres de {} prendront les valeurs par défaut",
                render_student(&params.students, *student)?
            )
        }
        Fix::RemoveStudentFromAssignmentRow {
            period,
            subject,
            student,
            ..
        } => {
            format!(
                "L'inscription de {} en {} sur la période {} sera supprimée",
                render_student(&params.students, *student)?,
                render_subject(&params.subjects, *subject)?,
                render_period(&params.periods, &params.weeks, *period)?,
            )
        }
        Fix::RemoveStudentColloscopePlacement {
            group_list,
            student,
            ..
        } => {
            format!(
                "{} sera retiré(e) de son groupe de « {} » dans le colloscope",
                render_student(&params.students, *student)?,
                render_group_list(&params.group_lists, *group_list)?,
            )
        }
        Fix::ClearSlotWeekPattern { slot, .. } => {
            format!(
                "Le créneau {} ne suivra plus de motif : il aura lieu toutes les semaines",
                render_slot(&params.subjects, &params.teachers, &params.slots, *slot)?
            )
        }
        Fix::ClearIncompatWeekPattern { incompat, .. } => {
            format!(
                "L'incompatibilité « {} » ne suivra plus de motif : elle s'appliquera toutes les semaines",
                render_incompat(&params.incompats, *incompat)?
            )
        }
        Fix::DeleteSlotPairingRule { rule } => {
            format!(
                "La règle de créneaux « {} » sera supprimée",
                render_slot_pairing_rule(
                    &params.subjects,
                    &params.teachers,
                    &params.slots,
                    &params.slot_pairings,
                    *rule
                )?
            )
        }
        Fix::ClearColloscopeGroupListRow { group_list } => {
            format!(
                "La répartition en groupes de « {} » dans le colloscope sera supprimée",
                render_group_list(&params.group_lists, *group_list)?
            )
        }
        Fix::RemoveGroupsFromInterrogationCell {
            slot, week, groups, ..
        } => {
            let group_list = cell_group_list(params, *slot, *week)?;
            let names = groups
                .iter()
                .map(|group| render_group(&params.group_lists, group_list, *group))
                .collect::<Result<Vec<_>, _>>()?;
            let slot_text = render_slot(&params.subjects, &params.teachers, &params.slots, *slot)?;
            let week_text = render_week(&params.periods, &params.weeks, *week)?;
            if names.len() > 1 {
                format!(
                    "Les groupes {} seront retirés de la colle du créneau {} en semaine {}",
                    join_french(&names),
                    slot_text,
                    week_text,
                )
            } else {
                format!(
                    "Le groupe {} sera retiré de la colle du créneau {} en semaine {}",
                    join_french(&names),
                    slot_text,
                    week_text,
                )
            }
        }
    })
}

/// The group list a subject uses on a period.
fn association(
    params: &Parameters,
    period: PeriodId,
    subject: SubjectId,
) -> Result<GroupListId, MissingId> {
    params
        .group_lists
        .subjects_associations
        .get(&(period, subject))
        .copied()
        .ok_or(MissingId::Association { period, subject })
}

/// The group list an interrogation cell's group numbers are numbered against:
/// the one associated at the cell's `(period, subject)` coordinate, which the
/// cell names through its week and its slot.
fn cell_group_list(
    params: &Parameters,
    slot: SlotId,
    week: WeekId,
) -> Result<GroupListId, MissingId> {
    let (subject, _slot) = params.slots.find_slot_with_subject(slot).ok_or(slot)?;
    let (period, _position) = params.weeks.week_position(week).ok_or(week)?;

    association(params, period, subject)
}
