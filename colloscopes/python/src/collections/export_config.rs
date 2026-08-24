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
//!
//! Written through the eleven setters of [ExportConfig] — `set_global`, the
//! five `set_*_enabled` toggles and the five `set_*_config` section writes.
//! Each of them replaces one field of the configuration whole, since the
//! sections themselves are wholes; none of them creates anything, and none of
//! them can be refused by the document: the configuration is pure presentation
//! data that names no entity, so there is nothing here for the cascade to
//! repair and nothing for the model to object to. `ExportConfigUpdateError`
//! has no variants at all, which is that same sentence written in rust — what
//! a value carries is the value boundary's business, as ever, and that is the
//! only refusal this family has.

use std::collections::BTreeMap;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use collomatique_ops::{ExportConfigUpdateOp, UpdateOp};
use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::export_config::{
    ColloscopeConfig as RawColloscopeConfig, GlobalConfig as RawGlobalConfig,
    PerGroupListConfig as RawPerGroupListConfig,
    PerStudentGroupsConfig as RawPerStudentGroupsConfig,
};

use crate::Document;
use crate::data::Value as _;
use crate::data::{
    ExportColloscopeConfigData, ExportConfigData, ExportGlobalConfigData,
    ExportGroupListConfigData, ExportStudentGroupsConfigData,
};
use crate::results::OpResult;
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

    /// The whole configuration, detached — an `ExportConfigData` holding the
    /// tree the views read
    ///
    /// A fresh object every call, the whole configuration as the document
    /// holds it: the settings shared by every sheet, the five enabled flags
    /// that sit beside the sections they gate, and the four per-sheet
    /// configs. Nothing here can go stale: the whole configuration is one
    /// atom, replaced wholesale, so this never raises `StaleHandleError`.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let config = self.with_data(py, |data| data.export_config.clone());

        ExportConfigData::to_py(py, &config)
    }

    /// Rewrites the settings shared by every sheet of the export
    ///
    /// Takes an `ExportGlobalConfigData` and installs it whole: what the value
    /// says is what the section becomes, field for field — there is no merging
    /// with what was there, so a value built from scratch writes the model's
    /// own defaults wherever it was left alone.
    ///
    /// ```python
    /// doc.export_config.set_global(collomatique.ExportGlobalConfigData(
    ///     stripes_color_enabled=False))
    /// ```
    ///
    /// `doc.export_config.global_config.to_data()` is how a script gets the
    /// section as it stands, to hand back with one field changed.
    fn set_global(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        // Extracted before the mutable borrow, never inside it: a value naming
        // an entity is resolved against this document, which borrows it to ask
        // (`docs/python/new_api_design.md` §5). No value of this family names
        // one, but the order is the boundary's and not each value's.
        let config = ExportGlobalConfigData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdateGlobalConfig(config)),
        )
    }

    /// Says whether the colloscope sheet is part of the export
    ///
    /// The flag sits beside the section it gates rather than inside it, so
    /// switching the sheet off keeps everything `set_colloscope_config` wrote:
    /// that is the interface's memory of what was chosen before, and switching
    /// the sheet back on finds it as it was.
    fn set_colloscope_enabled(&self, py: Python<'_>, enabled: bool) -> PyResult<OpResult> {
        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdateColloscopeEnabled(enabled)),
        )
    }

    /// Says whether the all-groups sheet is part of the export
    ///
    /// The flag sits beside the section it gates, like every other one here:
    /// what `set_all_groups_config` wrote outlives the sheet being switched
    /// off.
    fn set_all_groups_enabled(&self, py: Python<'_>, enabled: bool) -> PyResult<OpResult> {
        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdateAllGroupsEnabled(enabled)),
        )
    }

    /// Says whether the automatic-groups sheet is part of the export
    ///
    /// The flag sits beside the section it gates, like every other one here:
    /// what `set_automatic_groups_config` wrote outlives the sheet being
    /// switched off.
    fn set_automatic_groups_enabled(&self, py: Python<'_>, enabled: bool) -> PyResult<OpResult> {
        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdateAutomaticGroupsEnabled(enabled)),
        )
    }

    /// Says whether the prefilled-groups sheet is part of the export
    ///
    /// The flag sits beside the section it gates, like every other one here:
    /// what `set_prefilled_groups_config` wrote outlives the sheet being
    /// switched off.
    fn set_prefilled_groups_enabled(&self, py: Python<'_>, enabled: bool) -> PyResult<OpResult> {
        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdatePrefilledGroupsEnabled(enabled)),
        )
    }

    /// Says whether the per-group-list sheets are part of the export
    ///
    /// The flag sits beside the section it gates, like every other one here:
    /// what `set_per_group_list_config` wrote outlives the sheets being
    /// switched off.
    fn set_per_group_list_enabled(&self, py: Python<'_>, enabled: bool) -> PyResult<OpResult> {
        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdatePerGroupListEnabled(enabled)),
        )
    }

    /// Rewrites the settings of the colloscope sheet
    ///
    /// Takes an `ExportColloscopeConfigData` and installs it whole, the
    /// `extra_colors` map included: what the value holds is the whole of the
    /// section afterwards, so a value whose map is empty leaves the sheet with
    /// no extra colors at all.
    ///
    /// ```python
    /// config = doc.export_config.colloscope_config.to_data()
    /// config.extra_colors["Vacances"] = collomatique.Color(255, 240, 200)
    /// doc.export_config.set_colloscope_config(config)
    /// ```
    ///
    /// The sheet is written or not according to `set_colloscope_enabled`,
    /// which this never touches: the flag is beside the section, not in it.
    fn set_colloscope_config(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let config = ExportColloscopeConfigData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdateColloscopeConfig(config)),
        )
    }

    /// Rewrites the settings of the all-groups sheet
    ///
    /// Takes an `ExportStudentGroupsConfigData` and installs it whole. The
    /// three sheets share one value class, and `sheet_name` is a field of it
    /// like any other — what says *which* sheet is being written is the method
    /// that is called, so handing
    /// `ExportStudentGroupsConfigData.automatic_groups()` to this one is not
    /// refused: it renames the all-groups sheet « Groupes automatiques », which
    /// is what the value asked for.
    ///
    /// ```python
    /// doc.export_config.set_all_groups_config(
    ///     collomatique.ExportStudentGroupsConfigData(
    ///         sheet_name="Groupes", show_emails=False))
    /// ```
    fn set_all_groups_config(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let config = ExportStudentGroupsConfigData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdateAllGroupsConfig(config)),
        )
    }

    /// Rewrites the settings of the automatic-groups sheet
    ///
    /// The twin of `set_all_groups_config`, for the other sheet: the value is
    /// installed whole, and `sheet_name` is a field it carries rather than the
    /// address it is written to.
    fn set_automatic_groups_config(
        &self,
        py: Python<'_>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let config = ExportStudentGroupsConfigData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(config)),
        )
    }

    /// Rewrites the settings of the prefilled-groups sheet
    ///
    /// The twin of `set_all_groups_config`, for the other sheet: the value is
    /// installed whole, and `sheet_name` is a field it carries rather than the
    /// address it is written to.
    fn set_prefilled_groups_config(
        &self,
        py: Python<'_>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let config = ExportStudentGroupsConfigData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(config)),
        )
    }

    /// Rewrites the settings of the per-group-list sheets
    ///
    /// Takes an `ExportGroupListConfigData` and installs it whole. One section
    /// for every one of those sheets — they are written from one setting, as
    /// the model holds them.
    ///
    /// ```python
    /// doc.export_config.set_per_group_list_config(
    ///     collomatique.ExportGroupListConfigData(center_vertically=True))
    /// ```
    fn set_per_group_list_config(
        &self,
        py: Python<'_>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let config = ExportGroupListConfigData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::ExportConfig(ExportConfigUpdateOp::UpdatePerGroupListConfig(config)),
        )
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

impl ExportConfig {
    /// Writes through the document the view came from
    ///
    /// The whole family ends here: none of its eleven ops creates anything, so
    /// none of them needs [crate::results::created]'s second half.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
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

    /// This section, detached — an `ExportGlobalConfigData` holding what the
    /// view shows
    ///
    /// A fresh object every call. Nothing here can go stale: the whole
    /// configuration is one atom, replaced wholesale, so this never raises
    /// `StaleHandleError`.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let config = self.read(py, |config| config.clone());

        ExportGlobalConfigData::to_py(py, &config)
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

    /// This section, detached — an `ExportColloscopeConfigData` holding what
    /// the view shows
    ///
    /// A fresh object every call. Nothing here can go stale: the whole
    /// configuration is one atom, replaced wholesale, so this never raises
    /// `StaleHandleError`.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let config = self.read(py, |config| config.clone());

        ExportColloscopeConfigData::to_py(py, &config)
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

    /// This sheet's settings, detached — an `ExportStudentGroupsConfigData`
    /// holding what the view shows
    ///
    /// A fresh object every call, the section the view is bound to: the
    /// all-groups view hands back a value whose `sheet_name` is « Tous les
    /// groupes », and the two siblings hand back their own. Nothing here can
    /// go stale: the whole configuration is one atom, replaced wholesale, so
    /// this never raises `StaleHandleError`.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let config = self.read(py, |config| config.clone());

        ExportStudentGroupsConfigData::to_py(py, &config)
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

    /// This section, detached — an `ExportGroupListConfigData` holding what
    /// the view shows
    ///
    /// A fresh object every call. Nothing here can go stale: the whole
    /// configuration is one atom, replaced wholesale, so this never raises
    /// `StaleHandleError`.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let config = self.read(py, |config| config.clone());

        ExportGroupListConfigData::to_py(py, &config)
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
