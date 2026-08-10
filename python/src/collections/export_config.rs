//! The export configuration of a document
//!
//! Reached as `doc.export_config`: the
//! presentation preferences of the xlsx export, held the way the model holds
//! them — one atom of value data: a global section, four per-sheet sections,
//! and the enabled flag that gates each of them. The flags sit *beside* the
//! sections they gate, not inside them: a flag is the interface's memory of
//! what was chosen before a section was switched off.
//!
//! Everything reads as [Color] and [Orientation] values, and nothing
//! here can go stale: the whole configuration is one atom, replaced wholesale
//! — there is nothing to remove from under a view — so every view below is
//! bound to the document alone and reads the current state on every access.

use std::collections::BTreeMap;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::export_config::{
    ColloscopeConfig as RawColloscopeConfig, GlobalConfig as RawGlobalConfig,
    PerGroupListConfig as RawPerGroupListConfig,
    PerStudentGroupsConfig as RawPerStudentGroupsConfig,
};

use crate::Document;
use crate::values::{Color, Orientation};

/// The export configuration of one document
///
/// Frozen and holding nothing but the document: it is a view, so two of them
/// on the same document are interchangeable and neither can go stale. A
/// singleton view — the export configuration has no place in a collection and
/// no id of its own, so there is no collection protocol here, only the nine
/// members below.
#[pyclass(module = "collomatique", frozen)]
pub struct ExportConfig {
    doc: Py<Document>,
}

impl ExportConfig {
    /// Builds the view — `doc.export_config` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> ExportConfig {
        ExportConfig { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }
}

#[pymethods]
impl ExportConfig {
    /// Whether the colloscope sheet is part of the export
    #[getter]
    fn colloscope_enabled(&self, py: Python<'_>) -> bool {
        self.with_data(py, |data| data.export_config.colloscope_enabled)
    }

    /// Whether the all-groups sheet is part of the export
    #[getter]
    fn all_groups_enabled(&self, py: Python<'_>) -> bool {
        self.with_data(py, |data| data.export_config.all_groups_enabled)
    }

    /// Whether the automatic-groups sheet is part of the export
    #[getter]
    fn automatic_groups_enabled(&self, py: Python<'_>) -> bool {
        self.with_data(py, |data| data.export_config.automatic_groups_enabled)
    }

    /// Whether the prefilled-groups sheet is part of the export
    #[getter]
    fn prefilled_groups_enabled(&self, py: Python<'_>) -> bool {
        self.with_data(py, |data| data.export_config.prefilled_groups_enabled)
    }

    /// Whether the per-group-list sheets are part of the export
    #[getter]
    fn per_group_list_enabled(&self, py: Python<'_>) -> bool {
        self.with_data(py, |data| data.export_config.per_group_list_enabled)
    }

    /// The settings shared by every sheet of the export
    #[getter]
    fn global_config(&self, py: Python<'_>) -> ExportGlobalConfig {
        ExportGlobalConfig::mint(self.doc.clone_ref(py))
    }

    /// The settings of the colloscope sheet
    #[getter]
    fn colloscope_config(&self, py: Python<'_>) -> ExportColloscopeConfig {
        ExportColloscopeConfig::mint(self.doc.clone_ref(py))
    }

    /// The settings of the all-groups sheet
    #[getter]
    fn all_groups_config(&self, py: Python<'_>) -> ExportStudentGroupsConfig {
        ExportStudentGroupsConfig::mint(self.doc.clone_ref(py), StudentGroupsKind::All)
    }

    /// The settings of the automatic-groups sheet
    #[getter]
    fn automatic_groups_config(&self, py: Python<'_>) -> ExportStudentGroupsConfig {
        ExportStudentGroupsConfig::mint(self.doc.clone_ref(py), StudentGroupsKind::Automatic)
    }

    /// The settings of the prefilled-groups sheet
    #[getter]
    fn prefilled_groups_config(&self, py: Python<'_>) -> ExportStudentGroupsConfig {
        ExportStudentGroupsConfig::mint(self.doc.clone_ref(py), StudentGroupsKind::Prefilled)
    }

    /// The settings of the per-group-list sheets
    #[getter]
    fn per_group_list_config(&self, py: Python<'_>) -> ExportGroupListConfig {
        ExportGroupListConfig::mint(self.doc.clone_ref(py))
    }

    /// The view itself — `<collomatique.ExportConfig>`
    ///
    /// Deliberately without a field count: the view has five sections, and a
    /// repr that counted one of them would be describing part of the
    /// configuration.
    fn __repr__(&self) -> String {
        "<collomatique.ExportConfig>".to_owned()
    }
}

/// The settings shared by every sheet of the export
///
/// A live sub-view, bound to `(document,)`
/// alone: it reads the current state on every access, and nothing can go
/// stale — the whole configuration is one atom of value data. A view is
/// identified by its document, so two accesses of the same configuration
/// compare equal.
#[pyclass(module = "collomatique", frozen)]
pub struct ExportGlobalConfig {
    doc: Py<Document>,
}

impl ExportGlobalConfig {
    /// Builds the view — `doc.export_config.global_config` is the only way to
    /// get one
    pub(crate) fn mint(doc: Py<Document>) -> ExportGlobalConfig {
        ExportGlobalConfig { doc }
    }

    /// Reads the document behind the view
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&RawGlobalConfig) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(&doc.data().get_inner_data().export_config.global)
    }
}

#[pymethods]
impl ExportGlobalConfig {
    /// The background color of the sheets
    #[getter]
    fn background_color(&self, py: Python<'_>) -> Color {
        self.read(py, |config| Color::from_model(&config.background_color))
    }

    /// Whether the alternating stripes are drawn
    #[getter]
    fn stripes_color_enabled(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.stripes_color_enabled)
    }

    /// The color of the alternating stripes
    #[getter]
    fn stripes_color(&self, py: Python<'_>) -> Color {
        self.read(py, |config| Color::from_model(&config.stripes_color))
    }

    /// Whether two views read the same document
    ///
    /// Never reads the state, so it keeps working however the configuration
    /// changes — the whole thing is one atom, and there is nothing to go stale.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<ExportGlobalConfig>() {
            Ok(other) => std::ptr::eq(self.doc.as_ptr(), other.get().doc.as_ptr()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.doc.as_ptr() as usize).hash(&mut hasher);
        hasher.finish()
    }

    fn __repr__(&self) -> String {
        "<collomatique.ExportGlobalConfig>".to_owned()
    }
}

/// The settings of the colloscope sheet
///
/// A live sub-view bound to `(document,)` alone, like [ExportGlobalConfig]:
/// nothing can go stale, and two accesses of the same configuration compare
/// equal.
#[pyclass(module = "collomatique", frozen)]
pub struct ExportColloscopeConfig {
    doc: Py<Document>,
}

impl ExportColloscopeConfig {
    /// Builds the view — `doc.export_config.colloscope_config` is the only way
    /// to get one
    pub(crate) fn mint(doc: Py<Document>) -> ExportColloscopeConfig {
        ExportColloscopeConfig { doc }
    }

    /// Reads the document behind the view
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&RawColloscopeConfig) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(&doc.data().get_inner_data().export_config.colloscope_config)
    }
}

#[pymethods]
impl ExportColloscopeConfig {
    /// The name of the sheet
    #[getter]
    fn sheet_name(&self, py: Python<'_>) -> String {
        self.read(py, |config| config.sheet_name.clone())
    }

    /// Whether the extra-info column is written
    #[getter]
    fn extra_info_column_enabled(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.extra_info_column_enabled)
    }

    /// The heading of the extra-info column
    #[getter]
    fn extra_info_column_name(&self, py: Python<'_>) -> String {
        self.read(py, |config| config.extra_info_column_name.clone())
    }

    /// Whether the teachers' email column is written
    #[getter]
    fn teacher_email_enabled(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.teacher_email_enabled)
    }

    /// The heading of the email column
    #[getter]
    fn teacher_email(&self, py: Python<'_>) -> String {
        self.read(py, |config| config.teacher_email.clone())
    }

    /// Whether the teachers' tel column is written
    #[getter]
    fn teacher_tel_enabled(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.teacher_tel_enabled)
    }

    /// The heading of the tel column
    #[getter]
    fn teacher_tel(&self, py: Python<'_>) -> String {
        self.read(py, |config| config.teacher_tel.clone())
    }

    /// Whether the sheet is printed tall or wide
    #[getter]
    fn orientation(&self, py: Python<'_>) -> Orientation {
        self.read(py, |config| Orientation::from_model(&config.orientation))
    }

    /// Whether the week dates are written
    #[getter]
    fn display_week_dates(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.display_week_dates)
    }

    /// Whether the week annotations are written
    #[getter]
    fn display_annotations(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.display_annotations)
    }

    /// The color of a cell that holds no interrogation
    #[getter]
    fn no_interrogation_color(&self, py: Python<'_>) -> Color {
        self.read(py, |config| {
            Color::from_model(&config.no_interrogation_color)
        })
    }

    /// Whether the annotation cells are tinted
    #[getter]
    fn annotation_color_enabled(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.annotation_color_enabled)
    }

    /// The tint of the annotation cells
    #[getter]
    fn annotation_color(&self, py: Python<'_>) -> Color {
        self.read(py, |config| Color::from_model(&config.annotation_color))
    }

    /// The extra cell colors, by the label that names them
    ///
    /// A read-only mapping — a `types.MappingProxyType` over a fresh dict,
    /// like the colloscope placements: reading it is reading the document,
    /// mutating it is `TypeError`.
    #[getter]
    fn extra_colors<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let colors = self.read(py, |config| config.extra_colors.clone());
        extra_colors_mapping(py, colors)
    }

    /// Whether two views read the same document
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<ExportColloscopeConfig>() {
            Ok(other) => std::ptr::eq(self.doc.as_ptr(), other.get().doc.as_ptr()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.doc.as_ptr() as usize).hash(&mut hasher);
        hasher.finish()
    }

    fn __repr__(&self) -> String {
        "<collomatique.ExportColloscopeConfig>".to_owned()
    }
}

/// The extra colors of a colloscope sheet, as a read-only mapping of [Color]
/// values
///
/// A fresh dict keyed by the labels the model stores, wrapped in
/// `types.MappingProxyType` — a read-only mapping: the proxy cannot be written
/// through, and the
/// dict under it is unreachable, so there is nothing to mutate by accident.
fn extra_colors_mapping<'py>(
    py: Python<'py>,
    colors: BTreeMap<String, collomatique_state_colloscopes::export_config::Color>,
) -> PyResult<Bound<'py, PyAny>> {
    let dict = PyDict::new(py);
    for (name, color) in colors {
        dict.set_item(name, Color::from_model(&color))?;
    }
    py.import("types")?
        .getattr("MappingProxyType")?
        .call1((dict,))
}

/// Which per-student-groups sheet a view reads
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StudentGroupsKind {
    /// `doc.export_config.all_groups_config`
    All,
    /// `doc.export_config.automatic_groups_config`
    Automatic,
    /// `doc.export_config.prefilled_groups_config`
    Prefilled,
}

impl StudentGroupsKind {
    /// The word a repr shows, the config's own attribute name
    fn word(self) -> &'static str {
        match self {
            StudentGroupsKind::All => "all_groups",
            StudentGroupsKind::Automatic => "automatic_groups",
            StudentGroupsKind::Prefilled => "prefilled_groups",
        }
    }
}

/// The settings of one per-student-groups sheet
///
/// A live sub-view bound to `(document, kind)`: one class serves the three
/// sheets — all groups, automatic groups and prefilled groups — and the kind
/// is part of the view's identity, as it is for the settings [Limits] view: a
/// view of one sheet never compares equal to a view of another, and two views
/// of the same sheet do. Nothing can go stale — the whole configuration is one
/// atom of value data.
///
/// [Limits]: crate::collections::settings::Limits
#[pyclass(module = "collomatique", frozen)]
pub struct ExportStudentGroupsConfig {
    doc: Py<Document>,
    kind: StudentGroupsKind,
}

impl ExportStudentGroupsConfig {
    /// Builds the view — `doc.export_config`'s three members are the only ways
    /// to get one
    pub(crate) fn mint(doc: Py<Document>, kind: StudentGroupsKind) -> ExportStudentGroupsConfig {
        ExportStudentGroupsConfig { doc, kind }
    }

    /// Reads the section of the document's configuration the view is bound to
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&RawPerStudentGroupsConfig) -> R) -> R {
        let doc = self.doc.borrow(py);
        let config = &doc.data().get_inner_data().export_config;
        let section = match self.kind {
            StudentGroupsKind::All => &config.all_groups_config,
            StudentGroupsKind::Automatic => &config.automatic_groups_config,
            StudentGroupsKind::Prefilled => &config.prefilled_groups_config,
        };
        f(section)
    }
}

#[pymethods]
impl ExportStudentGroupsConfig {
    /// The name of the sheet
    #[getter]
    fn sheet_name(&self, py: Python<'_>) -> String {
        self.read(py, |config| config.sheet_name.clone())
    }

    /// Whether the sheet is printed tall, wide, or automatically
    ///
    /// `None` is the model's auto-detect: the orientation is chosen from the
    /// group count when the sheet is written, so nothing here reads as an
    /// orientation.
    #[getter]
    fn orientation(&self, py: Python<'_>) -> Option<Orientation> {
        self.read(py, |config| {
            config.orientation.as_ref().map(Orientation::from_model)
        })
    }

    /// Whether the students' emails are written
    #[getter]
    fn show_emails(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.show_emails)
    }

    /// Whether the students' phone numbers are written
    #[getter]
    fn show_tel(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.show_tel)
    }

    /// Whether two views are bound to the same sheet of the same document
    ///
    /// The kind is part of the identity: a view of the all-groups sheet never
    /// compares equal to a view of the automatic-groups one. Never reads the
    /// state, so it keeps working however the configuration changes.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<ExportStudentGroupsConfig>() {
            Ok(other) => {
                let other = other.get();
                std::ptr::eq(self.doc.as_ptr(), other.doc.as_ptr()) && self.kind == other.kind
            }
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.doc.as_ptr() as usize).hash(&mut hasher);
        self.kind.hash(&mut hasher);
        hasher.finish()
    }

    /// `<collomatique.ExportStudentGroupsConfig all_groups>` — the kind names
    /// which of the three sheets the view reads.
    fn __repr__(&self) -> String {
        format!(
            "<collomatique.ExportStudentGroupsConfig {}>",
            self.kind.word()
        )
    }
}

/// The settings of the per-group-list sheets
///
/// A live sub-view bound to `(document,)` alone, like [ExportGlobalConfig]:
/// nothing can go stale, and two accesses of the same configuration compare
/// equal.
#[pyclass(module = "collomatique", frozen)]
pub struct ExportGroupListConfig {
    doc: Py<Document>,
}

impl ExportGroupListConfig {
    /// Builds the view — `doc.export_config.per_group_list_config` is the only
    /// way to get one
    pub(crate) fn mint(doc: Py<Document>) -> ExportGroupListConfig {
        ExportGroupListConfig { doc }
    }

    /// Reads the document behind the view
    fn read<R>(&self, py: Python<'_>, f: impl FnOnce(&RawPerGroupListConfig) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(&doc
            .data()
            .get_inner_data()
            .export_config
            .per_group_list_config)
    }
}

#[pymethods]
impl ExportGroupListConfig {
    /// Whether the sheets are printed tall or wide
    #[getter]
    fn orientation(&self, py: Python<'_>) -> Orientation {
        self.read(py, |config| Orientation::from_model(&config.orientation))
    }

    /// Whether the students' emails are written
    #[getter]
    fn show_emails(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.show_emails)
    }

    /// Whether the students' phone numbers are written
    #[getter]
    fn show_tel(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.show_tel)
    }

    /// Whether the sheets are centered vertically on the page
    #[getter]
    fn center_vertically(&self, py: Python<'_>) -> bool {
        self.read(py, |config| config.center_vertically)
    }

    /// Whether two views read the same document
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<ExportGroupListConfig>() {
            Ok(other) => std::ptr::eq(self.doc.as_ptr(), other.get().doc.as_ptr()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.doc.as_ptr() as usize).hash(&mut hasher);
        hasher.finish()
    }

    fn __repr__(&self) -> String {
        "<collomatique.ExportGroupListConfig>".to_owned()
    }
}
