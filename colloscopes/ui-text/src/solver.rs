//! The words the application speaks about a solve.
//!
//! One French sentence per [`collomatique_strategies::ConductorWarning`]
//! variant, phrased the way the application's own solve dialog shows it
//! (`colloscopes/gtk4/src/editor/run_solver/conductor_config.rs`), kept here so the python
//! module's warnings say what the dialog says. The match is exhaustive with no
//! wildcard arm, so a new warning over there is a compile error here — the
//! [`crate::caveats::caveat_text`] shape.
//!
//! [`solve_verdict_text`] does the same for the one sentence that same dialog
//! writes once a solve has finished, and [`fixed_pin_violation_text`] for the
//! one piece of blame that comes from the solve configuration rather than from
//! the model.

use collomatique_constraints_colloscopes::Var;
use collomatique_constraints_colloscopes::ids::GroupNum;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::ids::{GroupListId, SlotId, StudentId};
use collomatique_strategies::{ConductorWarning, SolveVerdict};

/// The sentence a conductor warning is shown as
///
/// Flat, one line, no bullet: the dialog's list layout is its own, so a script
/// printing a warning gets a single sentence.
pub fn conductor_warning_text(warning: ConductorWarning) -> &'static str {
    match warning {
        ConductorWarning::NoStrategyEnabled => {
            "Aucune stratégie n'est activée : rien ne sera exécuté."
        }
        ConductorWarning::NoOptimizing => {
            "Aucune stratégie d'optimisation n'est activée : le solveur cherchera une solution \
             réalisable sans tenter de l'améliorer."
        }
        ConductorWarning::NoSeed => {
            "L'exploration aléatoire est activée mais aucune stratégie ne produit de solution \
             initiale (démarrage à chaud ou résolution incrémentale) : elle ne démarrera jamais et \
             le solveur s'arrêtera immédiatement."
        }
        ConductorWarning::StarvedFuzzy => {
            "L'exploration aléatoire est activée mais l'unique tâche est occupée par la stratégie \
             par défaut : elle n'aura jamais de créneau libre. Augmentez le nombre de tâches en \
             parallèle."
        }
        ConductorWarning::WontFinish => {
            "La stratégie par défaut est désactivée : sans elle, aucune borne ne prouve \
             l'optimalité et le solveur tournera indéfiniment."
        }
        ConductorWarning::ColdFuzzy => {
            "L'exploration aléatoire est activée sans solution initiale (démarrage à chaud ou \
             résolution incrémentale) : elle ne se déclenchera qu'une fois la stratégie par défaut \
             bien avancée et sera donc souvent inutile."
        }
        ConductorWarning::RedundantWarmStart => {
            "Le démarrage à chaud et la résolution incrémentale sont tous deux activés : la \
             résolution incrémentale fournit généralement un meilleur point de départ ; le \
             démarrage à chaud n'est utile que pour obtenir rapidement une solution."
        }
        ConductorWarning::OverwhelmedCpu => {
            "Le nombre de tâches en parallèle dépasse le nombre de cœurs du processeur."
        }
    }
}

/// The sentence the application shows for a finished solve
///
/// The solve dialog's own words (`colloscopes/gtk4/src/editor/run_solver.rs`), down to the
/// punctuation, so a script printing a status prints what the user would have
/// read.
pub fn solve_verdict_text(verdict: SolveVerdict) -> &'static str {
    match verdict {
        SolveVerdict::Optimal => "Solution optimale trouvée !",
        SolveVerdict::Feasible => "Solution trouvée !",
        SolveVerdict::NoSolution => "Pas de solution !",
        SolveVerdict::Error => "Erreur pendant l'exécution",
    }
}

/// The sentence a violated solve-configuration pin is shown as
///
/// A [`ConfiguredConstraintDesc::Fixed`] row holds one variable at one value:
/// the solve configuration said "do not recompute this", and the colloscope
/// being checked says otherwise. It is the only piece of blame the base model
/// knows nothing about, so its wording lives here rather than next to the other
/// constraint sentences.
///
/// `value` is the pinned value — above 0.5 the variable was pinned *present*,
/// below it, pinned absent.
///
/// It speaks the blame dialog's vocabulary, not this crate's:
/// a slot reads « Potions (lundi 14h00) » and a group « 3 (Gryffondor) », the
/// shapes `ConstraintDesc::user_readable` uses, because a script or a dialog
/// shows this sentence in the same list as those. That is also why the lookups
/// are written out here instead of going through [`crate::rendering`], whose
/// formats differ and whose misses are an `Err` — this is a solver diagnostic
/// and must stay printable, so an id that resolves to nothing degrades to its
/// `Debug` form (the policy split is documented in [`crate::rendering`]).
///
/// [`ConfiguredConstraintDesc::Fixed`]: collomatique_constraints_colloscopes::ConfiguredConstraintDesc::Fixed
pub fn fixed_pin_violation_text(var: &Var, value: f64, params: &Parameters) -> String {
    let pinned_present = value > 0.5;

    match var {
        Var::GroupInInterrogation { slot, week, group } => {
            let verb = if pinned_present {
                "passe en colle"
            } else {
                "ne passe pas en colle"
            };
            format!(
                "La configuration de résolution imposait que le groupe {} {} sur le créneau {} la \
                 semaine {}, mais le colloscope ne le respecte pas.",
                group_text(params, group),
                verb,
                slot_text(params, *slot),
                week.0 + 1,
            )
        }
        Var::StudentInGroup {
            group_list,
            student,
            group,
        } => {
            let verb = if pinned_present {
                "soit dans"
            } else {
                "ne soit pas dans"
            };
            format!(
                "La configuration de résolution imposait que l'élève {} {} le groupe {} de la \
                 liste {}, mais le colloscope ne le respecte pas.",
                student_text(params, *student),
                verb,
                group_text(params, group),
                group_list_text(params, *group_list),
            )
        }
    }
}

/// « Potions (lundi 14h00) », the subject then the slot's time.
fn slot_text(params: &Parameters, slot: SlotId) -> String {
    let slot_data = params.slots.find_slot(slot);
    let subject = params
        .slots
        .find_slot_subject_and_position(slot)
        .and_then(|(subject_id, _pos)| params.subjects.find_subject(subject_id))
        .map(|s| s.parameters.name.as_str());
    match (subject, slot_data) {
        (Some(subject), Some(data)) => format!("{} ({})", subject, data.start_time),
        (Some(subject), None) => subject.to_string(),
        _ => format!("{:?}", slot),
    }
}

/// « 3 (Gryffondor) » for a named group, « 3 » for an unnamed one.
fn group_text(params: &Parameters, group: &GroupNum) -> String {
    let number = group.index() + 1;
    let name = params
        .group_lists
        .group_list_map
        .get(&group.group_list())
        .and_then(|gl| gl.params().group_names.get(group.index()))
        .and_then(|name| name.as_ref());
    match name {
        Some(name) => format!("{} ({})", number, name),
        None => format!("{}", number),
    }
}

/// « Harry Potter ».
fn student_text(params: &Parameters, student: StudentId) -> String {
    params
        .students
        .student_map
        .get(&student)
        .map(|s| format!("{} {}", s.desc.firstname, s.desc.surname))
        .unwrap_or_else(|| format!("{:?}", student))
}

/// The group list's bare name.
fn group_list_text(params: &Parameters, group_list: GroupListId) -> String {
    params
        .group_lists
        .group_list_map
        .get(&group_list)
        .map(|gl| gl.params().name.clone())
        .unwrap_or_else(|| format!("{:?}", group_list))
}

#[cfg(test)]
mod tests;
