//! The slot pairing rules of a document
//!
//! Reached as `doc.slot_pairings`. A slot pairing rule is the slots' version of
//! a pairing rule (§3.12 of `docs/python/handle_api.md`): if a slot is used in
//! a week, the other slot must also be used — or not used. Both slots always
//! belong to the same subject (a document that paired slots of two subjects is
//! not a legal one), which is how the repr names the rule: the subject once,
//! then the two slots in the application's own notation.
//!
//! The two ends are the [SlotPairingRuleSide] sub-views, bound to
//! `(document, rule_id, side)` like their subject-level cousins, and they go
//! stale with their rule.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::SlotPairingRuleId as RawSlotPairingRuleId;
use collomatique_state_colloscopes::slot_pairings::SlotRulePart;

use crate::Document;
use crate::collections::periods::Period;
use crate::collections::slots::Slot;
use crate::errors::StaleHandleError;
use crate::handles::{Handle, RuleSide, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, SlotPairingRuleId};

/// The slot pairing rules of one document, in id order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// The model keeps no display order for the slot pairing rules — the
/// application groups them by subject as it shows them — so the order here is
/// the ids', which is the one order the document itself has.
#[pyclass(module = "collomatique", frozen)]
pub struct SlotPairings {
    doc: Py<Document>,
}

impl SlotPairings {
    /// Builds the view — `doc.slot_pairings` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> SlotPairings {
        SlotPairings { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The slot pairing rule an id or a handle names, when this document still
    /// holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawSlotPairingRuleId> {
        let id = named::<SlotPairingRule>(&self.doc, key)?;
        self.with_data(py, |data| SlotPairingRule::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl SlotPairings {
    /// How many slot pairing rules the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| {
            data.params.slot_pairings.slot_pairing_rule_map.len()
        })
    }

    /// The slot pairing rules, as handles, in id order
    fn __iter__(&self, py: Python<'_>) -> SlotPairingRuleIter {
        let ids = self.with_data(py, |data| {
            data.params
                .slot_pairings
                .slot_pairing_rule_map
                .keys()
                .collect()
        });
        SlotPairingRuleIter::new(self.doc.clone_ref(py), ids)
    }

    /// The slot pairing rule an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<SlotPairingRule> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("slot pairing rule", key))?;
        Ok(SlotPairingRule::mint(self.doc.clone_ref(py), id))
    }

    /// The slot pairing rule an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<SlotPairingRule> {
        let id = self.resolve(py, key)?;
        Some(SlotPairingRule::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.SlotPairings count={}>", self.__len__(py))
    }
}

handle_iterator! {
    /// The slot pairing rules of a collection, minted as the loop asks for them
    SlotPairingRuleIter yielding SlotPairingRule
}

/// One slot pairing rule of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose rule has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
///
/// The rule is an implication between two slots of the same subject: if the
/// antecedent slot is used in a week, the consequent slot must also be used —
/// or not. `.soft` says whether it is an objective for the solver rather than a
/// hard constraint, and `.excluded_periods` the periods it does not apply to.
#[pyclass(module = "collomatique", frozen)]
pub struct SlotPairingRule {
    doc: Py<Document>,
    id: RawSlotPairingRuleId,
}

impl Handle for SlotPairingRule {
    type IdClass = SlotPairingRuleId;

    const CLASS: &'static str = "SlotPairingRule";
    const NOUN: &'static str = "slot pairing rule";

    fn mint(doc: Py<Document>, id: RawSlotPairingRuleId) -> SlotPairingRule {
        SlotPairingRule { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawSlotPairingRuleId {
        self.id
    }

    fn exists(data: &InnerData, id: RawSlotPairingRuleId) -> bool {
        data.params
            .slot_pairings
            .slot_pairing_rule_map
            .contains(&id)
    }
}

#[pymethods]
impl SlotPairingRule {
    /// The rule's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> SlotPairingRuleId {
        SlotPairingRuleId::wrap(self.id)
    }

    /// The antecedent side of the implication
    ///
    /// The « if » half: a week that uses the side's slot must also satisfy the
    /// consequent. The view goes stale with the rule.
    #[getter]
    fn antecedent(&self, py: Python<'_>) -> PyResult<SlotPairingRuleSide> {
        self.read(py, |data| {
            data.params
                .slot_pairings
                .slot_pairing_rule_map
                .contains(&self.id)
                .then_some(())
        })?;
        Ok(SlotPairingRuleSide::mint(
            self.doc.clone_ref(py),
            self.id,
            RuleSide::Antecedent,
        ))
    }

    /// The consequent side of the implication
    ///
    /// The « then » half: what a week using the antecedent slot is also marked
    /// for — or off. The view goes stale with the rule.
    #[getter]
    fn consequent(&self, py: Python<'_>) -> PyResult<SlotPairingRuleSide> {
        self.read(py, |data| {
            data.params
                .slot_pairings
                .slot_pairing_rule_map
                .contains(&self.id)
                .then_some(())
        })?;
        Ok(SlotPairingRuleSide::mint(
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
                .slot_pairings
                .slot_pairing_rule_map
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
                .slot_pairings
                .slot_pairing_rule_map
                .get(&self.id)
                .map(|rule| rule.soft())
        })
    }

    /// Nothing can point at a slot pairing rule: the reference registry has no
    /// site vocabulary for the kind, so the answer is always the empty tuple
    /// while the handle is alive. A stale handle raises `StaleHandleError` like
    /// every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::never_referenced::<Self>(py, self)
    }

    /// Whether two handles name the same slot pairing rule of the same document
    ///
    /// Never reads the state, so it keeps working once the rule is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<SlotPairingRule>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let rendered = self.peek(py, |data| {
            collomatique_ui_text::rendering::render_slot_pairing_rule(
                &data.params.subjects,
                &data.params.teachers,
                &data.params.slots,
                &data.params.slot_pairings,
                self.id,
            )
            .ok()
        });
        self.repr_text(rendered.map(|text| quoted(py, &text)))
    }
}

/// One end of a slot pairing rule
///
/// A sub-view, which is a handle in everything but the `.id`
/// (`docs/python/handle_api.md` §1): it is bound to `(document, rule_id,
/// side)`, reads the current state on every access, and goes stale with its
/// rule. `rule.antecedent` and `rule.consequent` are the only ways to get one,
/// and asking the rule again always answers the current truth.
///
/// What it reads is the side's part of the rule: `.slot`, a live [Slot] handle
/// of this document, and `.should_have`, whether a week marked for the rule
/// has that slot used.
///
/// The repr names the side and the slot it is about, the way the application's
/// own slot descriptions do; a view whose rule is gone prints `(périmé)`.
#[pyclass(module = "collomatique", frozen)]
pub struct SlotPairingRuleSide {
    doc: Py<Document>,
    id: RawSlotPairingRuleId,
    side: RuleSide,
}

impl SlotPairingRuleSide {
    /// Builds the view — `rule.antecedent` and `rule.consequent` are the only
    /// ways to get one
    pub(crate) fn mint(
        doc: Py<Document>,
        id: RawSlotPairingRuleId,
        side: RuleSide,
    ) -> SlotPairingRuleSide {
        SlotPairingRuleSide { doc, id, side }
    }

    /// Borrows the document, finds the rule, and reads the side's part of it
    ///
    /// The one way of dying: the rule is gone. There is no second way the
    /// `Interrogation` view has to tell apart — a slot pairing rule has no
    /// switch that turns one end off while the other lives.
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&SlotRulePart) -> R) -> PyResult<R> {
        let doc = self.doc.borrow(py);
        let rule = doc
            .data()
            .get_inner_data()
            .params
            .slot_pairings
            .slot_pairing_rule_map
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
            "this SlotPairingRuleSide view is stale: slot pairing rule {} is \
             no longer in the document",
            SlotPairingRuleId::text(self.id),
        ))
    }
}

#[pymethods]
impl SlotPairingRuleSide {
    /// The slot this side of the rule is about
    ///
    /// A live handle of this document, whatever the rule's other side says.
    #[getter]
    fn slot(&self, py: Python<'_>) -> PyResult<Slot> {
        let slot_id = self.read(py, |part| part.slot_id)?;
        Ok(Slot::mint(self.doc.clone_ref(py), slot_id))
    }

    /// Whether a week marked for the rule should use this slot
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
        match other.cast::<SlotPairingRuleSide>() {
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

        let description = self.peek(py, |data| {
            let rule = data
                .params
                .slot_pairings
                .slot_pairing_rule_map
                .get(&self.id)?;
            let part = match self.side {
                RuleSide::Antecedent => rule.antecedent(),
                RuleSide::Consequent => rule.consequent(),
            };
            collomatique_ui_text::rendering::render_slot_in_subject(
                &data.params.teachers,
                &data.params.slots,
                part.slot_id,
            )
            .ok()
        });
        match description {
            Some(description) => format!(
                "<SlotPairingRuleSide #{} ({}) {}>",
                self.id.inner(),
                side_word(self.side),
                quoted(py, &description),
            ),
            None => format!("<SlotPairingRuleSide #{} (périmé)>", self.id.inner()),
        }
    }
}

/// The side as words, for the repr — « (antécédent) 'Séverus Rogue - Lundi
/// 14h00' »
fn side_word(side: RuleSide) -> &'static str {
    match side {
        RuleSide::Antecedent => "antécédent",
        RuleSide::Consequent => "conséquent",
    }
}
