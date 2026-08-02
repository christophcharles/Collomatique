//! The cascade session: how an [crate::UpdateOp] reaches the state layer.
//!
//! A user-facing op is one or several elementary
//! [collomatique_state_colloscopes::Op]s, and each of them may need repairs
//! before it can land — deleting a teacher takes their slots with it, deleting
//! those slots takes the colloscope cells that used them, and so on. The
//! repairs are the cascade engine's business
//! ([collomatique_state::cascade::apply_cascade], reached through
//! [collomatique_state::traits::Manager::apply_cascade]); what this module adds
//! is the *session*: the ops of one composite are applied one at a time, each
//! through the cascade, the ids they create are handed back inline so the next
//! op can use them, and every repair the engine had to make is collected as a
//! [CascadeWarning] to show the user afterwards.
//!
//! # The prefix-survival frame rule
//!
//! A composite must be written so that each of its ops is valid against the
//! state produced by its own earlier ops **and their cascades**. A composite
//! whose later op is convicted because an earlier op's cascade consumed its
//! target is a bug in the composite, not bad user input — the per-composite
//! fixtures are what establishes that it does not happen.
//!
//! **Rendering corollary**: a composite's cascades must only ever touch
//! material that was already in the composite's *pre-state*. Warnings are
//! rendered lazily against that pre-state (it is the document the user is
//! looking at when the dialog appears), so a repair on material an earlier op
//! of the same composite created could not be described at all — « cette colle
//! n'a jamais existé dans mon document ». That is the same kind of composite
//! bug, and the renderer's lookup panic is where it would surface.

use collomatique_state::AppSession;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{Data, Error, Fix, NewId, Op};

use crate::Desc;
use crate::rendering::MissingId;

/// One warning attached to an update: a repair the cascade had to apply beyond
/// the ops the user's own action asked for.
///
/// That is the *only* kind of warning there is — no composite ever hand-writes
/// one, so the warning list cannot drift from what actually happened. The
/// content is crate-private, readable through [CascadeWarning::fix].
///
/// What it carries is the repair's *meaning* (the [Fix] vocabulary), never the
/// invariant that caused it: which invariant the engine picked never leaves the
/// engine. No text is stored either — [CascadeWarning::text] computes it on
/// demand, against the composite's pre-state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CascadeWarning {
    fix: Fix,
}

impl CascadeWarning {
    pub(crate) fn new(fix: Fix) -> Self {
        CascadeWarning { fix }
    }

    /// Borrowed read-only view of what the repair did.
    pub fn fix(&self) -> &Fix {
        &self.fix
    }

    /// The French, effect-phrased sentence describing the repair, read against
    /// `data`.
    ///
    /// `data` must be the **pre-state** of the composite the warning came from —
    /// the document the user is still looking at when the dialog appears. That
    /// is where the material this warning names is to be found: rendering
    /// against the post-state would look for entities the update has just
    /// removed.
    ///
    /// `Err(MissingId)` means `data` does not hold that material, which is a
    /// violation of the frame rule's rendering corollary (see the module docs)
    /// rather than an outcome to handle: the callers that must not fail panic
    /// on it.
    pub fn text(&self, data: &Data) -> Result<String, MissingId> {
        crate::warning_text::render(&data.get_inner_data().params, &self.fix)
    }
}

/// A modification session that applies elementary ops through the cascade and
/// accumulates every repair as a [CascadeWarning].
///
/// The session owns the [Manager] it was built from and hands it back at the
/// end: always finish with [CascadeSession::commit] or
/// [CascadeSession::cancel], since simply dropping the session loses the
/// manager with it.
///
/// The ops of one composite are applied *sequentially*, not planned ahead: an
/// op that creates an entity returns its id inline, so a later op of the same
/// composite can use it, and each op is applied to the state its predecessors
/// (and their cascades) actually produced.
pub struct CascadeSession<T: Manager<Data = Data, Desc = Desc>> {
    session: AppSession<T, Desc>,
    warnings: Vec<CascadeWarning>,
}

impl<T: Manager<Data = Data, Desc = Desc>> CascadeSession<T> {
    /// Opens a session on `manager`, taking ownership of it.
    pub fn new(manager: T) -> Self {
        CascadeSession {
            session: AppSession::new(manager),
            warnings: Vec::new(),
        }
    }

    /// Read access to the document as the session has it *now* — after every
    /// op applied so far and every repair they cascaded.
    ///
    /// This is what the read-modify-write composites (assignment rows, the
    /// export config, the colloscope lookups) read to build their next op.
    pub fn get_data(&self) -> &Data {
        self.session.get_data()
    }

    /// Applies one elementary op through the cascade.
    ///
    /// Repairs land in the warning log; the id the op created, if any, comes
    /// back inline. On `Err` the document is unchanged and nothing is logged:
    /// the op could not be made sense of against the current state, and no
    /// repair would have helped.
    pub fn apply(&mut self, op: Op, desc: Desc) -> Result<Option<NewId>, Error> {
        let (new_id, fixes) = self.session.apply_cascade(op, desc)?;

        self.warnings
            .extend(fixes.into_iter().map(CascadeWarning::new));

        Ok(new_id)
    }

    /// Closes the session: everything it applied collapses into a single
    /// history slot on the manager, described by `desc`, so one undo takes the
    /// document back to where the composite found it.
    ///
    /// Returns the manager and the warnings, in application order.
    pub fn commit(self, desc: Desc) -> (T, Vec<CascadeWarning>) {
        let CascadeSession { session, warnings } = self;

        (session.commit(desc), warnings)
    }

    /// Abandons the session: every modification is unwound and the manager
    /// comes back as it was. The warnings are dropped with it — they describe
    /// repairs that no longer happened.
    pub fn cancel(self) -> T {
        self.session.cancel()
    }
}

/// The outcome of applying an [crate::UpdateOp] without committing it: the new
/// state is handed to the caller to install (or to drop, if the user refuses
/// the warnings).
pub struct CascadeResult<T: Manager<Data = Data, Desc = Desc>> {
    /// The repairs the cascade had to apply, in application order.
    pub warnings: Vec<CascadeWarning>,
    /// The id created by the op, if it created one.
    pub new_id: Option<NewId>,
    /// The document as it would be, with the whole update as one history slot.
    pub new_state: T,
}
