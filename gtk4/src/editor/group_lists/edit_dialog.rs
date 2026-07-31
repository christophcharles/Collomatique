use adw::prelude::{ComboRowExt, EditableExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::NonEmptyRangeInclusive;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PrefillMode {
    #[default]
    Automatic,
    Prefilled,
}

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,

    // Left pane: the general parameters
    selected_name: String,
    selected_students_per_group_minimum: u32,
    selected_students_per_group_maximum: u32,
    selected_max_group_count: u32,
    // Grow-only backing store: shrinking the group count only hides the tail,
    // so raising it again brings the names back.
    group_name_data: Vec<String>,
    group_name_entries: FactoryVecDeque<GroupNameEntry>,

    // Right pane: the filling
    prefill_mode: PrefillMode,
    filtered_students: BTreeMap<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student,
    >,
    available_students: BTreeSet<collomatique_state_colloscopes::StudentId>,

    // For Automatic mode: excluded students
    excluded_students: BTreeSet<collomatique_state_colloscopes::StudentId>,
    ordered_students: Vec<(collomatique_state_colloscopes::StudentId, String, String)>,
    student_exclusion_entries: FactoryVecDeque<StudentExclusionEntry>,

    // For Prefilled mode: group data. Grow-only, exactly like `group_name_data`.
    group_data: Vec<GroupEntryData>,
    group_entries: FactoryVecDeque<GroupEntry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::group_lists::GroupList,
        BTreeMap<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::students::Student,
        >,
    ),
    Cancel,
    Accept,

    UpdateSelectedName(String),
    UpdateStudentsPerGroupMinimum(u32),
    UpdateStudentsPerGroupMaximum(u32),
    UpdateMaxGroupCount(u32),
    UpdateGroupName(usize, String),

    UpdatePrefillMode(PrefillMode),
    UpdateStudentExclusion(usize, bool),
    UpdateGroup(usize, GroupEntryData),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::group_lists::GroupList),
}

impl Dialog {
    fn generate_prefill_mode_model() -> gtk::StringList {
        gtk::StringList::new(&["Remplir automatiquement", "Préremplir la liste"])
    }

    fn prefill_mode_to_selected(mode: PrefillMode) -> u32 {
        match mode {
            PrefillMode::Automatic => 0,
            PrefillMode::Prefilled => 1,
        }
    }

    fn selected_to_prefill_mode(selected: u32) -> PrefillMode {
        match selected {
            0 => PrefillMode::Automatic,
            _ => PrefillMode::Prefilled,
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Configuration de la liste de groupes"),
            set_default_size: (1000, 700),
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Paned {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_position: 500,
                    #[name(params_scrolled_window)]
                    #[wrap(Some)]
                    set_start_child = &gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        gtk::Box {
                            set_hexpand: true,
                            set_margin_all: 5,
                            set_spacing: 10,
                            set_orientation: gtk::Orientation::Vertical,
                            adw::PreferencesGroup {
                                set_title: "Paramètres généraux",
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[name(name_entry)]
                                adw::EntryRow {
                                    set_hexpand: true,
                                    set_title: "Nom de la liste",
                                    #[track(model.should_redraw)]
                                    set_text: &model.selected_name,
                                    connect_text_notify[sender] => move |widget| {
                                        let text : String = widget.text().into();
                                        sender.input(DialogInput::UpdateSelectedName(text));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Élèves par groupe",
                                set_description: Some("Nombre d'élève dans chaque groupe"),
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Minimum",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        #[watch]
                                        set_upper: model.selected_students_per_group_maximum as f64,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[track(model.should_redraw)]
                                    set_value: model.selected_students_per_group_minimum as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let students_per_group_min_u32 = widget.value() as u32;
                                        sender.input(DialogInput::UpdateStudentsPerGroupMinimum(students_per_group_min_u32));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Maximum",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        #[watch]
                                        set_lower: model.selected_students_per_group_minimum as f64,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[track(model.should_redraw)]
                                    set_value: model.selected_students_per_group_maximum as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let students_per_group_max_u32 = widget.value() as u32;
                                        sender.input(DialogInput::UpdateStudentsPerGroupMaximum(students_per_group_max_u32));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Groupes de colles",
                                set_description: Some("Nombre et noms des groupes"),
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Nombre de groupe",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[track(model.should_redraw)]
                                    set_value: model.selected_max_group_count as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let max_group_count = widget.value() as u32;
                                        sender.input(DialogInput::UpdateMaxGroupCount(max_group_count));
                                    },
                                },
                            },
                            #[local_ref]
                            group_name_entries_widget -> adw::PreferencesGroup {
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[watch]
                                set_visible: model.selected_max_group_count > 0,
                            },
                        },
                    },
                    #[name(prefill_scrolled_window)]
                    #[wrap(Some)]
                    set_end_child = &gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        gtk::Box {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_margin_all: 5,
                            set_spacing: 10,
                            set_orientation: gtk::Orientation::Vertical,
                            adw::PreferencesGroup {
                                set_title: "",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::ComboRow {
                                    set_title: "Mode de remplissage",
                                    #[track(model.should_redraw)]
                                    set_model: Some(&Dialog::generate_prefill_mode_model()),
                                    #[track(model.should_redraw)]
                                    set_selected: Dialog::prefill_mode_to_selected(model.prefill_mode),
                                    connect_selected_notify[sender] => move |widget| {
                                        let selected = widget.selected();
                                        let mode = Dialog::selected_to_prefill_mode(selected);
                                        sender.input(DialogInput::UpdatePrefillMode(mode));
                                    },
                                },
                            },
                            // Student exclusion UI for Automatic mode
                            #[local_ref]
                            student_exclusion_entries_widget -> adw::PreferencesGroup {
                                set_title: "Élèves dans la liste",
                                set_description: Some("Désactivez les élèves à exclure"),
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[watch]
                                set_visible: model.prefill_mode == PrefillMode::Automatic && !model.ordered_students.is_empty(),
                            },
                            // Prefilled groups UI
                            gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 0,
                                set_spacing: 10,
                                set_orientation: gtk::Orientation::Vertical,
                                #[watch]
                                set_visible: model.prefill_mode == PrefillMode::Prefilled,
                                #[local_ref]
                                entries_widget -> gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 0,
                                    set_spacing: 10,
                                    set_orientation: gtk::Orientation::Vertical,
                                },
                            },
                        },
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let group_name_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                GroupNameOutput::UpdateName(num, name) => DialogInput::UpdateGroupName(num, name),
            });

        let student_exclusion_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                StudentExclusionOutput::UpdateStatus(num, status) => {
                    DialogInput::UpdateStudentExclusion(num, status)
                }
            });

        let group_entries = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                GroupEntryOutput::UpdateGroup(index, group_data) => {
                    DialogInput::UpdateGroup(index, group_data)
                }
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            selected_name: String::new(),
            selected_students_per_group_minimum: 1,
            selected_students_per_group_maximum: u32::MAX,
            selected_max_group_count: 16,
            group_name_data: vec![String::new(); 16],
            group_name_entries,
            prefill_mode: PrefillMode::default(),
            filtered_students: BTreeMap::new(),
            available_students: BTreeSet::new(),
            excluded_students: BTreeSet::new(),
            ordered_students: vec![],
            student_exclusion_entries,
            group_data: vec![],
            group_entries,
        };

        let group_name_entries_widget = model.group_name_entries.widget();
        let student_exclusion_entries_widget = model.student_exclusion_entries.widget();
        let entries_widget = model.group_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(group_list_data, filtered_students) => {
                self.hidden = false;
                self.should_redraw = true;
                self.filtered_students = filtered_students;
                self.prefill_mode = match group_list_data.filling() {
                    collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                        ..
                    } => PrefillMode::Automatic,
                    collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled {
                        ..
                    } => PrefillMode::Prefilled,
                };
                self.update_from_data(group_list_data);

                self.update_ordered_students();
                self.update_group_name_entries();
                self.update_student_exclusion_entries();
                self.update_group_entries();
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                let group_list = collomatique_state_colloscopes::group_lists::GroupList::new(
                    self.generate_params(),
                    self.generate_filling(),
                )
                .expect("dialog maintains group count and student uniqueness by construction");
                sender.output(DialogOutput::Accepted(group_list)).unwrap();
            }
            DialogInput::UpdateSelectedName(name) => {
                if self.selected_name == name {
                    return;
                }
                self.selected_name = name;
            }
            DialogInput::UpdateStudentsPerGroupMinimum(selected_students_per_group_minimum) => {
                if self.selected_students_per_group_minimum == selected_students_per_group_minimum {
                    return;
                }
                self.selected_students_per_group_minimum = selected_students_per_group_minimum;
            }
            DialogInput::UpdateStudentsPerGroupMaximum(selected_students_per_group_maximum) => {
                if self.selected_students_per_group_maximum == selected_students_per_group_maximum {
                    return;
                }
                self.selected_students_per_group_maximum = selected_students_per_group_maximum;
            }
            DialogInput::UpdateMaxGroupCount(selected_max_group_count) => {
                if self.selected_max_group_count == selected_max_group_count {
                    return;
                }
                self.selected_max_group_count = selected_max_group_count;
                self.update_group_name_entries();
                // The prefill pane follows the count live: shrinking only hides
                // the tail groups (their students go back to the available pool),
                // growing brings them back with whatever they still hold.
                self.update_available_students();
                self.update_group_entries();
            }
            DialogInput::UpdateGroupName(group_num, name) => {
                if group_num < self.group_name_data.len() {
                    self.group_name_data[group_num] = name;
                }
                // The group titles on the right pane track the names on the left.
                self.update_group_entries();
            }
            DialogInput::UpdatePrefillMode(mode) => {
                if self.prefill_mode == mode {
                    return;
                }
                self.prefill_mode = mode;
            }
            DialogInput::UpdateStudentExclusion(student_num, included) => {
                assert!(student_num < self.ordered_students.len());
                let student_id = self.ordered_students[student_num].0;

                if included {
                    self.excluded_students.remove(&student_id);
                } else {
                    self.excluded_students.insert(student_id);
                }
            }
            DialogInput::UpdateGroup(index, group_data) => {
                assert!(index < self.group_data.len());
                if self.group_data[index] == group_data {
                    return;
                }
                self.group_data[index] = group_data;
                self.update_available_students();
                self.update_group_entries();
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            widgets.params_scrolled_window.vadjustment().set_value(0.);
            widgets.prefill_scrolled_window.vadjustment().set_value(0.);
            widgets.name_entry.grab_focus();
        }
    }
}

impl Dialog {
    fn group_count(&self) -> usize {
        self.selected_max_group_count as usize
    }

    fn group_name(&self, index: usize) -> Option<non_empty_string::NonEmptyString> {
        let name = self.group_name_data.get(index)?;
        non_empty_string::NonEmptyString::new(name.clone()).ok()
    }

    fn update_ordered_students(&mut self) {
        self.ordered_students = self
            .filtered_students
            .iter()
            .map(|(student_id, student)| {
                (
                    *student_id,
                    student.desc.firstname.clone(),
                    student.desc.surname.clone(),
                )
            })
            .collect();

        self.ordered_students
            .sort_by_key(|(id, firstname, surname)| (surname.clone(), firstname.clone(), *id));
    }

    fn update_from_data(&mut self, data: collomatique_state_colloscopes::group_lists::GroupList) {
        let params = data.params();
        self.selected_name = params.name.clone();
        self.selected_students_per_group_minimum = params.students_per_group.start().get();
        self.selected_students_per_group_maximum = params.students_per_group.end().get();
        self.selected_max_group_count = params.group_names.len() as u32;
        self.group_name_data = params
            .group_names
            .iter()
            .map(|opt| {
                opt.as_ref()
                    .map(|s| s.clone().into_inner())
                    .unwrap_or_default()
            })
            .collect();

        let group_count = self.group_count();

        match data.filling() {
            collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                excluded_students,
            } => {
                // Load excluded students
                self.excluded_students = excluded_students.clone();
                // Create empty group data for prefilled mode (in case user switches)
                self.available_students = self.filtered_students.keys().copied().collect();
                self.group_data = (0..group_count)
                    .map(|index| GroupEntryData {
                        group_name: self.group_name(index),
                        available_students: self.available_students.clone(),
                        filtered_students: self.filtered_students.clone(),
                        students: vec![],
                        selected_student_count: 0,
                    })
                    .collect();
            }
            collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
                // Clear excluded students for automatic mode (in case user switches)
                self.excluded_students = BTreeSet::new();
                // Load prefilled groups
                let selected_students: BTreeSet<_> = groups
                    .iter()
                    .flat_map(|g| g.students.iter().copied())
                    .collect();
                self.available_students = self
                    .filtered_students
                    .iter()
                    .filter_map(|(id, _student)| {
                        if selected_students.contains(id) {
                            return None;
                        }
                        Some(*id)
                    })
                    .collect();
                // Use data from prefilled groups (should match group_names.len())
                self.group_data = groups
                    .iter()
                    .enumerate()
                    .map(|(index, group)| GroupEntryData {
                        group_name: self.group_name(index),
                        available_students: self.available_students.clone(),
                        filtered_students: self.filtered_students.clone(),
                        students: group.students.iter().map(|x| Some(*x)).collect(),
                        selected_student_count: group.students.len() as u32,
                    })
                    .collect();
            }
        }
    }

    fn update_group_name_entries(&mut self) {
        let entries_count = self.group_count();

        // Resize group_name_data if needed
        if entries_count > self.group_name_data.len() {
            self.group_name_data.resize(entries_count, String::new());
        }

        // Sync factory with model
        crate::tools::factories::update_vec_deque(
            &mut self.group_name_entries,
            self.group_name_data
                .iter()
                .take(entries_count)
                .enumerate()
                .map(|(num, name)| GroupNameData {
                    name: name.clone(),
                    group_num: num,
                }),
            GroupNameInput::UpdateData,
        );
    }

    fn update_student_exclusion_entries(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.student_exclusion_entries,
            self.ordered_students
                .iter()
                .map(|(id, firstname, surname)| StudentExclusionData {
                    name: format!("{} {}", firstname, surname),
                    included: !self.excluded_students.contains(id),
                }),
            StudentExclusionInput::UpdateData,
        );
    }

    fn update_available_students(&mut self) {
        let entries_count = self.group_count();
        let selected_students: BTreeSet<_> = self
            .group_data
            .iter()
            .take(entries_count)
            .flat_map(|group| {
                let student_count = group.selected_student_count as usize;
                group
                    .students
                    .iter()
                    .take(student_count)
                    .filter_map(|student_opt| *student_opt)
            })
            .collect();
        self.available_students = self
            .filtered_students
            .iter()
            .filter_map(|(id, _student)| {
                if selected_students.contains(id) {
                    return None;
                }
                Some(*id)
            })
            .collect();

        let mut students_so_far = BTreeSet::new();
        for group in self.group_data.iter_mut().take(entries_count) {
            group.available_students = self.available_students.clone();
            let student_count = group.selected_student_count as usize;
            for student in group.students.iter_mut().take(student_count) {
                if let Some(s) = student {
                    if students_so_far.contains(s) {
                        *student = None;
                    } else {
                        students_so_far.insert(*s);
                    }
                }
            }
        }
    }

    fn update_group_entries(&mut self) {
        let entries_count = self.group_count();

        // Grow-only: entries past the current count are kept as they are, so
        // lowering then raising the group count restores their students.
        while self.group_data.len() < entries_count {
            let index = self.group_data.len();
            let group_name = self.group_name(index);
            self.group_data.push(GroupEntryData {
                group_name,
                available_students: self.available_students.clone(),
                students: vec![],
                selected_student_count: 0,
                filtered_students: self.filtered_students.clone(),
            });
        }

        // Keep the group titles in sync with the names typed on the left pane
        for (index, group) in self.group_data.iter_mut().enumerate().take(entries_count) {
            group.group_name = self
                .group_name_data
                .get(index)
                .and_then(|name| non_empty_string::NonEmptyString::new(name.clone()).ok());
        }

        crate::tools::factories::update_vec_deque(
            &mut self.group_entries,
            self.group_data.iter().take(entries_count).cloned(),
            GroupEntryInput::UpdateData,
        );
    }

    fn generate_params(&self) -> collomatique_state_colloscopes::group_lists::GroupListParameters {
        collomatique_state_colloscopes::group_lists::GroupListParameters {
            name: self.selected_name.clone(),
            students_per_group: NonEmptyRangeInclusive::new(
                NonZeroU32::new(self.selected_students_per_group_minimum).unwrap()
                    ..=NonZeroU32::new(self.selected_students_per_group_maximum).unwrap(),
            )
            .expect("spinners clamp min <= max"),
            group_names: self
                .group_name_data
                .iter()
                .take(self.group_count())
                .map(|s| non_empty_string::NonEmptyString::new(s.clone()).ok())
                .collect(),
        }
    }

    fn generate_filling(&self) -> collomatique_state_colloscopes::group_lists::GroupListFilling {
        match self.prefill_mode {
            PrefillMode::Automatic => {
                collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                    excluded_students: self.excluded_students.clone(),
                }
            }
            PrefillMode::Prefilled => {
                let entries_count = self.group_count();
                collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled {
                    groups: (0..entries_count)
                        .map(|index| {
                            let students = match self.group_data.get(index) {
                                Some(group) => {
                                    let student_count = group.selected_student_count as usize;
                                    group
                                        .students
                                        .iter()
                                        .take(student_count)
                                        .filter_map(|student| *student)
                                        .collect()
                                }
                                None => BTreeSet::new(),
                            };
                            collomatique_state_colloscopes::group_lists::PrefilledGroup { students }
                        })
                        .collect(),
                }
            }
        }
    }
}

// Group name entry factory component
#[derive(Debug, Clone)]
struct GroupNameData {
    name: String,
    group_num: usize,
}

#[derive(Debug)]
struct GroupNameEntry {
    data: GroupNameData,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum GroupNameInput {
    UpdateData(GroupNameData),
    UpdateName(String),
}

#[derive(Debug)]
enum GroupNameOutput {
    UpdateName(usize, String),
}

#[relm4::factory]
impl FactoryComponent for GroupNameEntry {
    type Init = GroupNameData;
    type Input = GroupNameInput;
    type Output = GroupNameOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::EntryRow {
            set_hexpand: true,
            #[watch]
            set_title: &format!("Nom du groupe {}", self.data.group_num + 1),
            #[track(self.should_redraw)]
            set_text: &self.data.name,
            connect_text_notify[sender] => move |widget| {
                let text: String = widget.text().into();
                sender.input(GroupNameInput::UpdateName(text));
            },
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            index: index.clone(),
            should_redraw: false,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.should_redraw = false;
        match msg {
            GroupNameInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            GroupNameInput::UpdateName(new_name) => {
                if self.data.name == new_name {
                    return;
                }
                self.data.name = new_name.clone();
                sender
                    .output(GroupNameOutput::UpdateName(
                        self.index.current_index(),
                        new_name,
                    ))
                    .unwrap();
            }
        }
    }
}

// Student exclusion entry for Automatic mode
#[derive(Debug, Clone)]
struct StudentExclusionData {
    name: String,
    included: bool,
}

#[derive(Debug)]
struct StudentExclusionEntry {
    data: StudentExclusionData,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum StudentExclusionInput {
    UpdateData(StudentExclusionData),
    UpdateStatus(bool),
}

#[derive(Debug)]
enum StudentExclusionOutput {
    UpdateStatus(usize, bool),
}

#[relm4::factory]
impl FactoryComponent for StudentExclusionEntry {
    type Init = StudentExclusionData;
    type Input = StudentExclusionInput;
    type Output = StudentExclusionOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::SwitchRow {
            set_hexpand: true,
            set_use_markup: false,
            #[watch]
            set_title: &self.data.name,
            #[track(self.should_redraw)]
            set_active: self.data.included,
            connect_active_notify[sender] => move |widget| {
                let status = widget.is_active();
                sender.input(StudentExclusionInput::UpdateStatus(status));
            },
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            index: index.clone(),
            should_redraw: false,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();
        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.should_redraw = false;
        match msg {
            StudentExclusionInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            StudentExclusionInput::UpdateStatus(new_status) => {
                if self.data.included == new_status {
                    return;
                }
                self.data.included = new_status;
                sender
                    .output(StudentExclusionOutput::UpdateStatus(
                        self.index.current_index(),
                        new_status,
                    ))
                    .unwrap();
            }
        }
    }
}

// Group entry for Prefilled mode
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEntryData {
    group_name: Option<non_empty_string::NonEmptyString>,
    selected_student_count: u32,
    students: Vec<Option<collomatique_state_colloscopes::StudentId>>,
    available_students: BTreeSet<collomatique_state_colloscopes::StudentId>,
    filtered_students: BTreeMap<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student,
    >,
}

struct GroupEntry {
    data: GroupEntryData,
    index: DynamicIndex,
    should_redraw: bool,
    student_entries: FactoryVecDeque<StudentEntry>,
}

#[derive(Clone, Debug)]
enum GroupEntryInput {
    UpdateData(GroupEntryData),

    UpdateSelectedStudentCount(u32),
    UpdateStudent(usize, Option<collomatique_state_colloscopes::StudentId>),
}

#[derive(Clone, Debug)]
enum GroupEntryOutput {
    UpdateGroup(usize, GroupEntryData),
}

impl GroupEntry {
    fn generate_group_title(&self) -> String {
        match &self.data.group_name {
            Some(name) => format!("Groupe {} : {}", self.index.current_index() + 1, name),
            None => format!("Groupe {}", self.index.current_index() + 1),
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for GroupEntry {
    type Init = GroupEntryData;
    type Input = GroupEntryInput;
    type Output = GroupEntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        root_widget = gtk::Box {
            set_hexpand: true,
            set_margin_all: 0,
            set_spacing: 10,
            set_orientation: gtk::Orientation::Vertical,
            adw::PreferencesGroup {
                #[watch]
                set_title: &self.generate_group_title(),
                set_margin_all: 5,
                set_hexpand: true,
                adw::SpinRow {
                    set_hexpand: true,
                    set_title: "Nombre d'élèves préremplis",
                    #[wrap(Some)]
                    set_adjustment = &gtk::Adjustment {
                        set_lower: 0.,
                        set_upper: u32::MAX as f64,
                        set_step_increment: 1.,
                        set_page_increment: 5.,
                    },
                    set_wrap: false,
                    set_snap_to_ticks: true,
                    set_numeric: true,
                    #[track(self.should_redraw)]
                    set_value: self.data.selected_student_count as f64,
                    connect_value_notify[sender] => move |widget| {
                        let value = widget.value() as u32;
                        sender.input(GroupEntryInput::UpdateSelectedStudentCount(value));
                    },
                },
            },
            #[local_ref]
            entries_widget -> adw::PreferencesGroup {
                set_margin_all: 5,
                set_hexpand: true,
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let student_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                StudentEntryOutput::UpdateStudent(index, student) => {
                    GroupEntryInput::UpdateStudent(index, student)
                }
            });

        let mut model = GroupEntry {
            data,
            index: index.clone(),
            should_redraw: false,
            student_entries,
        };

        model.update_entries();

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let entries_widget = self.student_entries.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.should_redraw = false;
        match msg {
            GroupEntryInput::UpdateData(new_data) => {
                if self.data == new_data {
                    return;
                }
                self.data = new_data;
                self.should_redraw = true;
                self.update_entries();
            }
            GroupEntryInput::UpdateSelectedStudentCount(selected_student_count) => {
                if self.data.selected_student_count == selected_student_count {
                    return;
                }
                self.data.selected_student_count = selected_student_count;
                self.update_entries();
                sender
                    .output(GroupEntryOutput::UpdateGroup(
                        self.index.current_index(),
                        self.data.clone(),
                    ))
                    .unwrap();
            }
            GroupEntryInput::UpdateStudent(index, student_opt) => {
                assert!(index < self.data.students.len());

                if self.data.students[index] == student_opt {
                    return;
                }

                self.data.students[index] = student_opt;
                sender
                    .output(GroupEntryOutput::UpdateGroup(
                        self.index.current_index(),
                        self.data.clone(),
                    ))
                    .unwrap();
            }
        }
    }
}

impl GroupEntry {
    fn update_entries(&mut self) {
        let entries_count = self.data.selected_student_count as usize;

        if entries_count > self.data.students.len() {
            self.data.students.resize(entries_count, None)
        }

        crate::tools::factories::update_vec_deque(
            &mut self.student_entries,
            self.data
                .students
                .iter()
                .take(entries_count)
                .map(|student| {
                    let mut available_students = self.data.available_students.clone();
                    if let Some(s) = student {
                        available_students.insert(*s);
                    }
                    StudentEntryData {
                        student: *student,
                        available_students,
                        filtered_students: self.data.filtered_students.clone(),
                    }
                }),
            StudentEntryInput::UpdateData,
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudentEntryData {
    student: Option<collomatique_state_colloscopes::StudentId>,
    available_students: BTreeSet<collomatique_state_colloscopes::StudentId>,
    filtered_students: BTreeMap<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student,
    >,
}

struct StudentEntry {
    data: StudentEntryData,
    index: DynamicIndex,
    should_update_list_model: bool,
    should_update_selected: bool,

    ordered_students: Vec<(collomatique_state_colloscopes::StudentId, String, String)>,
    selected_student: u32,
}

#[derive(Clone, Debug)]
enum StudentEntryInput {
    UpdateData(StudentEntryData),

    UpdateSelectedStudent(u32),
}

#[derive(Clone, Debug)]
enum StudentEntryOutput {
    UpdateStudent(usize, Option<collomatique_state_colloscopes::StudentId>),
}

impl StudentEntry {
    fn generate_entry_title(&self) -> String {
        format!("Élève {}", self.index.current_index() + 1)
    }

    fn generate_students_model(&self) -> gtk::StringList {
        let strings: Vec<_> = [String::from("(Non sélectionné)")]
            .into_iter()
            .chain(
                self.ordered_students
                    .iter()
                    .map(|(_id, firstname, lastname)| format!("{} {}", firstname, lastname)),
            )
            .collect();

        let str_ref: Vec<_> = strings.iter().map(|x| x.as_str()).collect();

        gtk::StringList::new(&str_ref[..])
    }

    fn student_selected_to_enum(
        &self,
        selected: u32,
    ) -> Option<collomatique_state_colloscopes::StudentId> {
        if selected == 0 {
            return None;
        }

        let student_num = (selected - 1) as usize;
        Some(self.ordered_students[student_num].0)
    }

    fn student_enum_to_selected(
        &self,
        student_opt: Option<collomatique_state_colloscopes::StudentId>,
    ) -> u32 {
        let Some(student) = student_opt else {
            return 0;
        };

        for (i, (id, _, _)) in self.ordered_students.iter().enumerate() {
            if *id == student {
                return (i as u32) + 1;
            }
        }

        panic!("Student ID should be valid");
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for StudentEntry {
    type Init = StudentEntryData;
    type Input = StudentEntryInput;
    type Output = StudentEntryOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        root_widget = adw::ComboRow {
            set_title: &self.generate_entry_title(),
            #[track(self.should_update_list_model)]
            set_model: Some(&self.generate_students_model()),
            #[track(self.should_update_selected)]
            set_selected: self.selected_student,
            connect_selected_notify[sender] => move |widget| {
                let selected = widget.selected();
                sender.input(StudentEntryInput::UpdateSelectedStudent(selected));
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let mut model = StudentEntry {
            data,
            index: index.clone(),
            should_update_list_model: false,
            should_update_selected: false,
            ordered_students: vec![],
            selected_student: 0,
        };
        model.update_from_data();
        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        if self.should_update_list_model {
            self.should_update_selected = true;
            self.should_update_list_model = false;
            return;
        }
        self.should_update_selected = false;
        match msg {
            StudentEntryInput::UpdateData(new_data) => {
                if self.data == new_data {
                    return;
                }
                self.data = new_data;
                self.should_update_list_model = true;
                self.update_from_data();
            }
            StudentEntryInput::UpdateSelectedStudent(selected_student) => {
                if self.selected_student == selected_student {
                    return;
                }
                self.selected_student = selected_student;
                self.update_data_from_selected();
                sender
                    .output(StudentEntryOutput::UpdateStudent(
                        self.index.current_index(),
                        self.data.student,
                    ))
                    .unwrap();
            }
        }
    }
}

impl StudentEntry {
    fn update_data_from_selected(&mut self) {
        self.data.student = self.student_selected_to_enum(self.selected_student);
        if let Some(student) = &self.data.student {
            self.data.available_students.insert(*student);
        }
    }

    fn update_from_data(&mut self) {
        self.update_ordered_students();
        self.update_selected_student();
    }

    fn update_ordered_students(&mut self) {
        if let Some(student) = &self.data.student {
            assert!(self.data.available_students.contains(student));
        }

        self.ordered_students = self
            .data
            .available_students
            .iter()
            .map(|id| {
                let student = self
                    .data
                    .filtered_students
                    .get(id)
                    .expect("Student id should be valid");

                (
                    *id,
                    student.desc.firstname.clone(),
                    student.desc.surname.clone(),
                )
            })
            .collect();

        self.ordered_students
            .sort_by_key(|(id, firstname, surname)| (surname.clone(), firstname.clone(), *id));
    }

    fn update_selected_student(&mut self) {
        self.selected_student = self.student_enum_to_selected(self.data.student);
    }
}
