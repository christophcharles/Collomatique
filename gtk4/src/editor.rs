use adw::prelude::NavigationPageExt;
use collomatique_state::traits::Manager;
use gtk::prelude::{ButtonExt, ObjectExt, OrientableExt, WidgetExt};
use relm4::prelude::{ComponentController, RelmWidgetExt};
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, Controller};
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::PathBuf;

use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;

use crate::tools;

mod error_dialog;

mod assignments;
mod check_script;
mod general_planning;
mod run_script;
mod students;
mod subjects;
mod teachers;

#[derive(Debug)]
pub enum EditorInput {
    Ignore,
    NewFile {
        file_name: Option<PathBuf>,
        data: collomatique_state_colloscopes::Data,
        dirty: bool,
    },
    SaveCurrentFileAs(PathBuf),
    SaveAsClicked,
    SaveClicked,
    UndoClicked,
    RedoClicked,
    UpdateOp(collomatique_core::ops::UpdateOp),
    RunScriptClicked,
    RunScript(PathBuf, String),
    NewStateFromScript(AppState<Data>),
}

#[derive(Debug)]
pub enum EditorOutput {
    UpdateActions,
    SaveError(PathBuf, String),
    PythonLoadingError(PathBuf, String),
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
    Students = 3,
    Assignments = 4,
}

pub struct EditorPanel {
    file_name: Option<PathBuf>,
    data: AppState<Data>,
    dirty: bool,
    toast_info: Option<ToastInfo>,
    pages_names: Vec<&'static str>,
    pages_titles_map: BTreeMap<&'static str, &'static str>,

    show_particular_panel: Option<PanelNumbers>,

    error_dialog: Controller<error_dialog::Dialog>,

    general_planning: Controller<general_planning::GeneralPlanning>,
    subjects: Controller<subjects::Subjects>,
    teachers: Controller<teachers::Teachers>,
    students: Controller<students::Students>,
    assignments: Controller<assignments::Assignments>,
    check_script_dialog: Controller<check_script::Dialog>,
    run_script_dialog: Controller<run_script::Dialog>,
}

impl EditorPanel {
    pub fn is_dirty(&self) -> bool {
        self.dirty
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
        let name = match &self.file_name {
            Some(path) => match path.file_name() {
                Some(file_name) => file_name.to_string_lossy().to_string(),
                None => default_name,
            },
            None => default_name,
        };
        if self.dirty {
            String::from("*") + &name
        } else {
            name
        }
    }

    fn generate_tooltip_text(&self) -> String {
        match &self.file_name {
            Some(x) => x.to_string_lossy().into(),
            None => "(Fichier non enregistré)".into(),
        }
    }

    fn send_msg_for_interface_update(&self, sender: ComponentSender<Self>) {
        sender.output(EditorOutput::UpdateActions).unwrap();
        self.general_planning
            .sender()
            .send(general_planning::GeneralPlanningInput::Update(
                self.data.get_data().get_periods().clone(),
            ))
            .unwrap();
        self.subjects
            .sender()
            .send(subjects::SubjectsInput::Update(
                self.data.get_data().get_periods().clone(),
                self.data.get_data().get_subjects().clone(),
            ))
            .unwrap();
        self.teachers
            .sender()
            .send(teachers::TeachersInput::Update(
                self.data.get_data().get_subjects().clone(),
                self.data.get_data().get_teachers().clone(),
            ))
            .unwrap();
        self.students
            .sender()
            .send(students::StudentsInput::Update(
                self.data.get_data().get_periods().clone(),
                self.data.get_data().get_students().clone(),
            ))
            .unwrap();
        self.assignments
            .sender()
            .send(assignments::AssignmentsInput::Update(
                self.data.get_data().get_periods().clone(),
                self.data.get_data().get_subjects().clone(),
                self.data.get_data().get_students().clone(),
                self.data.get_data().get_assignments().clone(),
            ))
            .unwrap();
    }

    fn inner_op_to_panel_number(
        op: &collomatique_state_colloscopes::AnnotatedOp,
    ) -> Option<PanelNumbers> {
        match op {
            collomatique_state_colloscopes::AnnotatedOp::Period(_) => {
                Some(PanelNumbers::GeneralPlanning)
            }
            collomatique_state_colloscopes::AnnotatedOp::Subject(_) => Some(PanelNumbers::Subjects),
            collomatique_state_colloscopes::AnnotatedOp::Teacher(_) => Some(PanelNumbers::Teachers),
            collomatique_state_colloscopes::AnnotatedOp::Student(_) => Some(PanelNumbers::Students),
            collomatique_state_colloscopes::AnnotatedOp::Assignment(_) => {
                Some(PanelNumbers::Assignments)
            }
            collomatique_state_colloscopes::AnnotatedOp::WeekPattern(_) => None,
        }
    }

    fn generate_undo_tooltip(&self) -> String {
        match self.data.get_undo_name() {
            Some(x) => format!("Annuler \"{}\"", x),
            None => "Rien à annuler".into(),
        }
    }

    fn generate_redo_tooltip(&self) -> String {
        match self.data.get_redo_name() {
            Some(x) => format!("Rétablir \"{}\"", x),
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
                        gtk::Box {
                            set_vexpand: true,
                        },
                        gtk::Button {
                            set_hexpand: true,
                            set_size_request: (-1,50),
                            add_css_class: "frame",
                            add_css_class: "warning",
                            set_margin_all: 5,
                            adw::ButtonContent {
                                set_icon_name: "text-x-script",
                                set_label: "Exécuter un script",
                            },
                            connect_clicked => EditorInput::RunScriptClicked,
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
                                set_icon_name: "edit-undo",
                                #[watch]
                                set_sensitive: model.can_undo(),
                                #[watch]
                                set_tooltip_text: Some(&model.generate_undo_tooltip()),
                                connect_clicked => EditorInput::UndoClicked,
                            },
                            gtk::Button {
                                set_icon_name: "edit-redo",
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
                                set_sensitive: model.dirty,
                                connect_clicked => EditorInput::SaveClicked,
                            },
                            gtk::Button {
                                set_icon_name: "document-save-as",
                                set_tooltip_text: Some("Enregistrer sous"),
                                connect_clicked => EditorInput::SaveAsClicked,
                            },
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
                EditorInput::UpdateOp(collomatique_core::ops::UpdateOp::GeneralPlanning(op))
            });

        let subjects = subjects::Subjects::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_core::ops::UpdateOp::Subjects(op))
            });

        let teachers = teachers::Teachers::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_core::ops::UpdateOp::Teachers(op))
            });

        let students = students::Students::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_core::ops::UpdateOp::Students(op))
            });

        let assignments = assignments::Assignments::builder()
            .launch(())
            .forward(sender.input_sender(), |op| {
                EditorInput::UpdateOp(collomatique_core::ops::UpdateOp::Assignments(op))
            });

        let check_script_dialog = check_script::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                check_script::DialogOutput::Run(path, script) => {
                    EditorInput::RunScript(path, script)
                }
            });

        let run_script_dialog = run_script::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                run_script::DialogOutput::NewData(new_data) => {
                    EditorInput::NewStateFromScript(new_data)
                }
            });

        let error_dialog = error_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let pages_names = vec![
            "general_planning",
            "subjects",
            "teachers",
            "students",
            "assignments",
        ];
        let pages_titles_map = BTreeMap::from([
            ("general_planning", "Planning général"),
            ("subjects", "Matières"),
            ("teachers", "Colleurs"),
            ("students", "Élèves"),
            ("assignments", "Inscriptions dans les matières"),
        ]);

        let model = EditorPanel {
            file_name: None,
            data: AppState::new(Data::new()),
            dirty: false,
            toast_info: None,
            pages_names,
            pages_titles_map,
            show_particular_panel: None,
            error_dialog,
            general_planning,
            subjects,
            teachers,
            students,
            assignments,
            check_script_dialog,
            run_script_dialog,
        };
        let widgets = view_output!();

        widgets.main_stack.add_titled(
            model.general_planning.widget(),
            Some(model.pages_names[0]),
            model.pages_titles_map.get(model.pages_names[0]).unwrap(),
        );
        widgets.main_stack.add_titled(
            model.subjects.widget(),
            Some(model.pages_names[1]),
            model.pages_titles_map.get(model.pages_names[1]).unwrap(),
        );
        widgets.main_stack.add_titled(
            model.teachers.widget(),
            Some(model.pages_names[2]),
            model.pages_titles_map.get(model.pages_names[2]).unwrap(),
        );
        widgets.main_stack.add_titled(
            model.students.widget(),
            Some(model.pages_names[3]),
            model.pages_titles_map.get(model.pages_names[3]).unwrap(),
        );
        widgets.main_stack.add_titled(
            model.assignments.widget(),
            Some(model.pages_names[4]),
            model.pages_titles_map.get(model.pages_names[4]).unwrap(),
        );

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.show_particular_panel = None;
        match message {
            EditorInput::Ignore => {}
            EditorInput::NewFile {
                file_name,
                data,
                dirty,
            } => {
                self.file_name = file_name;
                self.data = AppState::new(data);
                self.dirty = dirty;
                self.send_msg_for_interface_update(sender);
            }
            EditorInput::SaveClicked => match &self.file_name {
                Some(path) => {
                    sender.input(EditorInput::SaveCurrentFileAs(path.clone()));
                }
                None => {
                    sender.input(EditorInput::SaveAsClicked);
                }
            },
            EditorInput::SaveAsClicked => {
                let file_name = self.file_name.clone();
                sender.output(EditorOutput::StartOpenSaveDialog).unwrap();
                sender.oneshot_command(async move {
                    match tools::open_save::save_dialog(match &file_name {
                        Some(path) => tools::open_save::DefaultSaveFile::ExistingFile(path.clone()),
                        None => tools::open_save::DefaultSaveFile::SuggestedName(
                            "FichierSansNom.collomatique".into(),
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
                let data_copy = self.data.get_data().clone();
                self.dirty = false;
                self.file_name = Some(path.clone());
                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("Enregistrement en cours de {}...", path.to_string_lossy(),),
                    timeout: None,
                });
                sender.oneshot_command(async move {
                    match collomatique_storage::save_data_to_file(&data_copy, &path).await {
                        Ok(()) => EditorCommandOutput::SaveSuccessful(path),
                        Err(e) => EditorCommandOutput::SaveFailed(path, e.to_string()),
                    }
                });
                sender.output(EditorOutput::UpdateActions).unwrap();
            }
            EditorInput::UndoClicked => {
                if self.data.can_undo() {
                    let aggregated_op = self.data.undo().expect("Should be able to undo");
                    let first_action = aggregated_op.inner().first();
                    self.show_particular_panel = first_action
                        .map(|x| Self::inner_op_to_panel_number(x.inner()))
                        .flatten();
                    self.dirty = true;
                    self.send_msg_for_interface_update(sender);
                }
            }
            EditorInput::RedoClicked => {
                if self.data.can_redo() {
                    let aggregated_op = self.data.redo().expect("Should be able to undo");
                    let last_action = aggregated_op.inner().last();
                    self.show_particular_panel = last_action
                        .map(|x| Self::inner_op_to_panel_number(x.inner()))
                        .flatten();
                    self.dirty = true;
                    self.send_msg_for_interface_update(sender);
                }
            }
            EditorInput::UpdateOp(op) => {
                match op.apply(&mut self.data) {
                    Ok(_) => {
                        self.dirty = true;
                    }
                    Err(e) => {
                        self.error_dialog
                            .sender()
                            .send(error_dialog::DialogInput::Show(e.to_string()))
                            .unwrap();
                    }
                }
                // Update interface anyway, this is useful if we need to restore
                // some GUI element to the correct state in case of error
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
                self.run_script_dialog
                    .sender()
                    .send(run_script::DialogInput::Run(
                        path,
                        script,
                        self.data.clone(),
                    ))
                    .unwrap();
            }
            EditorInput::NewStateFromScript(new_data) => {
                self.data = new_data;
                if let Some(last_op) = self.data.get_last_op() {
                    let last_action = last_op.inner().last();
                    self.show_particular_panel = last_action
                        .map(|x| Self::inner_op_to_panel_number(x.inner()))
                        .flatten();
                }
                self.dirty = true;
                self.send_msg_for_interface_update(sender);
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
                if Some(&path) != self.file_name.as_ref() {
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

fn generate_period_title(
    global_first_week: &Option<collomatique_time::NaiveMondayDate>,
    index: usize,
    first_week_num: usize,
    week_count: usize,
) -> String {
    generate_week_succession_title(
        "Période",
        global_first_week,
        index,
        first_week_num,
        week_count,
    )
}

fn generate_week_succession_title(
    name: &str,
    global_first_week: &Option<collomatique_time::NaiveMondayDate>,
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
                .inner()
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
                    start_date.format("%d/%m/%Y").to_string(),
                    end_date.format("%d/%m/%Y").to_string(),
                    start_week,
                    end_week,
                )
            } else {
                format!(
                    "{} {} du {} au {} (semaine {})",
                    name,
                    index + 1,
                    start_date.format("%d/%m/%Y").to_string(),
                    end_date.format("%d/%m/%Y").to_string(),
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
