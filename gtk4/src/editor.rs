use adw::prelude::NavigationPageExt;
use collomatique_state::traits::Manager;
use gtk::prelude::{ButtonExt, ObjectExt, OrientableExt, WidgetExt};
use libadwaita::prelude::Cast;
use relm4::RelmWidgetExt;
use relm4::prelude::ComponentController;
use relm4::{Component, ComponentParts, ComponentSender, Controller};
use relm4::{adw, gtk};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::path::PathBuf;

use collomatique_ops::Desc;
use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;

use crate::editor::colloscope::ColloscopeOutput;
use crate::tools;

pub const DEFAULT_FILE_STEM: &str = "FichierSansNom";

/// The save target associated with the currently open document.
///
/// This distinguishes three situations that must drive display and save
/// behavior differently:
/// - [FileName::OkFile]: a clean file we can overwrite silently.
/// - [FileName::CaveatFile]: a file that was loaded with caveats (most
///   likely produced by a different version of Collomatique). We keep its
///   full path but must not overwrite it silently — "Enregistrer" behaves
///   like "Enregistrer sous".
/// - [FileName::NewFile]: a brand-new document that was never saved.
#[derive(Debug)]
pub enum FileName {
    OkFile(PathBuf),
    CaveatFile(PathBuf),
    NewFile,
}

impl FileName {
    /// The path backing this document, if any.
    ///
    /// Returns `Some` for both [FileName::OkFile] and [FileName::CaveatFile],
    /// and `None` for [FileName::NewFile].
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            FileName::OkFile(path) | FileName::CaveatFile(path) => Some(path),
            FileName::NewFile => None,
        }
    }
}

mod error_dialog;

mod advanced_tools;
mod assignments;
mod balancing;
mod check_script;
mod colloscope;
mod diagnostics;
mod export;
mod export_panel;
mod general_planning;
mod group_lists;
mod incompats;
mod pairings;
mod run_python_script;
mod run_solver;
mod settings;
mod slot_pairings;
mod slots;
mod students;
mod subjects;
mod teachers;
mod week_patterns;

mod warning_compact_ids;
mod warning_op;
mod warning_save_ids;

#[derive(Debug)]
pub enum EditorInput {
    Ignore,
    NewFile {
        file_name: FileName,
        data: collomatique_state_colloscopes::Data,
    },
    SaveCurrentFileAs(PathBuf),
    SaveCheckedFileAs(PathBuf, collomatique_state_colloscopes::InnerData),
    CompactAndSave,
    CancelSaveCompaction,
    CompactIdsClicked,
    CompactIds,
    SaveAsClicked,
    SaveClicked,
    UndoClicked,
    RedoClicked,
    UpdateOp(collomatique_ops::UpdateOp),
    CommitUpdateOp(
        collomatique_state::AppState<collomatique_state_colloscopes::Data, collomatique_ops::Desc>,
    ),
    ContinueOp,
    CancelOp,
    RunScriptClicked,
    RunScript(PathBuf, String),
    NewStateFromSecondInstance(AppState<Data, Desc>),
    UpdateFullColloscope(collomatique_state_colloscopes::colloscopes::Colloscope),
    ExportColloscopeAs(PathBuf, collomatique_xlsx::Config),
    ExportMpsClicked,
    ExportMpsAs(PathBuf),
    UpdateIlpProblem(Option<collomatique_constraints_colloscopes::IlpInnerProblem>),
}

#[derive(Debug)]
pub enum EditorOutput {
    UpdateActions,
    SaveError(PathBuf, String),
    PythonLoadingError(PathBuf, String),
    ExportError(PathBuf, String),
    StartOpenSaveDialog,
    EndOpenSaveDialog,
}

#[derive(Debug)]
pub enum EditorCommandOutput {
    FileNotChosen,
    FileChosen(PathBuf),
    SaveSuccessful(PathBuf),
    SaveFailed(PathBuf, String),
    ScriptChosen(PathBuf),
    ScriptNotChosen,
    ScriptLoaded(PathBuf, String),
    ScriptLoadingFailed(PathBuf, String),
    ExportXlsxSuccessful(PathBuf),
    ExportXlsxFailed(PathBuf, String),
    MpsFileChosen(PathBuf),
    MpsFileNotChosen,
    ExportMpsSuccessful(PathBuf),
    ExportMpsFailed(PathBuf, String),
}

const DEFAULT_TOAST_TIMEOUT: Option<NonZeroU32> = NonZeroU32::new(3);

enum ToastInfo {
    Toast {
        text: String,
        timeout: Option<NonZeroU32>,
    },
    Dismiss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
enum PanelNumbers {
    GeneralPlanning = 0,
    Subjects = 1,
    Teachers = 2,
    WeekPatterns = 3,
    Slots = 4,
    SlotPairings = 5,
    Pairings = 6,
    Incompats = 7,
    Students = 8,
    Assignments = 9,
    GroupLists = 10,
    ExtraSettings = 11,
    Balancing = 12,
    Colloscope = 13,
    Export = 14,
    AdvancedTools = 15,
}

impl PanelNumbers {
    fn iter() -> impl Iterator<Item = PanelNumbers> {
        [
            PanelNumbers::GeneralPlanning,
            PanelNumbers::Subjects,
            PanelNumbers::Teachers,
            PanelNumbers::WeekPatterns,
            PanelNumbers::Slots,
            PanelNumbers::SlotPairings,
            PanelNumbers::Pairings,
            PanelNumbers::Incompats,
            PanelNumbers::Students,
            PanelNumbers::Assignments,
            PanelNumbers::GroupLists,
            PanelNumbers::ExtraSettings,
            PanelNumbers::Balancing,
            PanelNumbers::Colloscope,
            PanelNumbers::Export,
            PanelNumbers::AdvancedTools,
        ]
        .into_iter()
    }

    fn panel_name(&self) -> &'static str {
        match self {
            PanelNumbers::GeneralPlanning => "general_planning",
            PanelNumbers::WeekPatterns => "week_patterns",
            PanelNumbers::Subjects => "subjects",
            PanelNumbers::Teachers => "teachers",
            PanelNumbers::Students => "students",
            PanelNumbers::Assignments => "assignments",
            PanelNumbers::Slots => "slots",
            PanelNumbers::SlotPairings => "slot_pairings",
            PanelNumbers::Incompats => "incompats",
            PanelNumbers::GroupLists => "group_lists",
            PanelNumbers::Pairings => "pairings",
            PanelNumbers::Balancing => "balancing",
            PanelNumbers::ExtraSettings => "extra_settings",
            PanelNumbers::Colloscope => "colloscope",
            PanelNumbers::Export => "export",
            PanelNumbers::AdvancedTools => "advanced_tools",
        }
    }

    fn panel_title(&self) -> &'static str {
        match self {
            PanelNumbers::GeneralPlanning => "Planning général",
            PanelNumbers::WeekPatterns => "Modèles de périodicité",
            PanelNumbers::Subjects => "Matières",
            PanelNumbers::Teachers => "Colleurs",
            PanelNumbers::Students => "Élèves",
            PanelNumbers::Assignments => "Inscriptions dans les matières",
            PanelNumbers::Slots => "Créneaux de colles",
            PanelNumbers::SlotPairings => "Appariements de créneaux",
            PanelNumbers::Incompats => "Incompatibilités horaires",
            PanelNumbers::GroupLists => "Groupes de colles",
            PanelNumbers::Pairings => "Appariements des matières",
            PanelNumbers::Balancing => "Équilibrage des colles",
            PanelNumbers::ExtraSettings => "Paramètres par élève",
            PanelNumbers::Colloscope => "Colloscope",
            PanelNumbers::Export => "Exporter",
            PanelNumbers::AdvancedTools => "Outils avancés",
        }
    }
}

pub struct EditorPanel {
    file_name: FileName,
    data: AppState<Data, Desc>,
    dirty: bool,
    toast_info: Option<ToastInfo>,
    pages_names: Vec<&'static str>,
    pages_titles_map: BTreeMap<&'static str, &'static str>,
    state_to_commit: Option<
        collomatique_state::AppState<collomatique_state_colloscopes::Data, collomatique_ops::Desc>,
    >,
    /// The save the id-compaction dialog is asking about
    ///
    /// Same pattern as [Self::state_to_commit]: the payload of a save
    /// that cannot go through as-is waits here while the user answers.
    save_pending_compaction: Option<(PathBuf, collomatique_state_colloscopes::InnerData)>,

    /// The current ILP problem, pushed here by the colloscope panel.
    ///
    /// It lives in the editor — and not in a panel — because two buttons
    /// export it now. The panels only ever receive its size.
    ilp_problem: Option<collomatique_constraints_colloscopes::IlpInnerProblem>,

    show_particular_panel: Option<PanelNumbers>,

    error_dialog: Controller<error_dialog::Dialog>,

    general_planning: Controller<general_planning::GeneralPlanning>,
    subjects: Controller<subjects::Subjects>,
    teachers: Controller<teachers::Teachers>,
    students: Controller<students::Students>,
    assignments: Controller<assignments::Assignments>,
    week_patterns: Controller<week_patterns::WeekPatterns>,
    slots: Controller<slots::Slots>,
    slot_pairings: Controller<slot_pairings::SlotPairings>,
    incompats: Controller<incompats::Incompats>,
    group_lists: Controller<group_lists::GroupLists>,
    pairings: Controller<pairings::Pairings>,
    settings: Controller<settings::Settings>,
    balancing: Controller<balancing::Balancing>,
    colloscope: Controller<colloscope::Colloscope>,
    export_panel: Controller<export_panel::ExportPanel>,
    advanced_tools: Controller<advanced_tools::AdvancedTools>,
    check_script_dialog: Controller<check_script::Dialog>,
    run_python_script_dialog: Controller<run_python_script::Dialog>,
    warning_op_dialog: Controller<warning_op::Dialog>,
    warning_save_ids_dialog: Controller<warning_save_ids::Dialog>,
    warning_compact_ids_dialog: Controller<warning_compact_ids::Dialog>,
}

impl EditorPanel {
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Whether a "save" is meaningful right now.
    ///
    /// True either because there are unsaved edits (`dirty`) or because there
    /// is no clean overwrite target yet (a new or caveat-loaded file). This
    /// drives the asterisk, the Save button sensitivity and the Save action —
    /// as opposed to [Self::is_dirty], which drives only the close guard.
    pub fn can_save(&self) -> bool {
        self.dirty || !matches!(self.file_name, FileName::OkFile(_))
    }

    pub fn can_undo(&self) -> bool {
        self.data.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.data.can_redo()
    }
}

impl EditorPanel {
    fn generate_subtitle(&self) -> String {
        let default_name = "Fichier sans nom".into();
        let name = match self.file_name.path() {
            Some(path) => match path.file_name() {
                Some(file_name) => file_name.to_string_lossy().to_string(),
                None => default_name,
            },
            None => default_name,
        };
        if self.can_save() {
            String::from("*") + &name
        } else {
            name
        }
    }

    fn generate_tooltip_text(&self) -> String {
        match self.file_name.path() {
            Some(x) => x.to_string_lossy().into(),
            None => "(Fichier non enregistré)".into(),
        }
    }

    /// Tooltip for the "Enregistrer" button.
    ///
    /// Only caveat-loaded files get an explanation: saving them will ask for a
    /// new location rather than overwriting the suspect original in place.
    fn save_button_tooltip(&self) -> Option<String> {
        match self.file_name {
            FileName::CaveatFile(_) => Some(
                "Ce fichier utilise un format qui n'est pas entièrement pris en charge (il provient probablement d'une version plus récente) : « Enregistrer » demandera un nouvel emplacement."
                    .into(),
            ),
            FileName::OkFile(_) | FileName::NewFile => None,
        }
    }

    fn send_msg_for_interface_update(&self, sender: ComponentSender<Self>) {
        sender.output(EditorOutput::UpdateActions).unwrap();
        self.general_planning
            .sender()
            .send(general_planning::GeneralPlanningInput::Update(
                self.data.get_data().get_inner_data().params.periods.clone(),
                self.data.get_data().get_inner_data().params.weeks.clone(),
            ))
            .unwrap();
        self.subjects
            .sender()
            .send(subjects::SubjectsInput::Update(
                self.data.get_data().get_inner_data().params.periods.clone(),
                self.data.get_data().get_inner_data().params.weeks.clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .clone(),
            ))
            .unwrap();
        self.teachers
            .sender()
            .send(teachers::TeachersInput::Update(
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .clone(),
            ))
            .unwrap();
        self.students
            .sender()
            .send(students::StudentsInput::Update(
                self.data.get_data().get_inner_data().params.periods.clone(),
                self.data.get_data().get_inner_data().params.weeks.clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .clone(),
            ))
            .unwrap();
        self.assignments
            .sender()
            .send(assignments::AssignmentsInput::Update(
                self.data.get_data().get_inner_data().params.periods.clone(),
                self.data.get_data().get_inner_data().params.weeks.clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .clone(),
            ))
            .unwrap();
        self.week_patterns
            .sender()
            .send(week_patterns::WeekPatternsInput::Update(
                self.data.get_data().get_inner_data().params.periods.clone(),
                self.data.get_data().get_inner_data().params.weeks.clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .week_patterns
                    .clone(),
            ))
            .unwrap();
        self.slots
            .sender()
            .send(slots::SlotsInput::Update(
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .week_patterns
                    .clone(),
                self.data.get_data().get_inner_data().params.slots.clone(),
            ))
            .unwrap();
        self.slot_pairings
            .sender()
            .send(slot_pairings::SlotPairingsInput::Update(
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .clone(),
                self.data.get_data().get_inner_data().params.slots.clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slot_pairings
                    .clone(),
                self.data.get_data().get_inner_data().params.periods.clone(),
            ))
            .unwrap();
        self.incompats
            .sender()
            .send(incompats::IncompatsInput::Update(
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .week_patterns
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .incompats
                    .clone(),
            ))
            .unwrap();
        self.pairings
            .sender()
            .send(pairings::PairingsInput::Update(
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .clone(),
                self.data.get_data().get_inner_data().params.periods.clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .pairings
                    .clone(),
            ))
            .unwrap();
        self.group_lists
            .sender()
            .send(group_lists::GroupListsInput::Update(
                self.data.get_data().get_inner_data().params.clone(),
            ))
            .unwrap();
        self.settings
            .sender()
            .send(settings::SettingsInput::Update(
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .settings
                    .clone(),
            ))
            .unwrap();
        self.balancing
            .sender()
            .send(balancing::BalancingInput::Update(
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .clone(),
                self.data
                    .get_data()
                    .get_inner_data()
                    .params
                    .balancing
                    .clone(),
            ))
            .unwrap();
        self.colloscope
            .sender()
            .send(colloscope::ColloscopeInput::Update(
                self.data.get_data().get_inner_data().params.clone(),
                self.data.get_data().get_inner_data().colloscope.clone(),
            ))
            .unwrap();
        let annotations: BTreeSet<String> = self
            .data
            .get_data()
            .get_inner_data()
            .params
            .walk_weeks()
            .filter_map(|(_, _, w)| w.annotation.as_ref().map(|a| a.to_string()))
            .collect();
        self.export_panel
            .sender()
            .send(export_panel::ExportPanelInput::Update(
                self.data.get_data().get_inner_data().export_config.clone(),
                self.file_name.path().cloned(),
                annotations,
            ))
            .unwrap();
        self.advanced_tools
            .sender()
            .send(advanced_tools::AdvancedToolsInput::Update(
                advanced_tools::Stats::from_inner_data(self.data.get_data().get_inner_data()),
            ))
            .unwrap();
    }

    fn op_cat_to_panel_number(op: &collomatique_ops::OpCategory) -> Option<PanelNumbers> {
        match op {
            collomatique_ops::OpCategory::None => None,
            collomatique_ops::OpCategory::GeneralPlanning => Some(PanelNumbers::GeneralPlanning),
            collomatique_ops::OpCategory::Subjects => Some(PanelNumbers::Subjects),
            collomatique_ops::OpCategory::Teachers => Some(PanelNumbers::Teachers),
            collomatique_ops::OpCategory::Students => Some(PanelNumbers::Students),
            collomatique_ops::OpCategory::Assignments => Some(PanelNumbers::Assignments),
            collomatique_ops::OpCategory::WeekPatterns => Some(PanelNumbers::WeekPatterns),
            collomatique_ops::OpCategory::Slots => Some(PanelNumbers::Slots),
            collomatique_ops::OpCategory::Incompatibilities => Some(PanelNumbers::Incompats),
            collomatique_ops::OpCategory::Pairings => Some(PanelNumbers::Pairings),
            collomatique_ops::OpCategory::SlotPairings => Some(PanelNumbers::SlotPairings),
            collomatique_ops::OpCategory::GroupLists => Some(PanelNumbers::GroupLists),
            collomatique_ops::OpCategory::Settings => Some(PanelNumbers::ExtraSettings),
            collomatique_ops::OpCategory::Balancing => Some(PanelNumbers::Balancing),
            collomatique_ops::OpCategory::Colloscope => Some(PanelNumbers::Colloscope),
            collomatique_ops::OpCategory::ExportConfig => Some(PanelNumbers::Export),
        }
    }

    fn generate_undo_tooltip(&self) -> String {
        match self.data.get_undo_name() {
            Some((_cat, desc)) => format!("Annuler \"{}\"", desc),
            None => "Rien à annuler".into(),
        }
    }

    fn generate_redo_tooltip(&self) -> String {
        match self.data.get_redo_name() {
            Some((_cat, desc)) => format!("Rétablir \"{}\"", desc),
            None => "Rien à rétablir".into(),
        }
    }
}

#[relm4::component(pub)]
impl Component for EditorPanel {
    type Input = EditorInput;
    type Output = EditorOutput;
    type Init = ();
    type CommandOutput = EditorCommandOutput;

    view! {
        #[root]
        nav_view = adw::NavigationSplitView {
            set_hexpand: true,
            set_vexpand: true,
            #[wrap(Some)]
            set_sidebar = &adw::NavigationPage {
                set_title: "Collomatique",
                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {
                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            set_title: "Collomatique",
                            #[watch]
                            set_subtitle: &model.generate_subtitle(),
                            #[watch]
                            set_tooltip_text: Some(&model.generate_tooltip_text()),
                        },
                        pack_end = &gtk::MenuButton {
                            set_icon_name: "open-menu-symbolic",
                            set_menu_model: Some(&main_menu),
                        },
                    },
                    #[wrap(Some)]
                    set_content = &gtk::Box {
                        set_vexpand: true,
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        gtk::StackSidebar {
                            set_vexpand: true,
                            set_size_request: (200, -1),
                            set_stack: &main_stack,
                        },
                    },
                },
            },
            #[wrap(Some)]
            set_content = &adw::NavigationPage {
                #[watch]
                set_title: match main_stack.visible_child_name() {
                    Some(n) => model.pages_titles_map.get(n.as_str()).unwrap(),
                    None => "Editor Panel",
                },
                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {
                        pack_start = &gtk::Box {
                            add_css_class: "linked",
                            gtk::Button {
                                set_icon_name: "edit-undo-symbolic",
                                #[watch]
                                set_sensitive: model.can_undo(),
                                #[watch]
                                set_tooltip_text: Some(&model.generate_undo_tooltip()),
                                connect_clicked => EditorInput::UndoClicked,
                            },
                            gtk::Button {
                                set_icon_name: "edit-redo-symbolic",
                                #[watch]
                                set_sensitive: model.can_redo(),
                                #[watch]
                                set_tooltip_text: Some(&model.generate_redo_tooltip()),
                                connect_clicked => EditorInput::RedoClicked,
                            },
                        },
                        pack_end = &gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "spacer",
                        },
                        pack_end = &gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "spacer",
                        },
                        pack_end = &gtk::Box {
                            add_css_class: "linked",
                            gtk::Button::with_label("Enregistrer") {
                                #[watch]
                                set_sensitive: model.can_save(),
                                #[watch]
                                set_tooltip_text: model.save_button_tooltip().as_deref(),
                                connect_clicked => EditorInput::SaveClicked,
                            },
                            gtk::Button {
                                set_icon_name: "document-save-as-symbolic",
                                set_tooltip_text: Some("Enregistrer sous"),
                                connect_clicked => EditorInput::SaveAsClicked,
                            },
                        },
                        pack_end = &gtk::Image {
                            set_icon_name: Some("dialog-warning-symbolic"),
                            set_tooltip: &super::in_dev_tooltip(),
                            set_visible: super::in_dev_shown(),
                        },
                    },
                    #[wrap(Some)]
                    #[name(toast_overlay)]
                    set_content = &adw::ToastOverlay {
                        #[name(main_stack)]
                        gtk::Stack {
                            set_hexpand: true,
                            set_transition_type: gtk::StackTransitionType::SlideUpDown,
                            // Force update_view when visible-child is changed
                            // This maintains the title up top
                            connect_notify: (
                                Some("visible-child"),
                                {
                                    let sender = sender.clone();
                                    move |_widget,_| {
                                        sender.input(EditorInput::Ignore);
                                    }
                                }
                            ),
                        },
                    },
                },
            },
        }
    }

    menu! {
        main_menu: {
            section! {
                "Nouveau" => super::NewAction,
                "Ouvrir" => super::OpenAction,
            },
            section! {
                "Annuler" => super::UndoAction,
                "Rétablir" => super::RedoAction,
            },
            section! {
                "Enregistrer" => super::SaveAction,
                "Enregistrer sous" => super::SaveAsAction,
            },
            section! {
                "Fermer" => super::CloseAction,
            },
            section! {
                "À propos" => super::AboutAction
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let general_planning = general_planning::GeneralPlanning::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::GeneralPlanning(op))
            });

        let subjects = subjects::Subjects::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Subjects(op))
            });

        let teachers = teachers::Teachers::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Teachers(op))
            });

        let students = students::Students::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Students(op))
            });

        let assignments = assignments::Assignments::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Assignments(op))
            });

        let week_patterns = week_patterns::WeekPatterns::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::WeekPatterns(op))
            });

        let slots = slots::Slots::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Slots(op))
            });

        let slot_pairings = slot_pairings::SlotPairings::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::SlotPairings(op))
            });

        let incompats = incompats::Incompats::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Incompatibilities(op))
            });

        let group_lists = group_lists::GroupLists::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::GroupLists(op))
            });

        let pairings = pairings::Pairings::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Pairings(op))
            });

        let settings = settings::Settings::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Settings(op))
            });

        let balancing = balancing::Balancing::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_ops::UpdateOp::Balancing(op))
            });

        let colloscope =
            colloscope::Colloscope::builder()
                .launch(())
                .forward(sender.input_sender(), |op| match op {
                    ColloscopeOutput::UpdateOp(op) => {
                        EditorInput::UpdateOp(collomatique_ops::UpdateOp::Colloscope(op))
                    }
                    ColloscopeOutput::NewColloscope(colloscope) => {
                        EditorInput::UpdateFullColloscope(colloscope)
                    }
                    ColloscopeOutput::UpdateIlpProblem(problem) => {
                        EditorInput::UpdateIlpProblem(problem)
                    }
                });

        let export_panel =
            export_panel::ExportPanel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    export_panel::ExportPanelOutput::ExportColloscopeAs(path, config) => {
                        EditorInput::ExportColloscopeAs(path, config)
                    }
                    export_panel::ExportPanelOutput::UpdateExportConfig(update_op) => {
                        EditorInput::UpdateOp(collomatique_ops::UpdateOp::ExportConfig(update_op))
                    }
                });

        let advanced_tools = advanced_tools::AdvancedTools::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                advanced_tools::AdvancedToolsOutput::RunPythonScriptClicked => {
                    EditorInput::RunScriptClicked
                }
                advanced_tools::AdvancedToolsOutput::ExportMpsClicked => {
                    EditorInput::ExportMpsClicked
                }
                advanced_tools::AdvancedToolsOutput::CompactIdsClicked => {
                    EditorInput::CompactIdsClicked
                }
            },
        );

        let check_script_dialog = check_script::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                check_script::DialogOutput::Run(path, script) => {
                    EditorInput::RunScript(path, script)
                }
            });

        let run_python_script_dialog = run_python_script::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                run_python_script::DialogOutput::NewData(new_data) => {
                    EditorInput::NewStateFromSecondInstance(new_data)
                }
            });

        let error_dialog = error_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let warning_op_dialog = warning_op::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                warning_op::DialogOutput::Continue => EditorInput::ContinueOp,
                warning_op::DialogOutput::Cancel => EditorInput::CancelOp,
            });

        let warning_save_ids_dialog = warning_save_ids::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                warning_save_ids::DialogOutput::Compact => EditorInput::CompactAndSave,
                warning_save_ids::DialogOutput::Cancel => EditorInput::CancelSaveCompaction,
            });

        let warning_compact_ids_dialog = warning_compact_ids::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                warning_compact_ids::DialogOutput::Compact => EditorInput::CompactIds,
            });

        let pages_names = PanelNumbers::iter().map(|x| x.panel_name()).collect();
        let pages_titles_map =
            BTreeMap::from_iter(PanelNumbers::iter().map(|x| (x.panel_name(), x.panel_title())));

        let model = EditorPanel {
            file_name: FileName::NewFile,
            data: AppState::new(Data::new()),
            dirty: false,
            toast_info: None,
            pages_names,
            pages_titles_map,
            show_particular_panel: None,
            state_to_commit: None,
            save_pending_compaction: None,
            ilp_problem: None,
            error_dialog,
            general_planning,
            subjects,
            teachers,
            students,
            assignments,
            week_patterns,
            slots,
            slot_pairings,
            incompats,
            group_lists,
            pairings,
            settings,
            balancing,
            colloscope,
            export_panel,
            advanced_tools,
            check_script_dialog,
            run_python_script_dialog,
            warning_op_dialog,
            warning_save_ids_dialog,
            warning_compact_ids_dialog,
        };
        let widgets = view_output!();

        for panel in PanelNumbers::iter() {
            let widget: gtk::Widget = match panel {
                PanelNumbers::GeneralPlanning => model.general_planning.widget().clone().upcast(),
                PanelNumbers::WeekPatterns => model.week_patterns.widget().clone().upcast(),
                PanelNumbers::Subjects => model.subjects.widget().clone().upcast(),
                PanelNumbers::Teachers => model.teachers.widget().clone().upcast(),
                PanelNumbers::Students => model.students.widget().clone().upcast(),
                PanelNumbers::Assignments => model.assignments.widget().clone().upcast(),
                PanelNumbers::Slots => model.slots.widget().clone().upcast(),
                PanelNumbers::SlotPairings => model.slot_pairings.widget().clone().upcast(),
                PanelNumbers::Incompats => model.incompats.widget().clone().upcast(),
                PanelNumbers::GroupLists => model.group_lists.widget().clone().upcast(),
                PanelNumbers::Pairings => model.pairings.widget().clone().upcast(),
                PanelNumbers::Balancing => model.balancing.widget().clone().upcast(),
                PanelNumbers::ExtraSettings => model.settings.widget().clone().upcast(),
                PanelNumbers::Colloscope => model.colloscope.widget().clone().upcast(),
                PanelNumbers::Export => model.export_panel.widget().clone().upcast(),
                PanelNumbers::AdvancedTools => model.advanced_tools.widget().clone().upcast(),
            };
            widgets
                .main_stack
                .add_titled(&widget, Some(panel.panel_name()), panel.panel_title());
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.show_particular_panel = None;
        match message {
            EditorInput::Ignore => {}
            EditorInput::NewFile { file_name, data } => {
                self.file_name = file_name;
                self.dirty = false;
                self.show_particular_panel = Some(PanelNumbers::GeneralPlanning);
                self.update_data(DataUpdate::Replace(AppState::new(data)));
                self.colloscope
                    .sender()
                    .send(colloscope::ColloscopeInput::ResetSolveConfig)
                    .unwrap();
                self.group_lists
                    .sender()
                    .send(group_lists::GroupListsInput::ResetGenerationConfig)
                    .unwrap();
                self.send_msg_for_interface_update(sender);
            }
            EditorInput::SaveClicked => match &self.file_name {
                // Only a clean file overwrites in place; new and caveat files
                // go through the Save-As dialog so nothing is silently clobbered.
                FileName::OkFile(path) => {
                    sender.input(EditorInput::SaveCurrentFileAs(path.clone()));
                }
                FileName::CaveatFile(_) | FileName::NewFile => {
                    sender.input(EditorInput::SaveAsClicked);
                }
            },
            EditorInput::SaveAsClicked => {
                let default_path = self.file_name.path().cloned();
                sender.output(EditorOutput::StartOpenSaveDialog).unwrap();
                sender.oneshot_command(async move {
                    match tools::open_save::save_collomatique_dialog(match default_path {
                        Some(path) => tools::open_save::DefaultSaveFile::ExistingFile(path),
                        None => tools::open_save::DefaultSaveFile::SuggestedName(
                            format!("{DEFAULT_FILE_STEM}.collomatique").into(),
                        ),
                    })
                    .await
                    {
                        Some(path) => EditorCommandOutput::FileChosen(path),
                        None => EditorCommandOutput::FileNotChosen,
                    }
                });
            }
            EditorInput::SaveCurrentFileAs(path) => {
                let inner_data = self.data.get_data().get_inner_data().clone();
                match collomatique_storage::check_encodable(&inner_data) {
                    Ok(()) => sender.input(EditorInput::SaveCheckedFileAs(path, inner_data)),
                    // The document's ids outgrew the file format. The
                    // dialog offers the one way out — compacting — and the
                    // payload waits here in the meantime, like
                    // state_to_commit does for destructive ops.
                    Err(collomatique_storage::EncodeError::IdAboveCeiling { .. }) => {
                        self.save_pending_compaction = Some((path, inner_data));
                        self.warning_save_ids_dialog
                            .sender()
                            .send(warning_save_ids::DialogInput::Show)
                            .unwrap();
                    }
                }
            }
            EditorInput::SaveCheckedFileAs(path, inner_data) => {
                self.dirty = false;
                // A successful save graduates any state to a clean file.
                self.file_name = FileName::OkFile(path.clone());
                self.send_msg_for_interface_update(sender.clone());

                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("Enregistrement en cours de {}...", path.to_string_lossy(),),
                    timeout: None,
                });
                sender.oneshot_command(async move {
                    match collomatique_storage::save_data_to_file(&inner_data, &path).await {
                        Ok(()) => EditorCommandOutput::SaveSuccessful(path),
                        Err(collomatique_storage::SaveError::IO(e)) => {
                            EditorCommandOutput::SaveFailed(path, e.to_string())
                        }
                        // This arm only receives documents that passed
                        // check_encodable (directly, or after compaction),
                        // so an encode error here is a programming bug,
                        // not a user situation.
                        Err(e @ collomatique_storage::SaveError::Encode(_)) => {
                            panic!("Cannot write the document: {e}")
                        }
                    }
                });
                sender.output(EditorOutput::UpdateActions).unwrap();
            }
            EditorInput::CompactAndSave => {
                if let Some((path, inner_data)) = self.save_pending_compaction.take() {
                    // Compaction renumbers every id densely from 0, so the
                    // result always fits the format. The compacted document
                    // replaces the current one: what is saved and what is
                    // being edited stay the same thing. Rebuilding Data
                    // resets the id issuer to just above the new dense ids
                    // — which is why the undo/redo history cannot survive:
                    // its entries hold ids the reset issuer would hand out
                    // again. The dialog warned about exactly this.
                    let compacted = inner_data.compact_ids();
                    let data = Data::from_inner_data(compacted.clone())
                        .expect("compaction preserves validity");
                    self.update_data(DataUpdate::Replace(AppState::new(data)));
                    sender.input(EditorInput::SaveCheckedFileAs(path, compacted));
                }
            }
            EditorInput::CancelSaveCompaction => {
                self.save_pending_compaction = None;
            }
            EditorInput::CompactIdsClicked => {
                self.warning_compact_ids_dialog
                    .sender()
                    .send(warning_compact_ids::DialogInput::Show)
                    .unwrap();
            }
            EditorInput::CompactIds => {
                // The same replacement CompactAndSave does, without the save.
                // Rebuilding Data resets the id issuer to just above the new
                // dense ids, which is what condemns the history: its entries
                // hold ids the issuer would hand out again. The dialog warned.
                let compacted = self.data.get_data().get_inner_data().clone().compact_ids();
                let data = Data::from_inner_data(compacted).expect("compaction preserves validity");
                self.update_data(DataUpdate::Replace(AppState::new(data)));
                // Nothing was written, so the document and the file now differ.
                self.dirty = true;
                self.toast_info = Some(ToastInfo::Toast {
                    text: "Identifiants compactés.".into(),
                    timeout: DEFAULT_TOAST_TIMEOUT,
                });
                self.send_msg_for_interface_update(sender);
            }
            EditorInput::UndoClicked => {
                if self.data.can_undo() {
                    let (cat, _) = self.data.get_undo_name().expect("Should be able to undo");
                    self.show_particular_panel = Self::op_cat_to_panel_number(cat);
                    self.update_data(DataUpdate::Undo);
                    self.dirty = true;
                    self.send_msg_for_interface_update(sender);
                }
            }
            EditorInput::RedoClicked => {
                if self.data.can_redo() {
                    let (cat, _) = self.data.get_redo_name().expect("Should be able to redo");
                    self.show_particular_panel = Self::op_cat_to_panel_number(cat);
                    self.update_data(DataUpdate::Redo);
                    self.dirty = true;
                    self.send_msg_for_interface_update(sender);
                }
            }
            EditorInput::UpdateOp(op) => {
                match op.dry_apply(&self.data) {
                    Ok(result) => {
                        if result.warnings.is_empty() {
                            sender.input(EditorInput::CommitUpdateOp(result.new_state));
                        } else {
                            // self.data still holds the pre-state the interface is
                            // showing, which is exactly the state a warning must be
                            // rendered against.
                            let texts: Vec<String> = result
                                .warnings
                                .iter()
                                .map(|w| {
                                    w.text(self.data.get_data())
                                        .expect("warning must render against the pre-state")
                                })
                                .collect();

                            // Rebuild the repair tree from the parent links. The
                            // warnings come in application order (a repair lands
                            // before the one that needed it), so filling the lists
                            // by ascending index leaves every sibling group in
                            // application order too.
                            let mut roots = Vec::new();
                            let mut children = vec![Vec::new(); result.warnings.len()];
                            for (i, warning) in result.warnings.iter().enumerate() {
                                match warning.parent() {
                                    Some(parent) => children[parent].push(i),
                                    None => roots.push(i),
                                }
                            }
                            let lines = warning_lines(&roots, &children, &texts, 0);

                            self.state_to_commit = Some(result.new_state);
                            self.warning_op_dialog
                                .sender()
                                .send(warning_op::DialogInput::Show(lines))
                                .unwrap();
                        }
                    }
                    Err(e) => {
                        self.error_dialog
                            .sender()
                            .send(error_dialog::DialogInput::Show(e.to_string()))
                            .unwrap();
                        // Update interface anyway, this is useful if we need to restore
                        // some GUI element to the correct state in case of error
                        self.send_msg_for_interface_update(sender);
                    }
                }
            }
            EditorInput::CommitUpdateOp(new_state) => {
                self.dirty = true;
                self.update_data(DataUpdate::Replace(new_state));
                self.send_msg_for_interface_update(sender);
            }
            EditorInput::ContinueOp => {
                if let Some(new_state) = self.state_to_commit.take() {
                    sender.input(EditorInput::CommitUpdateOp(new_state));
                }
            }
            EditorInput::CancelOp => {
                // Update interface
                // this is useful if we need to restore
                // some GUI element to the correct state
                // because of the cancelation
                self.send_msg_for_interface_update(sender);
            }
            EditorInput::RunScriptClicked => {
                sender.output(EditorOutput::StartOpenSaveDialog).unwrap();
                sender.oneshot_command(async move {
                    match tools::open_save::open_python_dialog().await {
                        Some(path) => EditorCommandOutput::ScriptChosen(path),
                        None => EditorCommandOutput::ScriptNotChosen,
                    }
                });
            }
            EditorInput::RunScript(path, script) => {
                self.run_python_script_dialog
                    .sender()
                    .send(run_python_script::DialogInput::Run(
                        path,
                        script,
                        self.data.clone(),
                    ))
                    .unwrap();
            }
            EditorInput::NewStateFromSecondInstance(new_data) => {
                self.update_data(DataUpdate::Replace(new_data));
                if let Some((cat, _desc)) = self.data.get_undo_name() {
                    self.show_particular_panel = Self::op_cat_to_panel_number(cat);
                }
                self.dirty = true;
                self.send_msg_for_interface_update(sender);
            }
            EditorInput::UpdateFullColloscope(new_colloscope) => {
                let mut inner = self.data.get_data().get_inner_data().clone();
                inner.colloscope = new_colloscope;
                let op = collomatique_state_colloscopes::Op::GlobalUpdate(inner);
                let desc = (
                    collomatique_ops::OpCategory::None,
                    "Résolution du colloscope".to_string(),
                );
                match Manager::apply(&mut self.data, op, desc) {
                    Ok(_) => {
                        self.dirty = true;
                        self.send_msg_for_interface_update(sender);
                    }
                    Err(e) => {
                        self.error_dialog
                            .sender()
                            .send(error_dialog::DialogInput::Show(e.to_string()))
                            .unwrap();
                    }
                }
            }
            EditorInput::ExportColloscopeAs(path, xlsx_config) => {
                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("Export en cours de {}...", path.to_string_lossy()),
                    timeout: None,
                });
                let inner_data = self.data.get_data().get_inner_data().clone();
                sender.spawn_oneshot_command(move || {
                    match export::export_to_xlsx(&inner_data, &path, &xlsx_config) {
                        Ok(()) => EditorCommandOutput::ExportXlsxSuccessful(path),
                        Err(e) => EditorCommandOutput::ExportXlsxFailed(path, e.to_string()),
                    }
                });
            }
            // The file chooser lives here rather than in a panel: both the
            // export panel and the advanced-tools panel offer this export, and
            // the editor is the one holding the problem to export.
            EditorInput::ExportMpsClicked => {
                let default = match self.file_name.path() {
                    Some(path) => {
                        let mut mps_path = path.clone();
                        mps_path.set_extension("mps");
                        tools::open_save::DefaultSaveFile::ExistingFile(mps_path)
                    }
                    None => tools::open_save::DefaultSaveFile::SuggestedName(
                        format!("{DEFAULT_FILE_STEM}.mps").into(),
                    ),
                };
                sender.oneshot_command(async move {
                    match tools::open_save::save_mps_dialog(default).await {
                        Some(path) => EditorCommandOutput::MpsFileChosen(path),
                        None => EditorCommandOutput::MpsFileNotChosen,
                    }
                });
            }
            EditorInput::ExportMpsAs(path) => {
                // Both buttons are insensitive without a problem, but the click
                // and the arrival of a `None` can cross: give up silently.
                let Some(problem) = self.ilp_problem.clone() else {
                    return;
                };
                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("Export MPS en cours de {}...", path.to_string_lossy()),
                    timeout: None,
                });
                sender.oneshot_command(async move {
                    match diagnostics::export_to_mps(&problem, &path).await {
                        Ok(()) => EditorCommandOutput::ExportMpsSuccessful(path),
                        Err(e) => EditorCommandOutput::ExportMpsFailed(path, e.to_string()),
                    }
                });
            }
            EditorInput::UpdateIlpProblem(problem) => {
                let info = problem
                    .as_ref()
                    .map(advanced_tools::IlpProblemInfo::from_problem);
                self.ilp_problem = problem;
                self.advanced_tools
                    .emit(advanced_tools::AdvancedToolsInput::UpdateIlpProblemInfo(
                        info,
                    ));
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            EditorCommandOutput::FileNotChosen => {
                sender.output(EditorOutput::EndOpenSaveDialog).unwrap();
            }
            EditorCommandOutput::FileChosen(path) => {
                sender.output(EditorOutput::EndOpenSaveDialog).unwrap();
                sender.input(EditorInput::SaveCurrentFileAs(path));
            }
            EditorCommandOutput::SaveSuccessful(path) => {
                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("{} enregistré", path.to_string_lossy()),
                    timeout: DEFAULT_TOAST_TIMEOUT,
                });
            }
            EditorCommandOutput::SaveFailed(path, error) => {
                if self.file_name.path() != Some(&path) {
                    return;
                }
                self.toast_info = Some(ToastInfo::Dismiss);
                self.dirty = true;
                sender.output(EditorOutput::UpdateActions).unwrap();
                sender.output(EditorOutput::SaveError(path, error)).unwrap();
            }
            EditorCommandOutput::ScriptChosen(path) => {
                sender.output(EditorOutput::EndOpenSaveDialog).unwrap();
                sender.oneshot_command(async move {
                    match tokio::fs::read_to_string(&path).await {
                        Ok(text) => EditorCommandOutput::ScriptLoaded(path, text),
                        Err(e) => EditorCommandOutput::ScriptLoadingFailed(path, e.to_string()),
                    }
                });
            }
            EditorCommandOutput::ScriptNotChosen => {
                sender.output(EditorOutput::EndOpenSaveDialog).unwrap();
            }
            EditorCommandOutput::ScriptLoaded(path, text) => {
                self.check_script_dialog
                    .sender()
                    .send(check_script::DialogInput::Show(path, text))
                    .unwrap();
            }
            EditorCommandOutput::ScriptLoadingFailed(path, error) => {
                sender
                    .output(EditorOutput::PythonLoadingError(path, error))
                    .unwrap();
            }
            EditorCommandOutput::ExportXlsxSuccessful(path) => {
                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("{} exporté", path.to_string_lossy()),
                    timeout: DEFAULT_TOAST_TIMEOUT,
                });
            }
            EditorCommandOutput::ExportXlsxFailed(path, error) => {
                self.toast_info = Some(ToastInfo::Dismiss);
                sender
                    .output(EditorOutput::ExportError(path, error))
                    .unwrap();
            }
            EditorCommandOutput::MpsFileNotChosen => {}
            EditorCommandOutput::MpsFileChosen(path) => {
                sender.input(EditorInput::ExportMpsAs(path));
            }
            EditorCommandOutput::ExportMpsSuccessful(path) => {
                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("{} exporté", path.to_string_lossy()),
                    timeout: DEFAULT_TOAST_TIMEOUT,
                });
            }
            EditorCommandOutput::ExportMpsFailed(path, error) => {
                self.toast_info = Some(ToastInfo::Dismiss);
                sender
                    .output(EditorOutput::ExportError(path, error))
                    .unwrap();
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update(message, sender.clone(), root);
        self.update_toast(widgets);
        self.update_view(widgets, sender);
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update_cmd(message, sender.clone(), root);
        self.update_toast(widgets);
        self.update_view(widgets, sender);
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if let Some(panel_number) = &model.show_particular_panel {
            widgets
                .main_stack
                .set_visible_child_name(self.pages_names[*panel_number as usize])
        }
    }
}

enum DataUpdate {
    Undo,
    Redo,
    Replace(AppState<Data, Desc>),
}

impl EditorPanel {
    fn update_data(&mut self, update: DataUpdate) {
        match update {
            DataUpdate::Undo => {
                self.data.undo().expect("Should be able to undo");
            }
            DataUpdate::Redo => {
                self.data.redo().expect("Should be able to redo");
            }
            DataUpdate::Replace(new_state) => {
                self.data = new_state;
            }
        }
    }
}

impl EditorPanel {
    fn update_toast(&mut self, widgets: &mut <Self as Component>::Widgets) {
        if let Some(toast_info) = self.toast_info.take() {
            widgets.toast_overlay.dismiss_all();
            match toast_info {
                ToastInfo::Toast { text, timeout } => {
                    let new_toast = adw::Toast::new(&text);
                    new_toast.set_timeout(match timeout {
                        Some(t) => t.get(),
                        None => 0,
                    });
                    widgets.toast_overlay.add_toast(new_toast);
                }
                ToastInfo::Dismiss => {} // Nothing else to do
            }
        }
    }
}

/// Flattens the repair tree for the warning dialog: each repair first, then the
/// repairs it needed one level deeper, siblings in application order.
///
/// Cause before consequence is the order a reader can follow — « le créneau
/// sera supprimé », then the interrogations that went with it — where the
/// engine's own order is the reverse, deepest first.
///
/// A sibling whose *whole rendered subtree* repeats an earlier sibling's is
/// dropped. The duplicates this removes come from a composite whose successive
/// ops trigger byte-identical repairs, and those land as root siblings with
/// identical subtrees. Deduplicating by line instead would break the tree: a
/// dropped parent leaves its children indented under nothing, and a child
/// dropped because the same sentence appeared under another parent hides a real
/// consequence of this one.
fn warning_lines(
    siblings: &[usize],
    children: &[Vec<usize>],
    texts: &[String],
    depth: usize,
) -> Vec<warning_op::WarningLine> {
    let mut seen = BTreeSet::new();
    let mut lines = Vec::new();

    for &idx in siblings {
        let mut subtree = vec![warning_op::WarningLine {
            text: texts[idx].clone(),
            depth,
        }];
        subtree.extend(warning_lines(&children[idx], children, texts, depth + 1));

        let key: Vec<(usize, String)> = subtree
            .iter()
            .map(|line| (line.depth, line.text.clone()))
            .collect();
        if seen.insert(key) {
            lines.extend(subtree);
        }
    }

    lines
}

/// Names a run of consecutive weeks — a period, or a block inside a subject.
///
/// Periods are named by [collomatique_ops::rendering::render_period], the
/// shared vocabulary the warning texts use; this helper survives for the one
/// caller that has no period to name at all, `subject_params::Block`, whose
/// blocks are a week succession with no id behind them.
fn generate_week_succession_title(
    name: &str,
    global_first_week: &Option<collomatique_time::WeekStart>,
    index: usize,
    first_week_num: usize,
    week_count: usize,
) -> String {
    if week_count == 0 {
        return format!("{} {} (vide)", name, index + 1);
    }

    let start_week = first_week_num + 1;
    let end_week = first_week_num + week_count;

    match global_first_week {
        Some(global_start_date) => {
            let start_date = global_start_date
                .monday()
                .checked_add_days(chrono::Days::new(7 * (first_week_num as u64)))
                .expect("Valid start date");
            let end_date = start_date
                .checked_add_days(chrono::Days::new(7 * (week_count as u64) - 1))
                .expect("Valid end date");
            if start_week != end_week {
                format!(
                    "{} {} du {} au {} (semaines {} à {})",
                    name,
                    index + 1,
                    start_date.format("%d/%m/%Y"),
                    end_date.format("%d/%m/%Y"),
                    start_week,
                    end_week,
                )
            } else {
                format!(
                    "{} {} du {} au {} (semaine {})",
                    name,
                    index + 1,
                    start_date.format("%d/%m/%Y"),
                    end_date.format("%d/%m/%Y"),
                    start_week,
                )
            }
        }
        None => {
            if start_week != end_week {
                format!(
                    "{} {} (semaines {} à {})",
                    name,
                    index + 1,
                    start_week,
                    end_week,
                )
            } else {
                format!("{} {} (semaine {})", name, index + 1, start_week,)
            }
        }
    }
}
