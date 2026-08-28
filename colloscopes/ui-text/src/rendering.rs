//! How an entity of the document is named to a user.
//!
//! One function per id kind, each doing its own lookups. Every renderer is
//! **noun-less**: it answers « 5 (du 28/09/2026 au 04/10/2026) », never
//! « Semaine 5 … ». The caller owns the sentence, and so owns the noun, the
//! article and the agreement — « La semaine {} sera supprimée » and « les
//! semaines {} et {} » are the same rendered week in two different sentences.
//!
//! # What a renderer takes
//!
//! The parts of the document it reads, in
//! [Parameters](collomatique_state_colloscopes::colloscope_params::Parameters)
//! field order, then the ids. A week is named from `(&Periods, &Weeks)`, a slot
//! from `(&Subjects, &Teachers, &Slots)`, a group from `&GroupLists`.
//!
//! Taking the whole document instead would be shorter to write here and
//! unusable at the other end: gtk4's panels are relm4 components that own
//! *subset clones* of the document — a `Periods` and a `Weeks`, or a `Subjects`
//! and a `Pairings` — and never a `Data`. Only the top-level editor holds one.
//! A `&Data` signature would therefore force every title to be rendered up
//! there and threaded back down through the message tree, which is a lot of
//! plumbing to hide the fact that naming a week reads two tables. The one
//! caller that *does* hold the whole document,
//! [`collomatique_ops::warning_text`], projects it in one line.
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
//! rule's rendering corollary (see [`collomatique_ops::cascade`]). The `Err` is
//! the instrument that surfaces a violation of it, which is why the callers
//! that must not fail (gtk4, the property walk) panic on it rather than
//! degrade. The solver's own renderer
//! (`collomatique_constraints_colloscopes::types::user_readable`) makes the
//! opposite choice on purpose — a diagnostic must stay printable, so it
//! degrades to a `{:?}` fallback. The formats below are shared with it and with
//! gtk4; the miss policy is not.

use collomatique_state_colloscopes::group_lists::GroupLists;
use collomatique_state_colloscopes::incompats::Incompats;
use collomatique_state_colloscopes::pairings::Pairings;
use collomatique_state_colloscopes::periods::Periods;
use collomatique_state_colloscopes::slot_pairings::SlotPairings;
use collomatique_state_colloscopes::slots::Slots;
use collomatique_state_colloscopes::students::Students;
use collomatique_state_colloscopes::subjects::Subjects;
use collomatique_state_colloscopes::teachers::Teachers;
use collomatique_state_colloscopes::week_patterns::WeekPatterns;
use collomatique_state_colloscopes::weeks::Weeks;
use collomatique_state_colloscopes::{
    GroupListId, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekId, WeekPatternId,
};
use std::collections::BTreeSet;

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

/// A week, as its global 1-based number, with its dates when the document has a
/// start date: « 5 (du 28/09/2026 au 04/10/2026) », or bare « 5 ».
pub fn render_week(periods: &Periods, weeks: &Weeks, id: WeekId) -> Result<String, MissingId> {
    let week_num = weeks.global_week_position(periods, id).ok_or(id)?;

    Ok(match &periods.first_week {
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
pub fn render_period(periods: &Periods, weeks: &Weeks, id: PeriodId) -> Result<String, MissingId> {
    let (index, first_week_num) = weeks
        .find_period_position_and_first_week(periods, id)
        .ok_or(id)?;
    // A period with no ordering row has no weeks — the same thing an empty row
    // means.
    let week_count = weeks.week_count_for_period(id).unwrap_or(0);

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

    Ok(match &periods.first_week {
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
pub fn render_subject(subjects: &Subjects, id: SubjectId) -> Result<String, MissingId> {
    let subject = subjects.find_subject(id).ok_or(id)?;
    Ok(subject.parameters.name.clone())
}

/// A teacher, as « {firstname} {surname} ».
pub fn render_teacher(teachers: &Teachers, id: TeacherId) -> Result<String, MissingId> {
    let teacher = teachers.teacher_map.get(&id).ok_or(id)?;
    Ok(format!(
        "{} {}",
        teacher.desc.firstname, teacher.desc.surname
    ))
}

/// A student, as « {firstname} {surname} ».
pub fn render_student(students: &Students, id: StudentId) -> Result<String, MissingId> {
    let student = students.student_map.get(&id).ok_or(id)?;
    Ok(format!(
        "{} {}",
        student.desc.firstname, student.desc.surname
    ))
}

/// A week pattern, as its bare name.
pub fn render_week_pattern(
    week_patterns: &WeekPatterns,
    id: WeekPatternId,
) -> Result<String, MissingId> {
    let pattern = week_patterns.week_pattern_map.get(&id).ok_or(id)?;
    Ok(pattern.name.clone())
}

/// A group list, as its bare name.
pub fn render_group_list(group_lists: &GroupLists, id: GroupListId) -> Result<String, MissingId> {
    let group_list = group_lists.group_list_map.get(&id).ok_or(id)?;
    Ok(group_list.params().name.clone())
}

/// An incompatibility, as its bare name.
pub fn render_incompat(incompats: &Incompats, id: IncompatId) -> Result<String, MissingId> {
    let incompat = incompats.incompat_map.get(&id).ok_or(id)?;
    Ok(incompat.name.clone())
}

/// A slot: « Séverus Rogue - Lundi 14h00 (Physique) ».
///
/// The teacher comes first because that is how a user *finds* a slot; the
/// subject trails in parentheses because it is context rather than the slot
/// itself. All three parts are needed — a teacher may hold several slots at the
/// same hour in different subjects.
pub fn render_slot(
    subjects: &Subjects,
    teachers: &Teachers,
    slots: &Slots,
    id: SlotId,
) -> Result<String, MissingId> {
    let (subject_id, _slot) = slots.find_slot_with_subject(id).ok_or(id)?;
    let slot_text = render_slot_in_subject(teachers, slots, id)?;
    let subject = render_subject(subjects, subject_id)?;
    Ok(format!("{} ({})", slot_text, subject))
}

/// A slot with its subject left out: « Séverus Rogue - Lundi 14h00 ».
///
/// For anywhere the subject is already established and repeating it would be
/// noise: the slot pairing rule notation names it once and then uses this form
/// twice, and gtk4's slot pairings tab is grouped by subject throughout (its
/// `build_slot_description` is this exact format).
pub fn render_slot_in_subject(
    teachers: &Teachers,
    slots: &Slots,
    id: SlotId,
) -> Result<String, MissingId> {
    let slot = slots.find_slot(id).ok_or(id)?;
    let teacher = render_teacher(teachers, slot.teacher_id)?;
    Ok(format!("{} - {}", teacher, slot.start_time.capitalize()))
}

/// A pairing rule, in the notation its own tab uses:
/// « Avoir Physique ⟹ Ne pas avoir Chimie ».
///
/// Softness is deliberately left out: it is a property of the rule, not part of
/// what identifies it. The list views append « (souple) » themselves.
pub fn render_pairing_rule(
    subjects: &Subjects,
    pairings: &Pairings,
    id: PairingRuleId,
) -> Result<String, MissingId> {
    let rule = pairings.pairing_rule_map.get(&id).ok_or(id)?;
    let ant_name = render_subject(subjects, rule.antecedent().subject_id)?;
    let con_name = render_subject(subjects, rule.consequent().subject_id)?;
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

/// A slot pairing rule: « Physique : \[utilisé\] Séverus Rogue - Lundi 14h00 ⟹
/// \[non utilisé\] Minerve McGonagall - Mardi 15h00 ».
///
/// The subject is fronted once — the rule's own tab groups rules by subject and
/// so never repeats it, but a warning arrives without that context. Both slots
/// belong to that same subject (a rule pairing slots of two subjects is not a
/// legal document), so the antecedent's answers for both, and each part is
/// rendered without its own subject parentheses.
pub fn render_slot_pairing_rule(
    subjects: &Subjects,
    teachers: &Teachers,
    slots: &Slots,
    slot_pairings: &SlotPairings,
    id: SlotPairingRuleId,
) -> Result<String, MissingId> {
    let rule = slot_pairings.slot_pairing_rule_map.get(&id).ok_or(id)?;
    let ant_slot = rule.antecedent().slot_id;
    let con_slot = rule.consequent().slot_id;

    let (subject_id, _slot) = slots.find_slot_with_subject(ant_slot).ok_or(ant_slot)?;
    let subject = render_subject(subjects, subject_id)?;

    Ok(format!(
        "{} : [{}] {} \u{27F9} [{}] {}",
        subject,
        use_condition(rule.antecedent().should_have),
        render_slot_in_subject(teachers, slots, ant_slot)?,
        use_condition(rule.consequent().should_have),
        render_slot_in_subject(teachers, slots, con_slot)?,
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
    group_lists: &GroupLists,
    group_list: GroupListId,
    index: u32,
) -> Result<Option<String>, MissingId> {
    let list = group_lists
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
pub fn render_group(
    group_lists: &GroupLists,
    group_list: GroupListId,
    index: u32,
) -> Result<String, MissingId> {
    Ok(match render_group_name(group_lists, group_list, index)? {
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

/// What one generated group list covers — « Sortilèges (période 1) »,
/// « Sortilèges et Métamorphose (périodes 1 et 2) » — and therefore its
/// default name: distinct specs cover disjoint pair sets, so these labels
/// are unique. Subjects come out in document display order, periods as their
/// 1-based positions in it.
///
/// The two exceptions to the module's rules, both because this is a *name*
/// and not material for a caller's sentence. It carries its own noun
/// («période»), being complete on its own; and it is infallible, because it
/// renders a *set* by filtering the two tables — an id the tables do not hold
/// is simply not printed, and the pairs come out of a generation plan built
/// against these very parameters, where every id holds.
pub fn coverage_label(
    periods: &Periods,
    subjects: &Subjects,
    covered: &BTreeSet<(PeriodId, SubjectId)>,
) -> String {
    let subject_ids: BTreeSet<SubjectId> = covered.iter().map(|&(_, subject)| subject).collect();
    let period_ids: BTreeSet<PeriodId> = covered.iter().map(|&(period, _)| period).collect();

    let subject_names: Vec<String> = subjects
        .ordered_subject_list
        .iter()
        .filter(|(id, _subject)| subject_ids.contains(id))
        .map(|(_id, subject)| subject.parameters.name.clone())
        .collect();

    // Periods have no name: the 1-based position is what the whole UI shows.
    let period_positions: Vec<String> = periods
        .period_ids()
        .enumerate()
        .filter(|(_pos, id)| period_ids.contains(id))
        .map(|(pos, _id)| (pos + 1).to_string())
        .collect();

    let period_part = if period_positions.len() == 1 {
        format!("période {}", period_positions[0])
    } else {
        format!("périodes {}", join_french(&period_positions))
    };

    format!("{} ({})", join_french(&subject_names), period_part)
}

#[cfg(test)]
mod tests {
    //! The three branchy leaves of this module.
    //!
    //! Everything else here is one lookup and one `format!`, and the property
    //! walk (`tests/property_update_ops.rs`) drives all of them at full width
    //! through the warning texts — what it cannot do is *choose* the shapes,
    //! and these three have shapes worth choosing. [join_french] has three, and
    //! [render_period] has six: a period spanning several weeks, a period
    //! spanning one (the singular « semaine {n} »), and an empty one (« (vide) »,
    //! which must win over everything else), each with and without a document
    //! start date. [coverage_label] has the singular and the plural of
    //! « période », and an ordering of its own to keep.
    //!
    //! Deliberately no French text pins beyond these: with typed ids a
    //! cross-field mix-up does not compile, and pinning wording would only
    //! manufacture fixture churn on every polish.

    use super::*;
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::colloscope_params::Parameters;
    use collomatique_state_colloscopes::subjects::{Subject, SubjectParameters};
    use collomatique_state_colloscopes::{
        Data, NewId, Op, PeriodOp, SubjectOp, WeekOp, weeks::WeekDesc,
    };

    /// A stand-in for `collomatique_ops`'s description types
    ///
    /// The tests build their document through `AppState`, whose `Description`
    /// is only `Send + Sync + Clone` metadata for the undo history — importing
    /// the real `Desc` would make this crate depend on the crate that depends
    /// on it.
    #[derive(Debug, Clone)]
    enum Category {
        GeneralPlanning,
    }

    type Desc = (Category, String);

    fn desc() -> Desc {
        (Category::GeneralPlanning, "Construire le document".into())
    }

    fn params(data: &Data) -> &Parameters {
        &data.get_inner_data().params
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

    /// Two named subjects over the periods of [three_periods], returned in
    /// document display order.
    ///
    /// They are added back to front, so that display order and id order
    /// disagree: a [coverage_label] built from the ids alone would answer
    /// « Métamorphose et Sortilèges ».
    fn periods_and_subjects() -> (Data, Vec<PeriodId>, Vec<SubjectId>) {
        let (data, periods) = three_periods(None);
        let mut state = AppState::<_, Desc>::new(data);

        let mut subjects = Vec::new();
        for name in ["Métamorphose", "Sortilèges"] {
            let subject = Subject {
                parameters: SubjectParameters {
                    name: name.to_string(),
                    ..Default::default()
                },
                ..Default::default()
            };
            match state.apply(Op::Subject(SubjectOp::AddAfter(None, subject)), desc()) {
                Ok(Some(NewId::SubjectId(id))) => subjects.push(id),
                other => panic!("adding a subject should return a subject id, got {other:?}"),
            }
        }
        subjects.reverse();

        (state.get_data().clone(), periods, subjects)
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
        let params = params(&data);

        assert_eq!(
            render_period(&params.periods, &params.weeks, periods[0]),
            Ok("1 (semaines 1 à 4)".to_string())
        );
        assert_eq!(
            render_period(&params.periods, &params.weeks, periods[1]),
            Ok("2 (semaine 5)".to_string())
        );
        assert_eq!(
            render_period(&params.periods, &params.weeks, periods[2]),
            Ok("3 (vide)".to_string())
        );
    }

    #[test]
    fn render_period_with_a_start_date_adds_the_dates() {
        let (data, periods) = three_periods(Some(start_date()));
        let params = params(&data);

        assert_eq!(
            render_period(&params.periods, &params.weeks, periods[0]),
            Ok("1 (du 31/08/2026 au 27/09/2026 - semaines 1 à 4)".to_string()),
        );
        assert_eq!(
            render_period(&params.periods, &params.weeks, periods[1]),
            Ok("2 (du 28/09/2026 au 04/10/2026 - semaine 5)".to_string()),
        );
        // The empty branch wins over the dates: a period with no week has no
        // span to print.
        assert_eq!(
            render_period(&params.periods, &params.weeks, periods[2]),
            Ok("3 (vide)".to_string())
        );
    }

    #[test]
    fn coverage_label_of_a_single_pair_is_singular() {
        let (data, periods, subjects) = periods_and_subjects();
        let params = params(&data);

        assert_eq!(
            coverage_label(
                &params.periods,
                &params.subjects,
                &BTreeSet::from([(periods[0], subjects[0])]),
            ),
            "Sortilèges (période 1)",
        );
    }

    #[test]
    fn coverage_label_enumerates_both_halves_in_document_order() {
        let (data, periods, subjects) = periods_and_subjects();
        let params = params(&data);

        // Both subjects on both of the first two periods: four pairs, each
        // half enumerated once, and the plural of « période ».
        let covered = BTreeSet::from([
            (periods[1], subjects[1]),
            (periods[0], subjects[0]),
            (periods[0], subjects[1]),
            (periods[1], subjects[0]),
        ]);

        assert_eq!(
            coverage_label(&params.periods, &params.subjects, &covered),
            "Sortilèges et Métamorphose (périodes 1 et 2)",
        );
    }

    #[test]
    fn render_period_on_a_dead_id_names_the_period() {
        let (data, periods) = three_periods(None);
        let dead = periods[2];

        let mut state = AppState::<_, Desc>::new(data);
        state
            .apply(Op::Period(PeriodOp::Remove(dead)), desc())
            .expect("the period is empty, so removing it breaks nothing");
        let params = params(state.get_data());

        assert_eq!(
            render_period(&params.periods, &params.weeks, dead),
            Err(MissingId::Period(dead)),
        );
    }
}
