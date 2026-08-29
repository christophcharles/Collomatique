//! Opening a colloscope file, and writing it back

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pyo3::prelude::*;
use pyo3::types::PyFrozenSet;

use collomatique_ops::{Desc, UpdateOp};
use collomatique_state::SessionStack;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{Data, NewId, Op};
use collomatique_storage::Caveat;

use crate::collections::{
    Assignments, Balancing, Colloscope, ExportConfig, GroupLists, Incompats, Pairings, Periods,
    Settings, Slot, SlotPairings, Slots, Students, Subjects, Teachers, Week, WeekPattern,
    WeekPatterns, Weeks,
};
use crate::dialogs::FileRequest;
use crate::errors::{
    Cancelled, CaveatedOverwrite, Error, ExportError, GroupListsGenerationError, IdCeilingExceeded,
    LoadError, ModelBuildError, NoDocument, NoOrigin, NothingToUndo, SaveError, UpdateError,
};
use crate::model::ColloscopeModel;
use crate::results::{OpResult, Warning};
use crate::transaction::Transaction;

/// Where a document came from, and where a bare `save()` writes
///
/// Fixed at creation and never changed: `save(path)` writes where it is told
/// and does not re-target a later `save()`. Pairing the path with its caveats
/// — mirroring the GUI's `FileName` — makes "caveats with no path"
/// unrepresentable, and gives the hosted document a slot of its own.
/// [Origin::Hosted] carries no caveats, because the handoff carries the `Data`
/// and not the host's caveat set.
enum Origin {
    /// Never on disk and not hosted: `save()` has nowhere to write
    None,
    File {
        path: PathBuf,
        caveats: BTreeSet<Caveat>,
    },
    /// The document the application handed over
    ///
    /// [crate::host::current_document] is the only way to get one: a hosted
    /// document is the one the application is showing, and there is exactly
    /// one of those.
    Hosted,
}

impl Origin {
    /// The file this came from, for [Document::source_path] and for `save()`
    ///
    /// `None` for [Origin::Hosted] too: a hosted document was never on disk,
    /// and `save()` sends it back to the application rather than writing.
    fn path(&self) -> Option<&Path> {
        match self {
            Origin::File { path, .. } => Some(path),
            Origin::None | Origin::Hosted => None,
        }
    }

    /// What could not be read, empty for everything but a file
    fn caveats(&self) -> &BTreeSet<Caveat> {
        static EMPTY: BTreeSet<Caveat> = BTreeSet::new();

        match self {
            Origin::File { caveats, .. } => caveats,
            Origin::None | Origin::Hosted => &EMPTY,
        }
    }
}

/// An open colloscope document
///
/// The document owns its own state, so two documents in one script share
/// nothing and the module holds no global state. Its origin is fixed at
/// creation and never changes: `save(path)` writes where it is told and does
/// not re-target a later `save()`.
///
/// There is no constructor: [load] and [new_document] are the only ways to get
/// one, so `collomatique.Document()` raises `TypeError`.
#[pyclass(module = "collomatique")]
pub struct Document {
    state: SessionStack<Data, Desc>,
    origin: Origin,
    /// The application's own name for the document this one was taken from, and
    /// then for the last one it accepted. `None` when it came from nowhere the
    /// application knows.
    host_token: Mutex<Option<u64>>,
}

/// Opens a colloscope file
///
/// The argument is anything python calls a path: a `str`, a `pathlib.Path`, or
/// any other `os.PathLike`.
#[pyfunction]
pub fn load(path: PathBuf) -> PyResult<Document> {
    let fail = |e: &dyn std::fmt::Display| LoadError::new_err(format!("{}: {e}", path.display()));

    let content = std::fs::read_to_string(&path).map_err(|e| fail(&e))?;

    // `deserialize_data` hands back an `InnerData`; its docs are explicit that
    // the caller owns the invariant gate, as `gtk4`'s file loader does too.
    let (inner_data, caveats) =
        collomatique_storage::deserialize_data(&content).map_err(|e| fail(&e))?;
    let data = Data::from_inner_data(inner_data).map_err(|e| fail(&e))?;

    // Caveats are handed back, not announced: nothing is printed and no
    // `warnings.warn` is raised. The GUI shows a modal because a human is
    // sitting in front of it; a script has nobody, and a library writing to
    // stderr on its own is a nuisance in a cron job. What was skipped was, by
    // construction, something this build cannot use anyway — the loss only
    // happens when the file is written back.
    Ok(Document {
        state: SessionStack::new(data),
        origin: Origin::File { path, caveats },
        host_token: Mutex::new(None),
    })
}

/// An empty document, with no origin
///
/// This is the state of a brand new file. It has nowhere to write, so `save()`
/// with no argument raises `NoOrigin` until it is given a path.
#[pyfunction]
pub fn new_document() -> Document {
    Document {
        state: SessionStack::new(Data::new()),
        origin: Origin::None,
        host_token: Mutex::new(None),
    }
}

/// The document a script should be working on
///
/// Almost every script wants the same three things tried in the same order, so
/// the chain has a name of its own: the document the application is hosting,
/// then `path`, then a file chooser.
///
/// ```python
/// doc = clm.default_document(sys.argv[1] if len(sys.argv) > 1 else None)
/// ```
///
/// The host comes first, and not merely first among equals: a script run inside
/// collomatique must never quietly start editing a file on disk because a stale
/// argument was lying around. It takes a path rather than reading `sys.argv`
/// itself, so a script using `argparse` keeps control of its own command line.
///
/// It raises rather than handing back `None` when there is nothing to open —
/// `Cancelled` for a chooser the user dismissed, `NoDocument` when `dialog` is
/// false and there was no other source. `None` would make every script write an
/// `if doc is None`, and forgetting it gives an obscure `AttributeError` twenty
/// lines further down.
///
/// `dialog=False` is what a cron job passes, where a chooser nobody is watching
/// would wait forever. A machine that cannot show one at all raises
/// `DialogUnavailable` instead, which is the more precise answer: there *was*
/// somewhere left to look.
///
/// A `path` that will not load is an error and not an invitation to the
/// chooser. The chain is a list of sources, not a retry loop: a script that
/// named a file and got the name wrong wants to hear so.
#[pyfunction]
#[pyo3(signature = (path=None, *, dialog=true))]
pub fn default_document(
    py: Python<'_>,
    path: Option<PathBuf>,
    dialog: bool,
) -> PyResult<Py<Document>> {
    if let Some(doc) = crate::host::current_document(py)? {
        return Ok(doc);
    }

    if let Some(path) = path {
        return Py::new(py, load(path)?);
    }

    if !dialog {
        return Err(NoDocument::new_err(
            "there is no document to work on: this script is not running inside \
             collomatique, no path was given, and dialog=False forbids asking for one",
        ));
    }

    // The words the application's own Open dialog uses
    // (`colloscopes/gtk4/src/tools/open_save.rs`), because a user who meets both should
    // meet the same ones.
    let request = FileRequest {
        title: Some("Ouvrir".to_owned()),
        filters: vec![
            (
                "Fichiers collomatique (*.collomatique)".to_owned(),
                vec!["collomatique".to_owned()],
            ),
            ("Tous les fichiers".to_owned(), vec!["*".to_owned()]),
        ],
        directory: None,
        file_name: None,
    };

    match crate::dialogs::ask_open(py, &request)? {
        Some(chosen) => Py::new(py, load(chosen)?),
        None => Err(Cancelled::new_err("no document was chosen to work on")),
    }
}

impl Document {
    /// The document the application handed over
    ///
    /// It carries no caveats: the handoff carries the `Data` and not the
    /// application's caveat set, so a script cannot see that the file behind
    /// it was opened with something missing. Showing that needs a protocol
    /// change.
    pub(crate) fn hosted(data: Data, token: Option<u64>) -> Document {
        Document {
            state: SessionStack::new(data),
            origin: Origin::Hosted,
            host_token: Mutex::new(token),
        }
    }

    /// The document as it is now, for the collections to read through
    pub(crate) fn data(&self) -> &Data {
        self.state.get_data()
    }

    /// The token to send along with this document
    pub(crate) fn host_token(&self) -> Option<u64> {
        *self.host_token.lock().unwrap()
    }

    /// Keeps what the application answered a send with, so the next one speaks
    /// of the document it now holds
    pub(crate) fn set_host_token(&self, token: Option<u64>) {
        *self.host_token.lock().unwrap() = token;
    }

    /// Applies one composite op, and keeps the repairs it had to make
    ///
    /// The single door every mutator goes through. It is `dry_apply` rather
    /// than `apply`: `apply` exists for callers with no way of showing the
    /// cascade's repairs, and this api has one — every mutator hands them back
    /// on its [OpResult].
    ///
    /// Public on the rust side, and python-facing only through the mutators
    /// that call it: the ops layer holds every write, but the python surface
    /// publishes them one family at a time. The tests use it directly, because
    /// staleness needs a removal the read surface cannot make yet.
    ///
    /// A refusal comes back as the exception class of the family that refused
    /// — a `collomatique.GeneralPlanningError` and its fourteen siblings, all
    /// under `collomatique.UpdateError` — carrying the op, the case and the
    /// entities the model named ([crate::errors::refused]).
    pub fn update(&mut self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let (warnings, _created) = self.write(py, op)?;

        Ok(OpResult::new(warnings))
    }

    /// Applies one composite op, keeping both halves of what it answered
    ///
    /// The repairs the cascade made, and the id the op issued when it created
    /// something. [Document::update] is the door for a mutator that creates
    /// nothing, and it drops the second half; a creating one goes through
    /// [crate::results::created], which turns it into the handle its
    /// `AddResult` carries.
    pub(crate) fn write(
        &mut self,
        py: Python<'_>,
        op: UpdateOp,
    ) -> PyResult<(Vec<Py<Warning>>, Option<NewId>)> {
        let result = op
            .dry_apply(&self.state)
            .map_err(|e| crate::errors::refused(py, &e))?;

        // Built here, while `self.state` is still the state the op was applied
        // to: a warning names material this update may be about to remove, so
        // the pre-state is the only one it can be read against
        // (`colloscopes/ops/src/cascade.rs`).
        let warnings = crate::results::from_cascade(py, &result.warnings, self.state.get_data())?;

        self.state = result.new_state;

        Ok((warnings, result.new_id))
    }

    /// Opens a transaction on the document
    ///
    /// [Transaction] is the only caller: a session that nothing holds is a
    /// session nothing can close, so opening one is not part of the python
    /// surface.
    pub(crate) fn begin_transaction(&mut self) {
        self.state.begin();
    }

    /// Closes the innermost transaction, keeping what it wrote as one step
    pub(crate) fn commit_transaction(&mut self, desc: Desc) {
        let committed = self.state.commit(desc);
        debug_assert!(
            committed,
            "a Transaction only ever closes a session it opened"
        );
    }

    /// Closes the innermost transaction, unwinding everything it wrote
    pub(crate) fn cancel_transaction(&mut self) {
        let cancelled = self.state.cancel();
        debug_assert!(
            cancelled,
            "a Transaction only ever closes a session it opened"
        );
    }

    /// How many transactions are open, for the innermost-first guard
    pub(crate) fn transaction_depth(&self) -> usize {
        self.state.depth()
    }
}

#[pymethods]
impl Document {
    /// The periods of the document, and the date the colles start
    ///
    /// A view on the document, not a copy: `doc.periods` twice gives two
    /// objects that read and write the same document.
    #[getter]
    fn periods(slf: Py<Self>) -> Periods {
        Periods::new(slf)
    }

    /// Every week of the document, in global week order
    ///
    /// Period display order, then position within the period — the order
    /// `week.index` counts in. The weeks of one period alone are `period.weeks`.
    #[getter]
    fn weeks(slf: Py<Self>) -> Weeks {
        Weeks::new(slf)
    }

    /// The subjects of the document, in user order
    ///
    /// The order the application shows them in, which is data of its own:
    /// `subject.index` is the position in it.
    #[getter]
    fn subjects(slf: Py<Self>) -> Subjects {
        Subjects::new(slf)
    }

    /// The teachers of the document, in id order
    ///
    /// The model keeps no display order for them — the application sorts them
    /// by name as it shows them — so there is no `.index` to read here, and no
    /// user order to iterate in.
    #[getter]
    fn teachers(slf: Py<Self>) -> Teachers {
        Teachers::new(slf)
    }

    /// The students of the document, in id order
    ///
    /// In id order for the reason the teachers are. What a student carries is
    /// their card and the periods they sit out; which subjects they take is
    /// kept apart from them, in a junction table keyed by period and subject.
    #[getter]
    fn students(slf: Py<Self>) -> Students {
        Students::new(slf)
    }

    /// The week patterns of the document, in id order
    ///
    /// A pattern is « les semaines paires » — the set of weeks a slot carrying
    /// it does *not* hold its colles on. Whether a week ends up carrying an
    /// interrogation is `is_week_active`'s question, since the week has a say of
    /// its own.
    #[getter]
    fn week_patterns(slf: Py<Self>) -> WeekPatterns {
        WeekPatterns::new(slf)
    }

    /// Whether colles can happen on this week, under this pattern
    ///
    /// ```python
    /// weeks = [week for week in doc.weeks if doc.is_week_active(week, pattern)]
    /// ```
    ///
    /// Two things switch a week off, and this is where they are put back
    /// together: the week's own `interrogations` flag — the week of the
    /// « Rentrée » holds no colles for anyone — and the pattern's exception set,
    /// which switches off the weeks that pattern names on top of that. Neither
    /// handle can answer alone, so the question lives on the document.
    ///
    /// `pattern=None` asks about no pattern at all, which is what a slot without
    /// one means: only the week's own flag counts. Both arguments take a handle
    /// or an id, as every argument of this api does.
    ///
    /// A `week` or a `pattern` this document does not hold raises
    /// `StaleHandleError` rather than answering `False`: the model shrugs a
    /// forgiving `false` at a week it never heard of, and a script that asked
    /// about a removed week would read that as « no colles that week » when what
    /// happened is that it lost track of its own document.
    #[pyo3(signature = (week, pattern=None))]
    fn is_week_active(
        slf: Py<Self>,
        py: Python<'_>,
        week: &Bound<'_, PyAny>,
        pattern: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let week_id = crate::handles::argument::<Week>(&slf, week)?;
        let pattern_id = pattern
            .map(|pattern| crate::handles::argument::<WeekPattern>(&slf, pattern))
            .transpose()?;

        // The model's own definition, and not a second copy of it: the gui grid
        // and the constraints layer ask `Parameters::is_week_active` the same
        // question, and an api that answered differently would be wrong about
        // the document it is showing.
        let doc = slf.borrow(py);
        Ok(doc
            .data()
            .get_inner_data()
            .params
            .is_week_active(week_id, pattern_id))
    }

    /// Every slot of the document, in subject-then-position order
    ///
    /// The subjects in the order `doc.subjects` shows them, each followed by its
    /// own slots in theirs — so this walk is `doc.subjects` and `subject.slots`
    /// laid end to end, and the two never disagree. The slots of one subject
    /// alone are `subject.slots`, and `slot.index` is the position in that
    /// shorter list.
    #[getter]
    fn slots(slf: Py<Self>) -> Slots {
        Slots::new(slf)
    }

    /// Which students take which subject in which period
    ///
    /// The junction table, reached through the `(period, subject)` pairs that
    /// key it:
    ///
    /// ```python
    /// doc.assignments[period, subject]   # frozenset[Student], possibly empty
    /// for period, subject, students in doc.assignments:
    ///     ...
    /// ```
    ///
    /// Its reads are total: an absent row is the empty frozenset, never a
    /// `KeyError`. There is no `len`, no `in`, no `.get` — over a total
    /// mapping, row count and row membership are statements about the model's
    /// storage, not about the data. A `period` or `subject` this document does
    /// not hold raises `StaleHandleError`, because the address was malformed
    /// before it had an answer.
    #[getter]
    fn assignments(slf: Py<Self>) -> Assignments {
        Assignments::new(slf)
    }

    /// The schedule incompatibilities of the document
    ///
    /// Reached as `doc.incompats`, in id order:
    ///
    /// ```python
    /// for incompat in doc.incompats:
    ///     for slot in incompat.slots:
    ///         ...
    /// ```
    ///
    /// An incompatibility says when the students of a subject may be
    /// unavailable: the busy windows of its list, at least `minimum_free_slots`
    /// of which must stay free. The subject is deliberately not required to run
    /// colles of its own.
    #[getter]
    fn incompats(slf: Py<Self>) -> Incompats {
        Incompats::new(slf)
    }

    /// The group lists of the document, and which subjects they serve
    ///
    /// Reached as `doc.group_lists`, in id order:
    ///
    /// ```python
    /// for group_list in doc.group_lists:
    ///     for number in range(group_list.group_count):
    ///         print(group_list.group_name(number))
    /// ```
    ///
    /// A group list is either prefilled — its groups are fixed sets of students
    /// — or automatic, filled by the solver, whose placements live in the
    /// colloscope. The `(period, subject) → group list` hop a colloscope cell
    /// needs is here too: `association_for`, and `associations()` for the whole
    /// table.
    #[getter]
    fn group_lists(slf: Py<Self>) -> GroupLists {
        GroupLists::new(slf)
    }

    /// The pairing rules of the document, in id order
    ///
    /// Reached as `doc.pairings`:
    ///
    /// ```python
    /// for rule in doc.pairings:
    ///     if rule.soft:
    ///         ...
    /// ```
    ///
    /// A pairing rule is an implication between two subjects: a student who
    /// `should_have` the antecedent subject's interrogation in a week should
    /// (or should not) have the consequent's that week. The two ends are the
    /// `rule.antecedent` / `rule.consequent` sub-views, which go stale with
    /// the rule.
    #[getter]
    fn pairings(slf: Py<Self>) -> Pairings {
        Pairings::new(slf)
    }

    /// The slot pairing rules of the document, in id order
    ///
    /// Reached as `doc.slot_pairings`:
    ///
    /// ```python
    /// for rule in doc.slot_pairings:
    ///     print(rule.antecedent.slot.teacher.surname)
    /// ```
    ///
    /// The slots' version of a pairing rule: if the antecedent slot is used in
    /// a week, the consequent slot must also be used — or not. Both slots of a
    /// rule belong to the same subject.
    #[getter]
    fn slot_pairings(slf: Py<Self>) -> SlotPairings {
        SlotPairings::new(slf)
    }

    /// The limits the resolution is held to, global and per student
    ///
    /// Reached as `doc.settings`:
    ///
    /// ```python
    /// limits = doc.settings.limits_for(student)
    /// ```
    ///
    /// One global entry plus sparse per-student overrides, with the whole-entry
    /// resolution kept in the model: `limits_for` hands back the student's
    /// override verbatim when there is one, the global entry otherwise — a
    /// `None` field in an override *disables* the corresponding global limit,
    /// it does not inherit it. The [Limits] views read the current state on
    /// every access; a resolved view tracks an override appearing or
    /// vanishing, and an override view goes stale with its entry.
    #[getter]
    fn settings(slf: Py<Self>) -> Settings {
        Settings::new(slf)
    }

    /// How the resolution balances the interrogations, global and per subject
    ///
    /// Reached as `doc.balancing`:
    ///
    /// ```python
    /// if doc.balancing.options_for(subject).teacher_rotation == clm.Enforcement.STRICT:
    ///     ...
    /// ```
    ///
    /// The structural twin of `doc.settings`: one global entry plus sparse
    /// per-subject overrides, whole-entry resolution in the model, and
    /// [BalancingOptions] views reading the current state — the rotation goals
    /// as `Enforcement | None` (`None` = not pursued, `OBJECTIVE` = optimize
    /// for it, `STRICT` = a hard constraint) and the two fairness switches.
    #[getter]
    fn balancing(slf: Py<Self>) -> Balancing {
        Balancing::new(slf)
    }

    /// What a resolution found: the cells and the group lists it filled
    ///
    /// Reached as `doc.colloscope`:
    ///
    /// ```python
    /// groups = doc.colloscope.interrogation(slot, week)   # frozenset[int] | None
    /// for slot, week, groups in doc.colloscope.interrogations():
    ///     ...
    /// placements = doc.colloscope.group_list(group_list)  # Mapping[Student, int] | None
    /// ```
    ///
    /// The result of the last resolution, in two sparse tables: which group
    /// numbers sit in which `(slot, week)` cell — numbers, because a group
    /// number names a group of the list the cell's subject uses on that
    /// week's period, the hop being `doc.group_lists.association_for` — and
    /// how each automatic group list was filled. An absent cell is `None`,
    /// the one thing an empty table and a missing one have in common;
    /// whether a cell *could* hold anything is `is_interrogation_possible`'s
    /// question, so the two reads pair.
    ///
    /// The view is read-only: nothing here mutates, and the placements
    /// mappings cannot be written to. All of that is the write surface's
    /// business.
    #[getter]
    fn colloscope(slf: Py<Self>) -> Colloscope {
        Colloscope::new(slf)
    }

    /// The presentation preferences of the xlsx export
    ///
    /// Reached as `doc.export_config`:
    ///
    /// ```python
    /// if doc.export_config.colloscope_enabled:
    ///     print(doc.export_config.colloscope_config.sheet_name)
    /// ```
    ///
    /// One atom of pure value data, held the way the model holds it: a global
    /// section, four per-sheet sections, and the enabled flag that gates each
    /// of them — the flags sit beside the sections, not inside them, because a
    /// flag is the interface's memory of what was chosen before a section was
    /// switched off. Everything reads as [Color] and [Orientation] values, the
    /// whole tree is read-only, and nothing in it can go stale.
    ///
    /// [Color]: crate::values::Color
    /// [Orientation]: crate::values::Orientation
    #[getter]
    fn export_config(slf: Py<Self>) -> ExportConfig {
        ExportConfig::new(slf)
    }

    /// Whether a colle can happen in this slot on this week
    ///
    /// ```python
    /// cells = [week for week in doc.weeks if doc.is_interrogation_possible(slot, week)]
    /// ```
    ///
    /// True exactly when the application would draw that cell in its grid.
    /// Three things have to hold at once: the slot's subject runs colles, it
    /// does not skip that week's period, and the week is active under the slot's
    /// pattern. No handle can answer alone — the question joins slots, subjects,
    /// periods and patterns — so it lives on the document, next to
    /// `is_week_active`.
    ///
    /// Both arguments take a handle or an id. A `slot` or a `week` this document
    /// does not hold raises `StaleHandleError` rather than answering `False`,
    /// for the reason `is_week_active` gives: the model shrugs a forgiving
    /// `false` at a reference it never heard of, and a script would read that as
    /// « no colle there » when what happened is that it lost track of its own
    /// document.
    fn is_interrogation_possible(
        slf: Py<Self>,
        py: Python<'_>,
        slot: &Bound<'_, PyAny>,
        week: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let slot_id = crate::handles::argument::<Slot>(&slf, slot)?;
        let week_id = crate::handles::argument::<Week>(&slf, week)?;

        // The model's own definition, and not a second copy of it: the gui grid,
        // the constraints layer and the storage decoder all ask
        // `Parameters::is_interrogation_possible` the same question, and an api
        // that answered differently would be wrong about the document it is
        // showing.
        let doc = slf.borrow(py);
        Ok(doc
            .data()
            .get_inner_data()
            .params
            .is_interrogation_possible(slot_id, week_id))
    }

    /// The whole document, detached — every section in one tree
    ///
    /// ```python
    /// tree = doc.snapshot()    # DocumentData
    /// ```
    ///
    /// The same conversion `to_data()` is, run over everything at once: a
    /// `DocumentData` mirroring the document section by section, the params
    /// tables as dicts and lists whose order is the document's own user order,
    /// the colloscope and the export configuration as values of their own.
    /// The ids in it are the document's, so the tree is a read-modify-write
    /// starting point — rename, delete, rewire — and `replace_all` takes it
    /// back.
    ///
    /// A pure read: it borrows the document, walks its data and builds the
    /// tree, so it cannot fail on a document that exists. A script that wants
    /// one section still calls the handle's own `to_data()`.
    fn snapshot<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Copied out of the borrow before anything python-facing happens:
        // building the tree calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let inner = self.state.get_data().get_inner_data().clone();

        crate::data::DocumentData::to_py(py, &inner)
    }

    /// Puts a whole tree back, as one step
    ///
    /// ```python
    /// tree = doc.snapshot()
    /// ...                                  # arbitrary transformation
    /// doc.replace_all(tree, "Rebuilt from scratch")
    /// ```
    ///
    /// The coarse door, and the way back in for what `snapshot()` hands out:
    /// one `GlobalUpdate`, one undo slot, whatever the tree changed. `label`
    /// names that slot and defaults to « Mise à jour globale », the name the
    /// application's own global updates carry.
    ///
    /// A tree can rename, delete and rewire, and it cannot **add**: it names
    /// its entities by id, every id in it has to be one this document holds —
    /// an id it does not raises `StaleHandleError`, as everywhere else — and
    /// ids have no constructor. Creating something is the incremental ops'
    /// business, and it stays theirs.
    ///
    /// A refused tree changes nothing at all: the document is left exactly as
    /// it was, and the `UpdateError` names every invariant the tree broke, not
    /// just the first. The realistic case is a reference left dangling — a
    /// tree that drops a subject but keeps its slots lists every one of those
    /// slots.
    ///
    /// The answer is an `OpResult` whose `warnings` is always empty: unlike an
    /// incremental op, a global update has nothing to repair. It lands as
    /// given or it is refused whole.
    #[pyo3(signature = (tree, label=None))]
    fn replace_all(
        slf: Py<Self>,
        py: Python<'_>,
        tree: &Bound<'_, PyAny>,
        label: Option<String>,
    ) -> PyResult<OpResult> {
        use crate::data::Value as _;

        // Extracted before the borrow below and never inside it: resolving the
        // ids a tree names borrows the document to ask, and doing that under a
        // `borrow_mut` is how a nested borrow becomes a `PanicException`.
        let inner = crate::data::DocumentData::from_py(&slf, tree)?;

        let desc = (
            collomatique_ops::OpCategory::None,
            label.unwrap_or_else(|| String::from("Mise à jour globale")),
        );

        // `apply` and not the `write` funnel: `GlobalUpdate` is not an
        // `UpdateOp` — it goes in below the ops layer, at the model's own
        // trust boundary, which is where a whole tree is checked. So there is
        // no cascade to hand back, and no repairs either.
        let mut doc = slf.borrow_mut(py);
        doc.state
            .apply(Op::GlobalUpdate(inner), desc)
            .map_err(|e| UpdateError::new_err(e.to_string()))?;

        Ok(OpResult::new(Vec::new()))
    }

    /// Groups every write in a block into one undo slot
    ///
    /// ```python
    /// with doc.transaction("Import Pronote"):
    ///     ...
    /// ```
    ///
    /// The block leaving normally commits everything it wrote as a single step
    /// named `label`; an exception rolls the whole block back and propagates.
    ///
    /// Blocks nest, and really nest: an inner block that rolls back takes only
    /// its own writes with it, so a helper that opens a transaction is safe to
    /// call from inside another one. Inside a block, `undo()` reaches back no
    /// further than the block's start.
    ///
    /// The object it returns does nothing until it is entered, and a `with` is
    /// the only sensible way to enter one.
    fn transaction(slf: Py<Self>, label: String) -> Transaction {
        Transaction::new(slf, label)
    }

    /// Where this document came from, as a `pathlib.Path`, or `None`
    ///
    /// `None` means the document was never on disk — it came from
    /// [new_document]. Saving it once does not give it an origin: the origin
    /// is where the document *came from*, not where it was last written.
    /// pyo3 converts a rust path to a `pathlib.Path`, so nothing is built by
    /// hand here — but the script asserts the `isinstance`, because it is a
    /// promise and not an implementation detail we happen to inherit.
    #[getter]
    fn source_path(&self) -> Option<&Path> {
        self.origin.path()
    }

    /// Whether this is the document the application handed over
    ///
    /// True only for what [crate::host::current_document] returned. A copy of
    /// it is not hosted — not `compacted()`'s, and not one built by loading
    /// the same file — because being hosted is about which document the
    /// application is showing, not about what is in it.
    #[getter]
    fn is_hosted(&self) -> bool {
        matches!(self.origin, Origin::Hosted)
    }

    /// What could not be read from the file, as a `frozenset` of [Caveat]s
    ///
    /// Empty for a clean file and for [new_document]. It is part of the
    /// origin, so it is fixed at creation like the path is: writing a copy of
    /// the document does not make the file it came from any more readable, so
    /// nothing clears it. (The GUI *does* clear its `FileName::CaveatFile`
    /// after a save, but that field is a save *target*, which is a different
    /// thing from an origin.)
    ///
    /// Each element says what was dropped; `str()` on one is a french sentence,
    /// the same one the application's caveat dialog writes, and the classes
    /// are in the module, so a script can test for the caveat it knows how to
    /// handle:
    ///
    /// ```python
    /// if clm.UnknownEntry("colloscope", 3) in doc.caveats:
    ///     ...
    /// ```
    #[getter]
    fn caveats<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyFrozenSet>> {
        let caveats = self
            .origin
            .caveats()
            .iter()
            .map(|caveat| crate::caveats::to_python(py, caveat))
            .collect::<PyResult<Vec<_>>>()?;
        PyFrozenSet::new(py, &caveats)
    }

    /// Takes back the last write
    ///
    /// Raises `NothingToUndo` when there is nothing left to take back, rather
    /// than doing nothing quietly: a script that undoes more than it wrote is
    /// mistaken about its own document, and silence would hide that.
    ///
    /// The history belongs to the document and never leaves the script. In
    /// hosted mode the script works on a copy, so an undo is invisible to the
    /// application: only `send_to_host` crosses back, and what it carries is
    /// the document as it stands, not the way it got there.
    fn undo(&mut self) -> PyResult<()> {
        self.state
            .undo()
            .map_err(|_| NothingToUndo::new_err("this document has nothing left to undo"))?;
        Ok(())
    }

    /// Puts back the last write [Document::undo] took away
    ///
    /// Raises `NothingToUndo` when there is nothing to put back. A new write
    /// empties the redo stack, as it does everywhere else: the history is a
    /// line, not a tree.
    fn redo(&mut self) -> PyResult<()> {
        self.state
            .redo()
            .map_err(|_| NothingToUndo::new_err("this document has nothing left to redo"))?;
        Ok(())
    }

    /// Whether [Document::undo] would do something rather than raise
    #[getter]
    fn can_undo(&self) -> bool {
        self.state.can_undo()
    }

    /// Whether [Document::redo] would do something rather than raise
    #[getter]
    fn can_redo(&self) -> bool {
        self.state.can_redo()
    }

    /// What [Document::undo] would take back, in french, or `None`
    ///
    /// The same short label the application shows in its Undo menu — « Changer
    /// le début des colles » and the like. It is a label for a human to read,
    /// so a script should not branch on it; `can_undo` is the question with a
    /// stable answer.
    ///
    /// The category the operation also carries is not exposed: it tells the
    /// GUI which screen to open, which is nothing a script can use.
    #[getter]
    fn undo_name(&self) -> Option<String> {
        self.state.get_undo_name().map(|(_, name)| name.clone())
    }

    /// What [Document::redo] would put back, in french, or `None`
    #[getter]
    fn redo_name(&self) -> Option<String> {
        self.state.get_redo_name().map(|(_, name)| name.clone())
    }

    /// A copy of the document with dense ids and no undo history
    ///
    /// The document itself is untouched: the copy is a new document, so
    /// nothing a script holds is invalidated, and "this clears the undo
    /// history" is not a warning — a new document simply has none.
    ///
    /// This is the way out of `IdCeilingExceeded`. The file format has a
    /// ceiling on the ids it can write and `save` never renumbers on its own,
    /// so the rescue is explicit:
    ///
    /// ```python
    /// clm.load("big_ids.collomatique").compacted().save()
    /// ```
    ///
    /// The copy inherits the file it came from, and with it the caveats — the
    /// script above overwrites the file it read, and a caveated file still
    /// refuses the bare `save()`, so compaction is not a laundering route
    /// around that guard.
    fn compacted(&self) -> PyResult<Document> {
        let inner_data = self.state.get_data().get_inner_data().clone().compact_ids();

        // Renumbering is injective and monotone, so it repairs nothing and
        // breaks nothing: a document that satisfied the invariants still does
        // (`colloscopes/state-colloscopes/src/compact.rs`, pinned by
        // `colloscopes/state-colloscopes/tests/compact_ids.rs`). The arm is still written
        // out rather than unwrapped, because a script never gets a panic.
        let data = Data::from_inner_data(inner_data).map_err(|e| {
            Error::new_err(format!(
                "compacting the document produced an invalid one: {e}"
            ))
        })?;

        Ok(Document {
            state: SessionStack::new(data),
            origin: match &self.origin {
                Origin::None => Origin::None,
                Origin::File { path, caveats } => Origin::File {
                    path: path.clone(),
                    caveats: caveats.clone(),
                },
                // Compaction cannot travel to the host: an `Op::GlobalUpdate`
                // only pushes the id issuer forward, so the application would
                // end up with dense ids, an issuer still at its old high-water
                // mark, and a Ctrl-Z bringing the big ids back. So a compacted
                // copy of the hosted document is an ordinary origin-less one.
                Origin::Hosted => Origin::None,
            },
            host_token: Mutex::new(None),
        })
    }

    /// Writes the document out
    ///
    /// With a path, writes that file. Without one, it goes where the document
    /// came from: back to the application for the hosted document — the same
    /// thing `send_to_host` does, and just as loud about replacing what the
    /// application holds — and to its file for a loaded one. A document with
    /// neither raises `NoOrigin`: it is never a silent no-op. The origin does
    /// not move — `save(other)` does not re-target a later `save()`.
    ///
    /// Writing back over a file that was loaded with caveats raises
    /// `CaveatedOverwrite` instead, because whatever could not be read is
    /// dropped by the rewrite. The four forms are:
    ///
    /// ```python
    /// doc.save()                      # raises, when doc.caveats is non-empty
    /// doc.save("copy.collomatique")   # writes; the suspect original survives
    /// doc.save(doc.source_path)       # writes; the script named the target
    /// doc.save(ignore_caveats=True)   # writes; deliberate
    /// ```
    ///
    /// The rule keys on *no path argument*, not on whether the path equals the
    /// origin. Path equality is fragile — symlinks, relative paths, hard links
    /// — and the GUI does not test it either: its "Enregistrer" on a caveated
    /// file opens a Save-As dialog defaulting to that same file, and a user who
    /// picks it overwrites it, because they chose. Naming the path from python
    /// is that same choice. `save()` with no argument is the one form that
    /// writes somewhere the script never named, so it is the one that is loud.
    ///
    /// `ignore_caveats` does nothing when a path is given, since that form
    /// never raises; it is accepted there so a script can pass it uniformly.
    #[pyo3(signature = (path=None, *, ignore_caveats=false))]
    fn save(&self, py: Python<'_>, path: Option<PathBuf>, ignore_caveats: bool) -> PyResult<()> {
        let target = match path {
            Some(path) => path,
            None => match &self.origin {
                // The hosted document has no file to write, and `ignore_caveats`
                // has nothing to say about it: it carries no caveats.
                Origin::Hosted => return crate::host::send_to_host(py, self),
                Origin::None => {
                    return Err(NoOrigin::new_err(
                        "this document has no origin: pass a path to save()",
                    ));
                }
                Origin::File { path, caveats } => {
                    if !ignore_caveats && !caveats.is_empty() {
                        return Err(CaveatedOverwrite::new_err(format!(
                            "{}: this file was loaded with caveats, so part of it could not be \
                             read and writing back would drop it ({}); pass a path to write \
                             elsewhere, or ignore_caveats=True to overwrite it anyway",
                            path.display(),
                            caveats
                                .iter()
                                .map(collomatique_ui_text::caveats::caveat_text)
                                .collect::<Vec<_>>()
                                .join("; "),
                        )));
                    }
                    path.clone()
                }
            },
        };

        let content = collomatique_storage::serialize_data(self.state.get_data().get_inner_data())
            .map_err(|collomatique_storage::EncodeError::IdAboveCeiling { id }| {
                IdCeilingExceeded::new_err(format!(
                    "id {id} exceeds the file-format ceiling; the document must be \
                     compacted before it can be written"
                ))
            })?;

        std::fs::write(&target, content)
            .map_err(|e| SaveError::new_err(format!("{}: {e}", target.display())))
    }

    /// Writes the colloscope out as a spreadsheet
    ///
    /// ```python
    /// doc.export_xlsx("colloscope.xlsx")
    ///
    /// config = doc.export_config.to_data()
    /// config.per_group_list_enabled = False
    /// doc.export_xlsx("colles.xlsx", config)
    /// ```
    ///
    /// The same workbook the application's own export produces, built by the
    /// same writer: which sheets it holds, how they are coloured and how they
    /// are laid out on the page all come from an export configuration. With no
    /// `config`, the document's own is used — the one `doc.export_config`
    /// reads and its mutators write. With one, that one is used *for this call
    /// only*: nothing is stored, and the document is not written to at all, so
    /// an export takes no undo slot.
    ///
    /// `config` is an `ExportConfigData`, the tree `doc.export_config.to_data()`
    /// hands out, so the usual way to build one is to take that tree and change
    /// what should differ.
    ///
    /// This is not `save()`: it writes a spreadsheet for people to read, and
    /// nothing reads one back. A document is saved with `save()`.
    ///
    /// A path that cannot be written, and a workbook that cannot be built,
    /// both raise `ExportError`.
    #[pyo3(signature = (path, config=None))]
    fn export_xlsx(
        slf: Py<Self>,
        py: Python<'_>,
        path: PathBuf,
        config: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        use crate::data::Value as _;

        // Extracted before the borrow below and never inside it: reading a
        // python object calls back into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let given = match config {
            Some(config) => Some(crate::data::ExportConfigData::from_py(&slf, config)?),
            None => None,
        };

        // Copied out of the borrow, both of them: the write below runs with the
        // GIL released, and it must not hold the document while another thread
        // could be handed it.
        let (inner, raw_config) = {
            let doc = slf.borrow(py);
            let inner = doc.data().get_inner_data();
            let raw_config = given.unwrap_or_else(|| inner.export_config.clone());
            (inner.clone(), raw_config)
        };

        let xlsx_config = collomatique_xlsx::Config::from(&raw_config);

        // Released for the duration: building a workbook out of a whole
        // colloscope and writing it is long enough to be worth not blocking
        // the interpreter over (`host.rs`, `dialogs.rs`).
        py.detach(|| collomatique_xlsx::write_xlsx(&inner, &path, &xlsx_config))
            .map_err(|e| ExportError::new_err(format!("{}: {e}", path.display())))
    }

    /// Builds the ILP problem a solve would attack
    ///
    /// ```python
    /// config = collomatique.ColloscopeSolveConfig()
    /// model = doc.build_colloscope_model(config)
    ///
    /// model = doc.build_colloscope_model(config, on_log=print)
    /// ```
    ///
    /// `config` says which periods and which group lists are recomputed, and
    /// what a dropped constraint or a previous value is worth as an objective
    /// term — a `ColloscopeSolveConfig`, the same vocabulary the application's
    /// own solve dialog fills in. It is required: the application never solves
    /// without passing that dialog, and a script that wants everything
    /// recomputed writes `ColloscopeSolveConfig()` and says so.
    ///
    /// What comes back is a `ColloscopeModel` — a token for a built problem,
    /// not a view of one. It is detached, like a value: a snapshot of the
    /// document as it stands now, which later edits neither change nor
    /// invalidate. `model.export_mps(path)` writes it out for a solver to read.
    ///
    /// `on_log` is called with one line of the build log at a time, the log the
    /// application shows while it builds; `None` discards it. There is no
    /// progress here — a build has lines, not a proportion. A callback that
    /// raises does not tear the build in half: the build runs to its end, the
    /// callback is not called again, and the exception comes out of this call
    /// with no model built.
    ///
    /// Nothing is written to the document, so a build takes no undo slot. A
    /// problem the builder refuses to assemble raises `ModelBuildError`.
    #[pyo3(signature = (config, *, on_log=None))]
    fn build_colloscope_model(
        slf: Py<Self>,
        py: Python<'_>,
        config: &Bound<'_, PyAny>,
        on_log: Option<Py<PyAny>>,
    ) -> PyResult<ColloscopeModel> {
        use crate::data::Value as _;

        // Extracted before the borrow below and never inside it, like
        // `export_xlsx`'s configuration and for the same reason.
        let config = crate::data::ColloscopeSolveConfig::from_py(&slf, config)?;

        // Copied out of the borrow: the build below runs with the GIL
        // released, and it must not hold the document while another thread
        // could be handed it.
        let inner = {
            let doc = slf.borrow(py);
            doc.data().get_inner_data().clone()
        };

        // What the log callback raised, if it raised. The builder takes an
        // infallible `FnMut`, so a raising callback cannot stop the build
        // half-way through — the first exception is kept here, the callback is
        // not called again, and the build is left to finish.
        let mut failure: Option<PyErr> = None;
        let mut log = |line: &str| {
            let Some(callback) = on_log.as_ref() else {
                return;
            };
            if failure.is_some() {
                return;
            }

            // The GIL is released for the whole build, so each line takes it
            // back for the length of one call and gives it up again.
            Python::attach(|py| {
                if let Err(error) = callback.call1(py, (line,)) {
                    failure = Some(error);
                }
            });
        };

        // Released for the duration: building the model of a whole colloscope
        // is the longest thing this module does, and a script that watches it
        // through `on_log` is watching a build that is really running.
        let built = py.detach(|| config.build_model(&inner, &mut log));

        // The callback's exception wins over whatever the build made of it:
        // the script asked for the lines and one of them was refused, so no
        // model is handed back.
        if let Some(failure) = failure {
            return Err(failure);
        }

        // The parameters go with the model: they are the half of this snapshot
        // a solution is read against, and `inner` is this call's own copy — the
        // borrow that made it ended long ago, so moving them out is free.
        built
            .map(|model| ColloscopeModel::new(model, inner.params))
            .map_err(ModelBuildError::new_err)
    }

    /// The generation request the application's own dialog opens with
    ///
    /// ```python
    /// req = doc.default_generation_request()
    /// result = doc.generate_group_lists(req)
    /// ```
    ///
    /// `rebuild` holds every `(period, subject)` pair that could take a group
    /// list and has none — the subject runs interrogations, it is not excluded
    /// from the period, and nothing is associated to the pair yet — and
    /// `kept_lists` holds every prefilled list, as a stability anchor. It is
    /// built by the same function the application's generation dialog fills
    /// its own switches from, so a script and a click start from one
    /// selection.
    ///
    /// A plain `GroupListsGenerationRequest`, fresh every call and holding
    /// ids: edit it, or ignore it and build one from nothing.
    ///
    /// It says nothing about what will work. A pair whose students the group
    /// sizes cannot split is defaulted on here just as the dialog shows it —
    /// where the user must clear it before « Valider » lights up, and where a
    /// script meets it as `generate_group_lists`'s refusal.
    fn default_generation_request<'py>(
        slf: Py<Self>,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        let request = {
            let doc = slf.borrow(py);
            collomatique_greedy_groups::default_generation_request(
                &doc.data().get_inner_data().params,
            )
        };

        // Built after the borrow ended: making the value calls back into
        // python, which must not happen while the document is held.
        crate::data::GroupListsGenerationRequest::to_py(py, &request)
    }

    /// Builds the group lists a request asks for
    ///
    /// ```python
    /// result = doc.generate_group_lists(doc.default_generation_request())
    /// doc.group_lists.add_generated(result.entries)
    /// ```
    ///
    /// The application's own generation, run from here: the same generator on
    /// the same document produces the same lists. It is fast — milliseconds on
    /// a whole class — and synchronous, so there is no run to wait on and
    /// nothing to stop.
    ///
    /// Nothing is written and no undo slot is taken. What comes back is a
    /// `GroupListsGenerationResult`, and `doc.group_lists.add_generated` is
    /// the door that lands it; a result nobody lands changes nothing.
    ///
    /// There is one entry per distinct list, not one per requested pair: two
    /// pairs whose students and group-size range agree get a single list
    /// between them, covering both. Each list is named after what it covers —
    /// « Sortilèges (période 1) », the label the application's naming dialog
    /// starts from — and a script renames one by editing `.name` on the value
    /// before landing it.
    ///
    /// `on_log` is called with one line of the generator's log at a time;
    /// `None` discards it. A callback that raises does not tear the
    /// generation in half: it runs to its end, the callback is not called
    /// again, and the exception comes out of this call with no result handed
    /// back.
    ///
    /// A request that asks for a pair nobody is registered for is not an
    /// error: no list is built for it, and the pair comes back in
    /// `result.skipped`. What *is* refused, with `GroupListsGenerationError`:
    /// a pair whose subject runs no interrogations, a kept list that is not
    /// prefilled, and a pair whose students the subject's group sizes cannot
    /// split — this last one reachable straight from
    /// `default_generation_request()`, which offers it exactly as the dialog
    /// does. A reference to something the document does not hold is refused
    /// earlier still, with `StaleHandleError`.
    #[pyo3(signature = (request, *, on_log=None))]
    fn generate_group_lists(
        slf: Py<Self>,
        py: Python<'_>,
        request: &Bound<'_, PyAny>,
        on_log: Option<Py<PyAny>>,
    ) -> PyResult<Py<crate::generation::GroupListsGenerationResult>> {
        use crate::data::Value as _;

        // Extracted before the borrow below and never inside it — and it
        // is the extraction that refuses dead references, so the plan below
        // can only fail on what the request asks for, never on what it names.
        let request = crate::data::GroupListsGenerationRequest::from_py(&slf, request)?;

        // Copied out of the borrow: the generator runs with the GIL released,
        // and it must not hold the document while another thread could be
        // handed it.
        let params = {
            let doc = slf.borrow(py);
            doc.data().get_inner_data().params.clone()
        };

        let plan = collomatique_greedy_groups::build_generation_plan(&params, &request)
            .map_err(|e| GroupListsGenerationError::new_err(e.to_string()))?;

        // The coverage labels, which is what the application's naming dialog
        // seeds its rows with — so the lists a script lands unrenamed are
        // named exactly as a click would have named them. One per spec is also
        // what the generator asks for.
        let names: Vec<String> = plan
            .specs
            .iter()
            .map(|(_spec, covered)| {
                collomatique_ui_text::rendering::coverage_label(
                    &params.periods,
                    &params.subjects,
                    covered,
                )
            })
            .collect();

        // What the log callback raised, if it raised. The generator takes an
        // infallible `FnMut`, so a raising callback cannot stop it half-way
        // through — the first exception is kept here, the callback is not
        // called again, and the generation is left to finish.
        let mut failure: Option<PyErr> = None;
        let mut log = |line: &str| {
            let Some(callback) = on_log.as_ref() else {
                return;
            };
            if failure.is_some() {
                return;
            }

            // The GIL is released for the whole generation, so each line takes
            // it back for the length of one call and gives it up again.
            Python::attach(|py| {
                if let Err(error) = callback.call1(py, (line,)) {
                    failure = Some(error);
                }
            });
        };

        // Released for the duration, like a model build: the generation is
        // short, but a script watching it through `on_log` is watching one
        // that is really running.
        let entries = py.detach(|| {
            collomatique_greedy_groups::greedy_group_lists_with_log(&plan, &names, &mut log)
        });

        // The callback's exception wins over what the generation made of it:
        // the script asked for the lines and one of them was refused, so no
        // result is handed back.
        if let Some(failure) = failure {
            return Err(failure);
        }

        crate::generation::build(py, entries, &plan.skipped)
    }
}
