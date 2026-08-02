//! How `ops/` names a document entity to a user.
//!
//! One function per id kind, each taking the [Data] the id is to be read
//! against and doing its own lookups. Every renderer is **noun-less**: it
//! answers « 5 (du 28/09/2026 au 04/10/2026) », never « Semaine 5 … ». The
//! caller owns the sentence, and so owns the noun, the article and the
//! agreement — « La semaine {} sera supprimée » and « les semaines {} et {} »
//! are the same rendered week in two different sentences.
//!
//! # Why `Result` and not `Option`
//!
//! Lookups chain: a slot resolves its subject and its teacher, a slot pairing
//! rule resolves its two slots and through them a subject. If a renderer
//! answered `None`, the caller could only report the id *it* was holding, which
//! is rarely the thing that is actually missing. [MissingId] names the material
//! the lookup could not find, however deep it sat.
//!
//! A miss is not an ordinary outcome here: the entity a warning names is
//! present in the document the warning is rendered against — that is the frame
//! rule's rendering corollary (see [crate::cascade]). The `Err` is the
//! instrument that surfaces a violation of it, which is why the callers that
//! must not fail (gtk4, the property walk) panic on it rather than degrade.
//! The solver's own renderer
//! (`collomatique_constraints_colloscopes::types::user_readable`) makes the
//! opposite choice on purpose — a diagnostic must stay printable, so it
//! degrades to a `{:?}` fallback. The formats below are shared with it and with
//! gtk4; the miss policy is not.

use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::{
    Data, GroupListId, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekId, WeekPatternId,
};

use thiserror::Error;

/// What a lookup could not find while rendering.
///
/// One variant per id kind, plus the two structural lookups that are not a bare
/// id: an association entry is keyed by a *pair* (neither id alone is the
/// missing thing), and a group is an index into a list that does exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum MissingId {
    #[error("Period id ({0:?}) is invalid")]
    Period(PeriodId),
    #[error("Week id ({0:?}) is invalid")]
    Week(WeekId),
    #[error("Subject id ({0:?}) is invalid")]
    Subject(SubjectId),
    #[error("Teacher id ({0:?}) is invalid")]
    Teacher(TeacherId),
    #[error("Student id ({0:?}) is invalid")]
    Student(StudentId),
    #[error("Week pattern id ({0:?}) is invalid")]
    WeekPattern(WeekPatternId),
    #[error("Slot id ({0:?}) is invalid")]
    Slot(SlotId),
    #[error("Group list id ({0:?}) is invalid")]
    GroupList(GroupListId),
    #[error("Incompatibility id ({0:?}) is invalid")]
    Incompat(IncompatId),
    #[error("Pairing rule id ({0:?}) is invalid")]
    PairingRule(PairingRuleId),
    #[error("Slot pairing rule id ({0:?}) is invalid")]
    SlotPairingRule(SlotPairingRuleId),
    /// The association entry at `(period, subject)` — what names the group list
    /// a subject uses on a period.
    #[error("No group list is associated to subject ({subject:?}) on period ({period:?})")]
    Association {
        period: PeriodId,
        subject: SubjectId,
    },
    /// A group index out of bounds. The list itself was found: this is not a
    /// missing list.
    #[error("Group list ({group_list:?}) has no group at index {index}")]
    Group { group_list: GroupListId, index: u32 },
}

// The `From` impls are what lets a renderer write `.ok_or(id)?`: the lookup
// names the id it failed on and `?` lifts it into the shared error.
macro_rules! missing_id_from {
    ($($id:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$id> for MissingId {
                fn from(id: $id) -> MissingId {
                    MissingId::$variant(id)
                }
            }
        )*
    };
}

missing_id_from! {
    PeriodId => Period,
    WeekId => Week,
    SubjectId => Subject,
    TeacherId => Teacher,
    StudentId => Student,
    WeekPatternId => WeekPattern,
    SlotId => Slot,
    GroupListId => GroupList,
    IncompatId => Incompat,
    PairingRuleId => PairingRule,
    SlotPairingRuleId => SlotPairingRule,
}

impl From<(PeriodId, SubjectId)> for MissingId {
    fn from((period, subject): (PeriodId, SubjectId)) -> MissingId {
        MissingId::Association { period, subject }
    }
}

fn params(data: &Data) -> &Parameters {
    &data.get_inner_data().params
}

/// A week, as its global 1-based number, with its dates when the document has a
/// start date: « 5 (du 28/09/2026 au 04/10/2026) », or bare « 5 ».
pub fn render_week(data: &Data, id: WeekId) -> Result<String, MissingId> {
    let params = params(data);
    let week_num = params
        .weeks
        .global_week_position(&params.periods, id)
        .ok_or(id)?;

    Ok(match &params.periods.first_week {
        Some(global_start_date) => {
            let start_date = global_start_date
                .monday()
                .checked_add_days(chrono::Days::new(7 * (week_num as u64)))
                .expect("Valid start date");
            let end_date = start_date
                .checked_add_days(chrono::Days::new(6))
                .expect("Valid end date");
            format!(
                "{} (du {} au {})",
                week_num + 1,
                start_date.format("%d/%m/%Y"),
                end_date.format("%d/%m/%Y"),
            )
        }
        None => format!("{}", week_num + 1),
    })
}

/// A period, as its 1-based number followed by what it spans:
/// « 1 (du 31/08/2026 au 27/09/2026 - semaines 1 à 4) », « 3 (semaine 12) »,
/// « 2 (vide) ».
pub fn render_period(data: &Data, id: PeriodId) -> Result<String, MissingId> {
    let params = params(data);
    let (index, first_week_num) = params
        .weeks
        .find_period_position_and_first_week(&params.periods, id)
        .ok_or(id)?;
    // A period with no ordering row has no weeks — the same thing an empty row
    // means.
    let week_count = params.weeks.week_count_for_period(id).unwrap_or(0);

    if week_count == 0 {
        return Ok(format!("{} (vide)", index + 1));
    }

    let start_week = first_week_num + 1;
    let end_week = first_week_num + week_count;
    let weeks_text = if start_week != end_week {
        format!("semaines {} à {}", start_week, end_week)
    } else {
        format!("semaine {}", start_week)
    };

    Ok(match &params.periods.first_week {
        Some(global_start_date) => {
            let start_date = global_start_date
                .monday()
                .checked_add_days(chrono::Days::new(7 * (first_week_num as u64)))
                .expect("Valid start date");
            let end_date = start_date
                .checked_add_days(chrono::Days::new(7 * (week_count as u64) - 1))
                .expect("Valid end date");
            format!(
                "{} (du {} au {} - {})",
                index + 1,
                start_date.format("%d/%m/%Y"),
                end_date.format("%d/%m/%Y"),
                weeks_text,
            )
        }
        None => format!("{} ({})", index + 1, weeks_text),
    })
}

/// A subject, as its bare name.
pub fn render_subject(data: &Data, id: SubjectId) -> Result<String, MissingId> {
    let subject = params(data).subjects.find_subject(id).ok_or(id)?;
    Ok(subject.parameters.name.clone())
}

/// A teacher, as « {firstname} {surname} ».
pub fn render_teacher(data: &Data, id: TeacherId) -> Result<String, MissingId> {
    let teacher = params(data).teachers.teacher_map.get(&id).ok_or(id)?;
    Ok(format!(
        "{} {}",
        teacher.desc.firstname, teacher.desc.surname
    ))
}

/// A student, as « {firstname} {surname} ».
pub fn render_student(data: &Data, id: StudentId) -> Result<String, MissingId> {
    let student = params(data).students.student_map.get(&id).ok_or(id)?;
    Ok(format!(
        "{} {}",
        student.desc.firstname, student.desc.surname
    ))
}

/// A week pattern, as its bare name.
pub fn render_week_pattern(data: &Data, id: WeekPatternId) -> Result<String, MissingId> {
    let pattern = params(data)
        .week_patterns
        .week_pattern_map
        .get(&id)
        .ok_or(id)?;
    Ok(pattern.name.clone())
}

/// A group list, as its bare name.
pub fn render_group_list(data: &Data, id: GroupListId) -> Result<String, MissingId> {
    let group_list = params(data).group_lists.group_list_map.get(&id).ok_or(id)?;
    Ok(group_list.params().name.clone())
}

/// An incompatibility, as its bare name.
pub fn render_incompat(data: &Data, id: IncompatId) -> Result<String, MissingId> {
    let incompat = params(data).incompats.incompat_map.get(&id).ok_or(id)?;
    Ok(incompat.name.clone())
}

/// A slot: « Séverus Rogue - lundi 14h00 (Physique) ».
///
/// The teacher comes first because that is how a user *finds* a slot; the
/// subject trails in parentheses because it is context rather than the slot
/// itself. All three parts are needed — a teacher may hold several slots at the
/// same hour in different subjects.
pub fn render_slot(data: &Data, id: SlotId) -> Result<String, MissingId> {
    let (subject_id, _slot) = params(data).slots.find_slot_with_subject(id).ok_or(id)?;
    let slot_text = render_slot_in_subject(data, id)?;
    let subject = render_subject(data, subject_id)?;
    Ok(format!("{} ({})", slot_text, subject))
}

/// A slot with its subject left out: « Séverus Rogue - lundi 14h00 ».
///
/// For anywhere the subject is already established and repeating it would be
/// noise: the slot pairing rule notation names it once and then uses this form
/// twice, and gtk4's slot pairings tab is grouped by subject throughout (its
/// `build_slot_description` is this exact format).
pub fn render_slot_in_subject(data: &Data, id: SlotId) -> Result<String, MissingId> {
    let slot = params(data).slots.find_slot(id).ok_or(id)?;
    let teacher = render_teacher(data, slot.teacher_id)?;
    Ok(format!("{} - {}", teacher, slot.start_time))
}

/// A pairing rule, in the notation its own tab uses:
/// « Avoir Physique ⟹ Ne pas avoir Chimie ».
///
/// Softness is deliberately left out: it is a property of the rule, not part of
/// what identifies it. The list views append « (souple) » themselves.
pub fn render_pairing_rule(data: &Data, id: PairingRuleId) -> Result<String, MissingId> {
    let rule = params(data).pairings.pairing_rule_map.get(&id).ok_or(id)?;
    let ant_name = render_subject(data, rule.antecedent().subject_id)?;
    let con_name = render_subject(data, rule.consequent().subject_id)?;
    Ok(format!(
        "{} {} \u{27F9} {} {}",
        have_condition(rule.antecedent().should_have),
        ant_name,
        have_condition(rule.consequent().should_have),
        con_name,
    ))
}

fn have_condition(should_have: bool) -> &'static str {
    if should_have { "Avoir" } else { "Ne pas avoir" }
}

/// A slot pairing rule: « Physique : [utilisé] Séverus Rogue - lundi 14h00 ⟹
/// [non utilisé] Minerve McGonagall - mardi 15h00 ».
///
/// The subject is fronted once — the rule's own tab groups rules by subject and
/// so never repeats it, but a warning arrives without that context. Both slots
/// belong to that same subject (a rule pairing slots of two subjects is not a
/// legal document), so the antecedent's answers for both, and each part is
/// rendered without its own subject parentheses.
pub fn render_slot_pairing_rule(data: &Data, id: SlotPairingRuleId) -> Result<String, MissingId> {
    let params = params(data);
    let rule = params
        .slot_pairings
        .slot_pairing_rule_map
        .get(&id)
        .ok_or(id)?;
    let ant_slot = rule.antecedent().slot_id;
    let con_slot = rule.consequent().slot_id;

    let (subject_id, _slot) = params
        .slots
        .find_slot_with_subject(ant_slot)
        .ok_or(ant_slot)?;
    let subject = render_subject(data, subject_id)?;

    Ok(format!(
        "{} : [{}] {} \u{27F9} [{}] {}",
        subject,
        use_condition(rule.antecedent().should_have),
        render_slot_in_subject(data, ant_slot)?,
        use_condition(rule.consequent().should_have),
        render_slot_in_subject(data, con_slot)?,
    ))
}

fn use_condition(should_have: bool) -> &'static str {
    if should_have {
        "utilisé"
    } else {
        "non utilisé"
    }
}

/// The name of a group of a group list, if it has one. `Ok(None)` means the
/// group exists and is unnamed — only `Err` means it does not exist.
pub fn render_group_name(
    data: &Data,
    group_list: GroupListId,
    index: u32,
) -> Result<Option<String>, MissingId> {
    let list = params(data)
        .group_lists
        .group_list_map
        .get(&group_list)
        .ok_or(group_list)?;
    let name = list
        .params()
        .group_names
        .get(index as usize)
        .ok_or(MissingId::Group { group_list, index })?;
    Ok(name.clone().map(|n| n.into_inner()))
}

/// A group of a group list: its name if it has one, otherwise its 1-based
/// number — « B2 » or « 4 », never both.
pub fn render_group(data: &Data, group_list: GroupListId, index: u32) -> Result<String, MissingId> {
    Ok(match render_group_name(data, group_list, index)? {
        Some(name) => name,
        None => (index + 1).to_string(),
    })
}

/// Joins already-rendered items the way a French sentence does: « a », « a et
/// b », « a, b et c ».
pub fn join_french(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [single] => single.clone(),
        [head @ .., last] => format!("{} et {}", head.join(", "), last),
    }
}

#[cfg(test)]
mod tests {
    //! The two branchy leaves of this module.
    //!
    //! Everything else here is one lookup and one `format!`, and the property
    //! walk (`tests/property_update_ops.rs`) drives all of them at full width
    //! through the warning texts — what it cannot do is *choose* the shapes,
    //! and these two have shapes worth choosing. [join_french] has three, and
    //! [render_period] has six: a period spanning several weeks, a period
    //! spanning one (the singular « semaine {n} »), and an empty one (« (vide) »,
    //! which must win over everything else), each with and without a document
    //! start date.
    //!
    //! Deliberately no French text pins beyond these: with typed ids a
    //! cross-field mix-up does not compile, and pinning wording would only
    //! manufacture fixture churn on every polish.

    use super::*;
    use crate::{Desc, OpCategory};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{NewId, Op, PeriodOp, WeekOp, weeks::WeekDesc};

    fn desc() -> Desc {
        (OpCategory::GeneralPlanning, "Construire le document".into())
    }

    /// Three periods, spanning four weeks, one week and no week at all — the
    /// three branches of [render_period], in display order so their numbers
    /// read 1, 2, 3.
    fn three_periods(first_week: Option<collomatique_time::WeekStart>) -> (Data, Vec<PeriodId>) {
        let mut state = AppState::<_, Desc>::new(Data::default());

        state
            .apply(Op::Period(PeriodOp::ChangeStartDate(first_week)), desc())
            .expect("setting the start date should land");

        let mut periods = Vec::new();
        for _ in 0..3 {
            let op = match periods.last() {
                None => PeriodOp::AddFront,
                Some(previous) => PeriodOp::AddAfter(*previous),
            };
            match state.apply(Op::Period(op), desc()) {
                Ok(Some(NewId::PeriodId(id))) => periods.push(id),
                other => panic!("adding a period should return a period id, got {other:?}"),
            }
        }

        for (period, week_count) in [(periods[0], 4), (periods[1], 1), (periods[2], 0)] {
            let mut previous = None;
            for _ in 0..week_count {
                let op = match previous {
                    None => WeekOp::AddFront(period, WeekDesc::new(true)),
                    Some(week) => WeekOp::AddAfter(week, WeekDesc::new(true)),
                };
                match state.apply(Op::Week(op), desc()) {
                    Ok(Some(NewId::WeekId(id))) => previous = Some(id),
                    other => panic!("adding a week should return a week id, got {other:?}"),
                }
            }
        }

        (state.get_data().clone(), periods)
    }

    /// Monday 31 August 2026 — the document's first week.
    fn start_date() -> collomatique_time::WeekStart {
        collomatique_time::WeekStart::new(
            chrono::NaiveDate::from_ymd_opt(2026, 8, 31).expect("valid date"),
        )
        .expect("31 August 2026 is a Monday")
    }

    #[test]
    fn join_french_reads_as_a_sentence() {
        assert_eq!(join_french(&["a".to_string()]), "a");
        assert_eq!(join_french(&["a".to_string(), "b".to_string()]), "a et b");
        assert_eq!(
            join_french(&["a".to_string(), "b".to_string(), "c".to_string()]),
            "a, b et c",
        );
        // No caller can reach this one — a fix naming zero groups is not a fix
        // — but the branch exists, so it says what it does.
        assert_eq!(join_french(&[]), "");
    }

    #[test]
    fn render_period_without_a_start_date_says_which_weeks() {
        let (data, periods) = three_periods(None);

        assert_eq!(
            render_period(&data, periods[0]),
            Ok("1 (semaines 1 à 4)".to_string())
        );
        assert_eq!(
            render_period(&data, periods[1]),
            Ok("2 (semaine 5)".to_string())
        );
        assert_eq!(render_period(&data, periods[2]), Ok("3 (vide)".to_string()));
    }

    #[test]
    fn render_period_with_a_start_date_adds_the_dates() {
        let (data, periods) = three_periods(Some(start_date()));

        assert_eq!(
            render_period(&data, periods[0]),
            Ok("1 (du 31/08/2026 au 27/09/2026 - semaines 1 à 4)".to_string()),
        );
        assert_eq!(
            render_period(&data, periods[1]),
            Ok("2 (du 28/09/2026 au 04/10/2026 - semaine 5)".to_string()),
        );
        // The empty branch wins over the dates: a period with no week has no
        // span to print.
        assert_eq!(render_period(&data, periods[2]), Ok("3 (vide)".to_string()));
    }

    #[test]
    fn render_period_on_a_dead_id_names_the_period() {
        let (data, periods) = three_periods(None);
        let dead = periods[2];

        let mut state = AppState::<_, Desc>::new(data);
        state
            .apply(Op::Period(PeriodOp::Remove(dead)), desc())
            .expect("the period is empty, so removing it breaks nothing");

        assert_eq!(
            render_period(state.get_data(), dead),
            Err(MissingId::Period(dead)),
        );
    }
}
