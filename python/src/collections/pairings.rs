//! The pairing rules of a document
//!
//! Reached as `doc.pairings`. A pairing rule is an implication between two
//! subjects: if a student is marked to have the antecedent subject's
//! interrogation in a week, they are also marked to have — or not to have —
//! the consequent's that week (§3.11 of `docs/python/handle_api.md`). The two
//! ends are the [PairingRuleSide] sub-views, which are handles in everything
//! but the `.id`: bound to `(document, rule_id, side)`, they read the current
//! state and go stale with their rule.
//!
//! A rule only applies to students enrolled in both subjects, and it is either
//! a hard constraint or an objective for the solver to optimize — the `.soft`
//! flag. The periods a rule does not apply to are its `.excluded_periods`.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::PairingRuleId as RawPairingRuleId;
use collomatique_state_colloscopes::pairings::RulePart;

use crate::Document;
use crate::collections::periods::Period;
use crate::collections::subjects::Subject;
use crate::errors::StaleHandleError;
use crate::handles::{Handle, RuleSide, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, PairingRuleId};

/// The pairing rules of one document, in id order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// The model keeps no display order for the pairing rules — the application
/// lists them as the table hands them over — so the order here is the ids',
/// which is the one order the document itself has.
#[pyclass(module = "collomatique", frozen)]
pub struct Pairings {
    doc: Py<Document>,
}

impl Pairings {
    /// Builds the view — `doc.pairings` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Pairings {
        Pairings { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The pairing rule an id or a handle names, when this document still holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawPairingRuleId> {
        let id = named::<PairingRule>(&self.doc, key)?;
        self.with_data(py, |data| PairingRule::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl Pairings {
    /// How many pairing rules the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.pairings.pairing_rule_map.len())
    }

    /// The pairing rules, as handles, in id order
    fn __iter__(&self, py: Python<'_>) -> PairingRuleIter {
        let ids = self.with_data(py, |data| {
            data.params.pairings.pairing_rule_map.keys().collect()
        });
        PairingRuleIter::new(self.doc.clone_ref(py), ids)
    }

    /// The pairing rule an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<PairingRule> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("pairing rule", key))?;
        Ok(PairingRule::mint(self.doc.clone_ref(py), id))
    }

    /// The pairing rule an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<PairingRule> {
        let id = self.resolve(py, key)?;
        Some(PairingRule::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Pairings count={}>", self.__len__(py))
    }
}

handle_iterator! {
    /// The pairing rules of a collection, minted as the loop asks for them
    PairingRuleIter yielding PairingRule
}

/// One pairing rule of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose rule has been removed raises `StaleHandleError`; `.id`, `==` and `hash`
/// keep working, since they never touch the state.
///
/// The rule is an implication: a student who `should_have` the antecedent
/// subject's interrogation in a week should (or should not) have the
/// consequent's that week. It only applies to students enrolled in both
/// subjects, and `.soft` says whether it is an objective for the solver rather
/// than a hard constraint.
#[pyclass(module = "collomatique", frozen)]
pub struct PairingRule {
    doc: Py<Document>,
    id: RawPairingRuleId,
}

impl Handle for PairingRule {
    type IdClass = PairingRuleId;

    const CLASS: &'static str = "PairingRule";
    const NOUN: &'static str = "pairing rule";

    fn mint(doc: Py<Document>, id: RawPairingRuleId) -> PairingRule {
        PairingRule { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawPairingRuleId {
        self.id
    }

    fn exists(data: &InnerData, id: RawPairingRuleId) -> bool {
        data.params.pairings.pairing_rule_map.contains(&id)
    }
}

#[pymethods]
impl PairingRule {
    /// The rule's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> PairingRuleId {
        PairingRuleId::wrap(self.id)
    }

    /// The antecedent side of the implication
    ///
    /// The « if » half: a student who `should_have` the side's subject's
    /// interrogation in a week must also satisfy the consequent. The view goes
    /// stale with the rule.
    #[getter]
    fn antecedent(&self, py: Python<'_>) -> PyResult<PairingRuleSide> {
        self.read(py, |data| {
            data.params
                .pairings
                .pairing_rule_map
                .contains(&self.id)
                .then_some(())
        })?;
        Ok(PairingRuleSide::mint(
            self.doc.clone_ref(py),
            self.id,
            RuleSide::Antecedent,
        ))
    }

    /// The consequent side of the implication
    ///
    /// The « then » half: what a student who satisfies the antecedent is also
    /// marked for — or off. The view goes stale with the rule.
    #[getter]
    fn consequent(&self, py: Python<'_>) -> PyResult<PairingRuleSide> {
        self.read(py, |data| {
            data.params
                .pairings
                .pairing_rule_map
                .contains(&self.id)
                .then_some(())
        })?;
        Ok(PairingRuleSide::mint(
            self.doc.clone_ref(py),
            self.id,
            RuleSide::Consequent,
        ))
    }

    /// The periods the rule does not apply to, as a `frozenset` of [Period]
    ///
    /// A snapshot, built when it is asked for: the set does not grow when the
    /// document does. The handles in it stay live.
    #[getter]
    fn excluded_periods<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyFrozenSet>> {
        let ids = self.read(py, |data| {
            data.params
                .pairings
                .pairing_rule_map
                .get(&self.id)
                .map(|rule| rule.excluded_periods().iter().copied().collect::<Vec<_>>())
        })?;

        let periods: Vec<_> = ids
            .into_iter()
            .map(|period_id| Period::mint(self.doc.clone_ref(py), period_id))
            .collect();
        PyFrozenSet::new(py, periods)
    }

    /// Whether the rule is an objective rather than a hard constraint
    ///
    /// The solver optimizes for a soft rule and enforces a strict one.
    #[getter]
    fn soft(&self, py: Python<'_>) -> PyResult<bool> {
        self.read(py, |data| {
            data.params
                .pairings
                .pairing_rule_map
                .get(&self.id)
                .map(|rule| rule.soft())
        })
    }

    /// Whether two handles name the same pairing rule of the same document
    ///
    /// Never reads the state, so it keeps working once the rule is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<PairingRule>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let rendered = self.peek(py, |data| {
            collomatique_ui_text::rendering::render_pairing_rule(
                &data.params.subjects,
                &data.params.pairings,
                self.id,
            )
            .ok()
        });
        self.repr_text(rendered.map(|text| quoted(py, &text)))
    }
}

/// One end of a pairing rule
///
/// A sub-view, which is a handle in everything but the `.id`
/// (`docs/python/handle_api.md` §1): it is bound to `(document, rule_id,
/// side)`, reads the current state on every access, and goes stale with its
/// rule. `rule.antecedent` and `rule.consequent` are the only ways to get one,
/// and asking the rule again always answers the current truth.
///
/// What it reads is the side's part of the rule: `.subject`, a live [Subject]
/// handle of this document, and `.should_have`, whether a student marked for
/// the rule is marked *for* that subject's interrogation.
///
/// The repr names the side and the subject it is about, the way the reprs of
/// the handles name their entity; a view whose rule is gone prints `(périmé)`.
#[pyclass(module = "collomatique", frozen)]
pub struct PairingRuleSide {
    doc: Py<Document>,
    id: RawPairingRuleId,
    side: RuleSide,
}

impl PairingRuleSide {
    /// Builds the view — `rule.antecedent` and `rule.consequent` are the only
    /// ways to get one
    pub(crate) fn mint(doc: Py<Document>, id: RawPairingRuleId, side: RuleSide) -> PairingRuleSide {
        PairingRuleSide { doc, id, side }
    }

    /// Borrows the document, finds the rule, and reads the side's part of it
    ///
    /// The one way of dying: the rule is gone. There is no second way the
    /// `Interrogation` view has to tell apart — a pairing rule has no switch
    /// that turns one end off while the other lives.
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&RulePart) -> R) -> PyResult<R> {
        let doc = self.doc.borrow(py);
        let rule = doc
            .data()
            .get_inner_data()
            .params
            .pairings
            .pairing_rule_map
            .get(&self.id)
            .ok_or_else(|| self.stale())?;
        let part = match self.side {
            RuleSide::Antecedent => rule.antecedent(),
            RuleSide::Consequent => rule.consequent(),
        };
        Ok(f(part))
    }

    /// Reads without saying anything about liveness — for `repr`, which never
    /// raises
    fn peek<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> Option<R>) -> Option<R> {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The error a read through a dead rule raises
    fn stale(&self) -> PyErr {
        StaleHandleError::new_err(format!(
            "this PairingRuleSide view is stale: pairing rule {} is no longer \
             in the document",
            PairingRuleId::text(self.id),
        ))
    }
}

#[pymethods]
impl PairingRuleSide {
    /// The subject this side of the rule is about
    ///
    /// A live handle of this document, whatever the rule's other side says.
    #[getter]
    fn subject(&self, py: Python<'_>) -> PyResult<Subject> {
        let subject_id = self.read(py, |part| part.subject_id)?;
        Ok(Subject::mint(self.doc.clone_ref(py), subject_id))
    }

    /// Whether a student marked for the rule should have this subject's
    /// interrogation in a week
    #[getter]
    fn should_have(&self, py: Python<'_>) -> PyResult<bool> {
        self.read(py, |part| part.should_have)
    }

    /// Whether two views are about the same end of the same rule of the same
    /// document
    ///
    /// The side is part of the identity: the two ends of one rule never compare
    /// equal. Never reads the state, so it keeps working once the rule is gone.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<PairingRuleSide>() {
            Ok(other) => {
                let other = other.get();
                std::ptr::eq(self.doc.as_ptr(), other.doc.as_ptr())
                    && self.id == other.id
                    && self.side == other.side
            }
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.doc.as_ptr() as usize).hash(&mut hasher);
        self.id.hash(&mut hasher);
        self.side.hash(&mut hasher);
        hasher.finish()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        use collomatique_state::ids::Id as _;

        let name = self.peek(py, |data| {
            let rule = data.params.pairings.pairing_rule_map.get(&self.id)?;
            let part = match self.side {
                RuleSide::Antecedent => rule.antecedent(),
                RuleSide::Consequent => rule.consequent(),
            };
            collomatique_ui_text::rendering::render_subject(&data.params.subjects, part.subject_id)
                .ok()
        });
        match name {
            Some(name) => format!(
                "<PairingRuleSide #{} ({}) {}>",
                self.id.inner(),
                side_word(self.side),
                quoted(py, &name),
            ),
            None => format!("<PairingRuleSide #{} (périmé)>", self.id.inner()),
        }
    }
}

/// The side as words, for the repr — « (antécédent) 'Sortilèges' »
fn side_word(side: RuleSide) -> &'static str {
    match side {
        RuleSide::Antecedent => "antécédent",
        RuleSide::Consequent => "conséquent",
    }
}
