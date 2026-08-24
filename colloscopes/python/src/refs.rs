//! What points at an entity
//!
//! `referenced_by()`, the one reverse door a
//! script asks before a remove — *what will a cascade touch?* It rides
//! `InnerData::references_to_*` (the reference registry of
//! `colloscopes/state-colloscopes/src/refs.rs`) and hands the sites back as frozen value
//! classes under the [RefSite] base, in the registry's walk order.
//!
//! A site carries the **full coordinates of the referring place** as handles —
//! target included, unlike the rust site enums, whose context implies it. Full
//! coordinates cost nothing and let one class serve every target kind that can
//! appear in the same place: `AssignmentRow(period, subject)` is the site for a
//! period, for a subject, and for each student in the row.
//!
//! Three kinds are never the target of a reference — the registry has no site
//! vocabulary for `Incompat`, `PairingRule` and `SlotPairingRule` — so their
//! `referenced_by()` always answers `()` while the handle is alive. Stale is
//! still loud there as everywhere: once the entity is gone, `referenced_by()`
//! raises `StaleHandleError` like any other read, because the handle itself
//! reads nothing more than its liveness to answer.

use pyo3::PyClass;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use collomatique_state_colloscopes::ids::{
    GroupListId, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekId, WeekPatternId,
};
use collomatique_state_colloscopes::refs::{
    GroupListRefSite, PeriodRefSite, SlotRefSite, StudentRefSite, SubjectRefSite, TeacherRefSite,
    WeekPatternRefSite, WeekRefSite,
};

use crate::Document;
use crate::collections::{
    GroupList, Incompat, PairingRule, Period, Slot, SlotPairingRule, Student, Subject, Teacher,
    Week, WeekPattern,
};
use crate::handles::{Handle, RawId};

/// The base of every reference site
///
/// A site is a frozen value class under this base, so `isinstance(site,
/// RefSite)` catches all of them. It has no constructor of its own: every site
/// is one of the subclasses, and `collomatique.RefSite()` raises `TypeError`.
#[pyclass(module = "collomatique", subclass, frozen)]
pub struct RefSite;

/// Declares one site class with a single coordinate
///
/// The coordinate is stored as the pair a handle keeps — `(document, id)` —
/// and the getter mints the handle on demand, like every navigation on the
/// read surface. The site is a value: `==` and `hash` work on
/// `(document identity, ids)` and never touch the state, so sites keep
/// comparing after anything dies, and the repr reuses the coordinate handle's
/// own repr.
macro_rules! single_site {
    (
        $(#[$meta:meta])*
        $name:ident, one $coord:ident: $handle:ty;
    ) => {
        $(#[$meta])*
        #[pyclass(module = "collomatique", extends = RefSite, frozen)]
        pub struct $name {
            doc: pyo3::Py<Document>,
            $coord: RawId<$handle>,
        }

        impl $name {
            fn init(self) -> PyClassInitializer<Self> {
                PyClassInitializer::from(RefSite).add_subclass(self)
            }
        }

        #[pymethods]
        impl $name {
            /// Builds the site from the handle of the referring place
            ///
            /// A site is a *place*, so its coordinates are handles — a bare id
            /// cannot name a document, and the site carries none of its own. A
            /// script writes `clm.SlotTeacher(slot)` to ask whether that place
            /// is what the document points at.
            #[new]
            fn new($coord: PyRef<'_, $handle>) -> PyClassInitializer<Self> {
                use crate::handles::Handle as _;

                let py = $coord.py();
                $name {
                    doc: $coord.document().clone_ref(py),
                    $coord: $coord.raw_id(),
                }
                .init()
            }

            #[classattr]
            #[allow(non_upper_case_globals)]
            const __match_args__: (&'static str,) = (stringify!($coord),);

            /// The referring place, as a live handle
            #[getter]
            fn $coord(&self, py: Python<'_>) -> $handle {
                use crate::handles::Handle as _;

                <$handle>::mint(self.doc.clone_ref(py), self.$coord)
            }

            /// Whether two sites name the same place
            ///
            /// The coordinates compare the way handles do — document identity
            /// and id — so a site built from another document's handles never
            /// matches one this document handed out.
            fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
                match other.cast::<$name>() {
                    Ok(other) => {
                        let other = other.get();
                        same_pair(&self.doc, &other.doc, self.$coord, other.$coord)
                    }
                    Err(_) => false,
                }
            }

            fn __hash__(&self) -> u64 {
                hash_pair(&self.doc, self.$coord)
            }

            fn __repr__(&self, py: Python<'_>) -> String {
                use crate::handles::Handle as _;

                let coord = <$handle>::mint(self.doc.clone_ref(py), self.$coord);
                let coord = Py::new(py, coord)
                    .and_then(|obj| obj.bind(py).repr())
                    .map(|repr| repr.to_string())
                    .unwrap_or_else(|_| "<périmé>".to_owned());
                format!("{}({}={})", stringify!($name), stringify!($coord), coord)
            }
        }
    };
}

/// Declares one site class with two coordinates
///
/// Each coordinate keeps its own `(document, id)` pair: the two handles a
/// script builds the site from need not be bound to the same document, and a
/// site whose coordinates straddle two documents simply never matches
/// anything — the honest answer for a value that was built wrong.
macro_rules! double_site {
    (
        $(#[$meta:meta])*
        $name:ident, $first:ident: $first_handle:ty, $second:ident: $second_handle:ty;
    ) => {
        $(#[$meta])*
        #[pyclass(module = "collomatique", extends = RefSite, frozen)]
        pub struct $name {
            #[allow(non_snake_case)]
            $first: (pyo3::Py<Document>, RawId<$first_handle>),
            #[allow(non_snake_case)]
            $second: (pyo3::Py<Document>, RawId<$second_handle>),
        }

        impl $name {
            fn init(self) -> PyClassInitializer<Self> {
                PyClassInitializer::from(RefSite).add_subclass(self)
            }
        }

        #[pymethods]
        impl $name {
            /// Builds the site from the handles of the referring place
            ///
            /// A site is a *place*, so its coordinates are handles — a bare id
            /// cannot name a document, and the site carries none of its own.
            #[new]
            fn new(
                $first: PyRef<'_, $first_handle>,
                $second: PyRef<'_, $second_handle>,
            ) -> PyClassInitializer<Self> {
                use crate::handles::Handle as _;

                let py = $first.py();
                $name {
                    $first: ($first.document().clone_ref(py), $first.raw_id()),
                    $second: ($second.document().clone_ref(py), $second.raw_id()),
                }
                .init()
            }

            #[classattr]
            #[allow(non_upper_case_globals)]
            const __match_args__: (&'static str, &'static str) =
                (stringify!($first), stringify!($second));

            /// The first coordinate, as a live handle
            #[getter]
            fn $first(&self, py: Python<'_>) -> $first_handle {
                use crate::handles::Handle as _;

                let (doc, id) = &self.$first;
                <$first_handle>::mint(doc.clone_ref(py), *id)
            }

            /// The second coordinate, as a live handle
            #[getter]
            fn $second(&self, py: Python<'_>) -> $second_handle {
                use crate::handles::Handle as _;

                let (doc, id) = &self.$second;
                <$second_handle>::mint(doc.clone_ref(py), *id)
            }

            /// Whether two sites name the same place
            fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
                match other.cast::<$name>() {
                    Ok(other) => {
                        let other = other.get();
                        same_pair(&self.$first.0, &other.$first.0, self.$first.1, other.$first.1)
                            && same_pair(
                                &self.$second.0,
                                &other.$second.0,
                                self.$second.1,
                                other.$second.1,
                            )
                    }
                    Err(_) => false,
                }
            }

            fn __hash__(&self) -> u64 {
                hash_quad(&self.$first.0, self.$first.1, &self.$second.0, self.$second.1)
            }

            fn __repr__(&self, py: Python<'_>) -> String {
                use crate::handles::Handle as _;

                let (first_doc, first_id) = &self.$first;
                let (second_doc, second_id) = &self.$second;
                let first = <$first_handle>::mint(first_doc.clone_ref(py), *first_id);
                let second = <$second_handle>::mint(second_doc.clone_ref(py), *second_id);
                let first = Py::new(py, first)
                    .and_then(|obj| obj.bind(py).repr())
                    .map(|repr| repr.to_string())
                    .unwrap_or_else(|_| "<périmé>".to_owned());
                let second = Py::new(py, second)
                    .and_then(|obj| obj.bind(py).repr())
                    .map(|repr| repr.to_string())
                    .unwrap_or_else(|_| "<périmé>".to_owned());
                format!(
                    "{}({}={}, {}={})",
                    stringify!($name),
                    stringify!($first),
                    first,
                    stringify!($second),
                    second,
                )
            }
        }
    };
}

/// Whether two coordinates name the same place — document identity and id,
/// like a handle's `==`
fn same_pair<T: PartialEq>(doc_a: &Py<Document>, doc_b: &Py<Document>, id_a: T, id_b: T) -> bool {
    std::ptr::eq(doc_a.as_ptr(), doc_b.as_ptr()) && id_a == id_b
}

/// The hash of the same pair `==` compares
fn hash_pair<T: std::hash::Hash>(doc: &Py<Document>, id: T) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (doc.as_ptr() as usize).hash(&mut hasher);
    id.hash(&mut hasher);
    hasher.finish()
}

/// The hash of a two-coordinate site, over both pairs
fn hash_quad<T: std::hash::Hash, U: std::hash::Hash>(
    first_doc: &Py<Document>,
    first_id: T,
    second_doc: &Py<Document>,
    second_id: U,
) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (first_doc.as_ptr() as usize).hash(&mut hasher);
    first_id.hash(&mut hasher);
    (second_doc.as_ptr() as usize).hash(&mut hasher);
    second_id.hash(&mut hasher);
    hasher.finish()
}

/// Builds one site instance for python
fn build<S>(py: Python<'_>, init: PyClassInitializer<S>) -> PyResult<Py<PyAny>>
where
    S: PyClass + Send + Sync,
{
    Ok(Py::new(py, init)?.into_any())
}

single_site! {
    /// The period a week belongs to
    ///
    /// A week's `.period` is a reference the same way its `excluded_weeks`
    /// patterns are: `week.referenced_by()` on the *period* hands this site
    /// out. It is the one site every week of the document produces.
    WeekPeriod, one week: Week;
}

single_site! {
    /// A period a subject's exclusion set names
    ///
    /// The `subject.excluded_periods` reads of the document are the same edges
    /// these sites are, seen from the other end.
    SubjectExcludedPeriod, one subject: Subject;
}

single_site! {
    /// A period a student's exclusion set names
    ///
    /// The `student.excluded_periods` reads of the document are the same edges
    /// these sites are, seen from the other end.
    StudentExcludedPeriod, one student: Student;
}

single_site! {
    /// A period a pairing rule's exclusion set names
    PairingRuleExcludedPeriod, one rule: PairingRule;
}

single_site! {
    /// A period a slot pairing rule's exclusion set names
    SlotPairingRuleExcludedPeriod, one rule: SlotPairingRule;
}

double_site! {
    /// An assignments row — its key, or one of its students
    ///
    /// The one class that serves three targets: the row's period, the row's
    /// subject, and every student in the row all read this same-shaped site
    /// from their `referenced_by()`.
    AssignmentRow, period: Period, subject: Subject;
}

double_site! {
    /// An association entry — its key, or its group list
    ///
    /// The one class that serves three targets: the entry's period, the
    /// entry's subject, and the group list the entry points at.
    GroupListAssociation, period: Period, subject: Subject;
}

single_site! {
    /// A subject a teacher's subject set names
    ///
    /// The `teacher.subjects` reads of the document are the same edges these
    /// sites are, seen from the other end.
    TeacherSubject, one teacher: Teacher;
}

single_site! {
    /// The subject a slot belongs to
    ///
    /// Fixed at creation, per the design's write table — a slot can never
    /// change subject, so this edge lives for the slot's whole life.
    SlotSubject, one slot: Slot;
}

single_site! {
    /// The teacher a slot runs under
    SlotTeacher, one slot: Slot;
}

single_site! {
    /// The week pattern a slot follows
    SlotWeekPattern, one slot: Slot;
}

single_site! {
    /// The subject an incompatibility blocks
    IncompatSubject, one incompat: Incompat;
}

single_site! {
    /// The week pattern an incompatibility follows
    IncompatWeekPattern, one incompat: Incompat;
}

single_site! {
    /// The antecedent subject of a pairing rule
    PairingRuleAntecedent, one rule: PairingRule;
}

single_site! {
    /// The consequent subject of a pairing rule
    PairingRuleConsequent, one rule: PairingRule;
}

single_site! {
    /// The antecedent slot of a slot pairing rule
    SlotPairingRuleAntecedent, one rule: SlotPairingRule;
}

single_site! {
    /// The consequent slot of a slot pairing rule
    SlotPairingRuleConsequent, one rule: SlotPairingRule;
}

single_site! {
    /// A per-student limits override entry
    ///
    /// The key of a `doc.settings` override row: the student the override
    /// applies to.
    SettingsOverride, one student: Student;
}

single_site! {
    /// A per-subject balancing override entry
    ///
    /// The key of a `doc.balancing` override row: the subject the override
    /// applies to.
    BalancingOverride, one subject: Subject;
}

single_site! {
    /// A week a week pattern's exception set names
    ///
    /// The `pattern.excluded_weeks` reads of the document are the same edges
    /// these sites are, seen from the other end.
    WeekPatternExcludedWeek, one week_pattern: WeekPattern;
}

single_site! {
    /// A student inside a prefilled group
    ///
    /// One site per student per group: a student in two groups of the same
    /// list yields the same-shaped site twice.
    GroupListPrefilledStudent, one group_list: GroupList;
}

single_site! {
    /// A student an automatic list's exclusion set names
    GroupListExcludedStudent, one group_list: GroupList;
}

double_site! {
    /// A colloscope interrogation cell
    ///
    /// The one class that serves two targets: the cell's slot and the cell's
    /// week both read this same-shaped site from their `referenced_by()`.
    ColloscopeInterrogation, slot: Slot, week: Week;
}

single_site! {
    /// A colloscope placements row — its key, or one of its students
    ///
    /// The one class that serves two targets: the group list the placements
    /// belong to, and every student placed in it.
    ColloscopeGroupListRow, one group_list: GroupList;
}

/// The conversion each referencable kind's sites take
///
/// `references_to_*` hands back the sites whose target is the id it was asked
/// about — the site enums do not carry the target, because their context
/// implies it — so every conversion takes the target id along, and fills the
/// target coordinate with it.
///
/// Written as matches rather than as `From` impls per variant, so that a new
/// site variant in the registry is a compile error here — the standing rule
/// for every model↔python conversion in this crate.
pub(crate) fn period_sites(
    py: Python<'_>,
    doc: &Py<Document>,
    target: PeriodId,
    sites: &[PeriodRefSite],
) -> PyResult<Vec<Py<PyAny>>> {
    let doc = || doc.clone_ref(py);
    sites
        .iter()
        .map(|site| match site {
            PeriodRefSite::WeekPeriodFk(week) => build(
                py,
                WeekPeriod {
                    doc: doc(),
                    week: *week,
                }
                .init(),
            ),
            PeriodRefSite::SubjectExcludedPeriods(subject) => build(
                py,
                SubjectExcludedPeriod {
                    doc: doc(),
                    subject: *subject,
                }
                .init(),
            ),
            PeriodRefSite::StudentExcludedPeriods(student) => build(
                py,
                StudentExcludedPeriod {
                    doc: doc(),
                    student: *student,
                }
                .init(),
            ),
            PeriodRefSite::PairingRuleExcludedPeriods(rule) => build(
                py,
                PairingRuleExcludedPeriod {
                    doc: doc(),
                    rule: *rule,
                }
                .init(),
            ),
            PeriodRefSite::SlotPairingRuleExcludedPeriods(rule) => build(
                py,
                SlotPairingRuleExcludedPeriod {
                    doc: doc(),
                    rule: *rule,
                }
                .init(),
            ),
            PeriodRefSite::AssignmentsKey { subject } => build(
                py,
                AssignmentRow {
                    period: (doc(), target),
                    subject: (doc(), *subject),
                }
                .init(),
            ),
            PeriodRefSite::AssociationEntry { subject } => build(
                py,
                GroupListAssociation {
                    period: (doc(), target),
                    subject: (doc(), *subject),
                }
                .init(),
            ),
        })
        .collect()
}

pub(crate) fn week_sites(
    py: Python<'_>,
    doc: &Py<Document>,
    target: WeekId,
    sites: &[WeekRefSite],
) -> PyResult<Vec<Py<PyAny>>> {
    let doc = || doc.clone_ref(py);
    sites
        .iter()
        .map(|site| match site {
            WeekRefSite::WeekPatternExcludedWeek(week_pattern) => build(
                py,
                WeekPatternExcludedWeek {
                    doc: doc(),
                    week_pattern: *week_pattern,
                }
                .init(),
            ),
            WeekRefSite::ColloscopeInterrogation { slot } => build(
                py,
                ColloscopeInterrogation {
                    slot: (doc(), *slot),
                    week: (doc(), target),
                }
                .init(),
            ),
        })
        .collect()
}

pub(crate) fn subject_sites(
    py: Python<'_>,
    doc: &Py<Document>,
    target: SubjectId,
    sites: &[SubjectRefSite],
) -> PyResult<Vec<Py<PyAny>>> {
    let doc = || doc.clone_ref(py);
    sites
        .iter()
        .map(|site| match site {
            SubjectRefSite::TeacherSubjects(teacher) => build(
                py,
                TeacherSubject {
                    doc: doc(),
                    teacher: *teacher,
                }
                .init(),
            ),
            SubjectRefSite::SlotSubject(slot) => build(
                py,
                SlotSubject {
                    doc: doc(),
                    slot: *slot,
                }
                .init(),
            ),
            SubjectRefSite::IncompatSubject(incompat) => build(
                py,
                IncompatSubject {
                    doc: doc(),
                    incompat: *incompat,
                }
                .init(),
            ),
            SubjectRefSite::PairingRuleAntecedent(rule) => build(
                py,
                PairingRuleAntecedent {
                    doc: doc(),
                    rule: *rule,
                }
                .init(),
            ),
            SubjectRefSite::PairingRuleConsequent(rule) => build(
                py,
                PairingRuleConsequent {
                    doc: doc(),
                    rule: *rule,
                }
                .init(),
            ),
            SubjectRefSite::BalancingSubjectKey => build(
                py,
                BalancingOverride {
                    doc: doc(),
                    subject: target,
                }
                .init(),
            ),
            SubjectRefSite::AssignmentsKey { period } => build(
                py,
                AssignmentRow {
                    period: (doc(), *period),
                    subject: (doc(), target),
                }
                .init(),
            ),
            SubjectRefSite::AssociationEntry { period } => build(
                py,
                GroupListAssociation {
                    period: (doc(), *period),
                    subject: (doc(), target),
                }
                .init(),
            ),
        })
        .collect()
}

pub(crate) fn teacher_sites(
    py: Python<'_>,
    doc: &Py<Document>,
    _target: TeacherId,
    sites: &[TeacherRefSite],
) -> PyResult<Vec<Py<PyAny>>> {
    sites
        .iter()
        .map(|site| match site {
            TeacherRefSite::SlotTeacher(slot) => build(
                py,
                SlotTeacher {
                    doc: doc.clone_ref(py),
                    slot: *slot,
                }
                .init(),
            ),
        })
        .collect()
}

pub(crate) fn student_sites(
    py: Python<'_>,
    doc: &Py<Document>,
    target: StudentId,
    sites: &[StudentRefSite],
) -> PyResult<Vec<Py<PyAny>>> {
    let doc = || doc.clone_ref(py);
    sites
        .iter()
        .map(|site| match site {
            StudentRefSite::GroupListPrefilledStudent(group_list) => build(
                py,
                GroupListPrefilledStudent {
                    doc: doc(),
                    group_list: *group_list,
                }
                .init(),
            ),
            StudentRefSite::GroupListExcludedStudent(group_list) => build(
                py,
                GroupListExcludedStudent {
                    doc: doc(),
                    group_list: *group_list,
                }
                .init(),
            ),
            StudentRefSite::SettingsStudentKey => build(
                py,
                SettingsOverride {
                    doc: doc(),
                    student: target,
                }
                .init(),
            ),
            StudentRefSite::AssignmentsStudent { period, subject } => build(
                py,
                AssignmentRow {
                    period: (doc(), *period),
                    subject: (doc(), *subject),
                }
                .init(),
            ),
            StudentRefSite::ColloscopeGroupListStudent(group_list) => build(
                py,
                ColloscopeGroupListRow {
                    doc: doc(),
                    group_list: *group_list,
                }
                .init(),
            ),
        })
        .collect()
}

pub(crate) fn week_pattern_sites(
    py: Python<'_>,
    doc: &Py<Document>,
    _target: WeekPatternId,
    sites: &[WeekPatternRefSite],
) -> PyResult<Vec<Py<PyAny>>> {
    sites
        .iter()
        .map(|site| match site {
            WeekPatternRefSite::SlotWeekPattern(slot) => build(
                py,
                SlotWeekPattern {
                    doc: doc.clone_ref(py),
                    slot: *slot,
                }
                .init(),
            ),
            WeekPatternRefSite::IncompatWeekPattern(incompat) => build(
                py,
                IncompatWeekPattern {
                    doc: doc.clone_ref(py),
                    incompat: *incompat,
                }
                .init(),
            ),
        })
        .collect()
}

pub(crate) fn slot_sites(
    py: Python<'_>,
    doc: &Py<Document>,
    target: SlotId,
    sites: &[SlotRefSite],
) -> PyResult<Vec<Py<PyAny>>> {
    sites
        .iter()
        .map(|site| match site {
            SlotRefSite::SlotPairingRuleAntecedent(rule) => build(
                py,
                SlotPairingRuleAntecedent {
                    doc: doc.clone_ref(py),
                    rule: *rule,
                }
                .init(),
            ),
            SlotRefSite::SlotPairingRuleConsequent(rule) => build(
                py,
                SlotPairingRuleConsequent {
                    doc: doc.clone_ref(py),
                    rule: *rule,
                }
                .init(),
            ),
            SlotRefSite::ColloscopeInterrogation { week } => build(
                py,
                ColloscopeInterrogation {
                    slot: (doc.clone_ref(py), target),
                    week: (doc.clone_ref(py), *week),
                }
                .init(),
            ),
        })
        .collect()
}

pub(crate) fn group_list_sites(
    py: Python<'_>,
    doc: &Py<Document>,
    target: GroupListId,
    sites: &[GroupListRefSite],
) -> PyResult<Vec<Py<PyAny>>> {
    sites
        .iter()
        .map(|site| match site {
            GroupListRefSite::AssociationEntry { period, subject } => build(
                py,
                GroupListAssociation {
                    period: (doc.clone_ref(py), *period),
                    subject: (doc.clone_ref(py), *subject),
                }
                .init(),
            ),
            GroupListRefSite::ColloscopeGroupListKey => build(
                py,
                ColloscopeGroupListRow {
                    doc: doc.clone_ref(py),
                    group_list: target,
                }
                .init(),
            ),
        })
        .collect()
}

/// What points at this entity — the `referenced_by` a handle class exposes
///
/// One function per kind the registry can point at: `$lookup` is the
/// `InnerData::references_to_*` reverse lookup, `$convert` the conversion that
/// turns its sites into python site values. They are called by the handle
/// classes' own `referenced_by` methods, which live next to their classes —
/// pyo3 collects all of a class's methods into one `#[pymethods]` block.
macro_rules! references {
    ($fn_name:ident, $handle:ty, $lookup:ident, $convert:ident) => {
        pub(crate) fn $fn_name(py: Python<'_>, handle: &$handle) -> PyResult<Py<PyTuple>> {
            use crate::handles::Handle as _;

            let id = handle.raw_id();
            // The reverse lookup answers an empty vec for an id the document no
            // longer holds — a dead entity has no live references — so the
            // liveness question is asked separately: `read` sees `None` and
            // raises, the way every other read does.
            let sites = handle.read(py, |data| {
                <$handle as crate::handles::Handle>::exists(data, id).then(|| data.$lookup(id))
            })?;
            let sites = $convert(py, handle.document(), id, &sites)?;
            Ok(PyTuple::new(py, sites)?.unbind())
        }
    };
}

references!(
    period_references,
    Period,
    references_to_period,
    period_sites
);
references!(week_references, Week, references_to_week, week_sites);
references!(
    subject_references,
    Subject,
    references_to_subject,
    subject_sites
);
references!(
    teacher_references,
    Teacher,
    references_to_teacher,
    teacher_sites
);
references!(
    student_references,
    Student,
    references_to_student,
    student_sites
);
references!(
    week_pattern_references,
    WeekPattern,
    references_to_week_pattern,
    week_pattern_sites
);
references!(slot_references, Slot, references_to_slot, slot_sites);
references!(
    group_list_references,
    GroupList,
    references_to_group_list,
    group_list_sites
);

/// The `referenced_by` of a kind nothing can point at
///
/// `Incompat`, `PairingRule` and `SlotPairingRule` are never the target of a
/// reference — the registry has no site vocabulary for them — so the answer is
/// the empty tuple, for as long as the handle is alive. The liveness question
/// is still asked, so a stale handle raises `StaleHandleError` like every
/// other read.
pub(crate) fn never_referenced<H>(py: Python<'_>, handle: &H) -> PyResult<Py<PyTuple>>
where
    H: Handle,
{
    let id = handle.raw_id();
    handle.read(py, |data| <H as Handle>::exists(data, id).then_some(()))?;
    Ok(PyTuple::empty(py).unbind())
}

/// Adds the site classes to the module
///
/// They are registered so `isinstance` and `repr` say something useful, and so
/// a script can name one: every site is constructible from the handles of its
/// place.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RefSite>()?;
    m.add_class::<WeekPeriod>()?;
    m.add_class::<SubjectExcludedPeriod>()?;
    m.add_class::<StudentExcludedPeriod>()?;
    m.add_class::<PairingRuleExcludedPeriod>()?;
    m.add_class::<SlotPairingRuleExcludedPeriod>()?;
    m.add_class::<AssignmentRow>()?;
    m.add_class::<GroupListAssociation>()?;
    m.add_class::<TeacherSubject>()?;
    m.add_class::<SlotSubject>()?;
    m.add_class::<SlotTeacher>()?;
    m.add_class::<SlotWeekPattern>()?;
    m.add_class::<IncompatSubject>()?;
    m.add_class::<IncompatWeekPattern>()?;
    m.add_class::<PairingRuleAntecedent>()?;
    m.add_class::<PairingRuleConsequent>()?;
    m.add_class::<SlotPairingRuleAntecedent>()?;
    m.add_class::<SlotPairingRuleConsequent>()?;
    m.add_class::<SettingsOverride>()?;
    m.add_class::<BalancingOverride>()?;
    m.add_class::<WeekPatternExcludedWeek>()?;
    m.add_class::<GroupListPrefilledStudent>()?;
    m.add_class::<GroupListExcludedStudent>()?;
    m.add_class::<ColloscopeInterrogation>()?;
    m.add_class::<ColloscopeGroupListRow>()?;
    Ok(())
}
