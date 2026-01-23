use adw::prelude::{AdjustmentExt, ComboRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

use std::collections::{BTreeMap, BTreeSet};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    list_name: String,
    group_names: Vec<Option<non_empty_string::NonEmptyString>>,
    filtered_students: BTreeMap<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student,
    >,
    available_students: BTreeSet<collomatique_state_colloscopes::StudentId>,

    selected_group_count: u32,
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

    UpdateSelectedGroupCount(u32),
    UpdateGroup(usize, GroupEntryData),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups),
}

impl Dialog {
    fn generate_list_name(&self) -> String {
        format!("Liste concernée : {}", self.list_name)
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
            set_title: Some("Préremplissage de la liste de groupes"),
            set_default_size: (500, 700),
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
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 5,
                    set_spacing: 10,
                    set_orientation: gtk::Orientation::Vertical,
                    #[name(scrolled_window)]
                    gtk::ScrolledWindow {
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
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Nombre de groupes",
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
                                    set_value: model.selected_group_count as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateSelectedGroupCount(value));
                                    },
                                },
                            },
                            #[local_ref]
                            entries_widget -> gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 0,
                                set_spacing: 10,
                                set_orientation: gtk::Orientation::Vertical,
                            },
                        },
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        #[watch]
                        set_label: &model.generate_list_name(),
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
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
            filtered_students: BTreeMap::new(),
            selected_group_count: 0,
            group_data: vec![],
            group_entries,
            available_students: BTreeSet::new(),
            list_name: String::new(),
            group_names: vec![],
        };

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
                self.update_from_data(group_list_data);
                self.update_group_entries();
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.generate_data()))
                    .unwrap();
            }
            DialogInput::UpdateSelectedGroupCount(selected_group_count) => {
                if self.selected_group_count == selected_group_count {
                    return;
                }
                self.selected_group_count = selected_group_count;
                self.update_available_students();
                self.update_group_entries();
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
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
        }
    }
}

impl Dialog {
    fn update_from_data(&mut self, data: collomatique_state_colloscopes::group_lists::GroupList) {
        let selected_students: BTreeSet<_> = data.prefilled_groups.iter_students().collect();
        self.list_name = data.params.name.clone();
        self.group_names = data.params.group_names.clone();
        self.available_students = self
            .filtered_students
            .iter()
            .filter_map(|(id, _student)| {
                if selected_students.contains(id) {
                    return None;
                }
                Some(id.clone())
            })
            .collect();
        self.selected_group_count = data.prefilled_groups.groups.len() as u32;
        self.group_data = data
            .prefilled_groups
            .groups
            .iter()
            .enumerate()
            .map(|(index, group)| GroupEntryData {
                sealed: group.sealed,
                group_name: self.group_names.get(index).cloned().flatten(),
                available_students: self.available_students.clone(),
                filtered_students: self.filtered_students.clone(),
                students: group.students.iter().map(|x| Some(x.clone())).collect(),
                selected_student_count: group.students.len() as u32,
            })
            .collect();
    }

    fn update_available_students(&mut self) {
        let entries_count = self.selected_group_count as usize;
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
                    .filter_map(|student_opt| student_opt.clone())
            })
            .collect();
        self.available_students = self
            .filtered_students
            .iter()
            .filter_map(|(id, _student)| {
                if selected_students.contains(id) {
                    return None;
                }
                Some(id.clone())
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
        let entries_count = self.selected_group_count as usize;

        while self.group_data.len() < entries_count {
            let index = self.group_data.len();
            self.group_data.push(GroupEntryData {
                sealed: false,
                group_name: self.group_names.get(index).cloned().flatten(),
                available_students: self.available_students.clone(),
                students: vec![],
                selected_student_count: 0,
                filtered_students: self.filtered_students.clone(),
            });
        }

        crate::tools::factories::update_vec_deque(
            &mut self.group_entries,
            self.group_data.iter().take(entries_count).cloned(),
            |x| GroupEntryInput::UpdateData(x),
        );
    }

    fn generate_data(
        &self,
    ) -> collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups {
        let entries_count = self.selected_group_count as usize;
        collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups {
            groups: self
                .group_data
                .iter()
                .take(entries_count)
                .map(|group| {
                    let student_count = group.selected_student_count as usize;
                    collomatique_state_colloscopes::group_lists::PrefilledGroup {
                        sealed: group.sealed,
                        students: group
                            .students
                            .iter()
                            .take(student_count)
                            .filter_map(|student| student.clone())
                            .collect(),
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEntryData {
    sealed: bool,
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

    UpdateSealed(bool),
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
                adw::SwitchRow {
                    set_hexpand: true,
                    set_use_markup: false,
                    set_title: "Groupe scellé",
                    #[track(self.should_redraw)]
                    set_active: self.data.sealed,
                    connect_active_notify[sender] => move |widget| {
                        let sealed = widget.is_active();
                        sender.input(GroupEntryInput::UpdateSealed(sealed));
                    },
                },
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
            GroupEntryInput::UpdateSealed(new_sealed) => {
                if self.data.sealed == new_sealed {
                    return;
                }
                self.data.sealed = new_sealed;
                sender
                    .output(GroupEntryOutput::UpdateGroup(
                        self.index.current_index(),
                        self.data.clone(),
                    ))
                    .unwrap();
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
                        student: student.clone(),
                        available_students,
                        filtered_students: self.data.filtered_students.clone(),
                    }
                }),
            |x| StudentEntryInput::UpdateData(x),
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
                let selected = widget.selected() as u32;
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
                    id.clone(),
                    student.desc.firstname.clone(),
                    student.desc.surname.clone(),
                )
            })
            .collect();

        self.ordered_students
            .sort_by_key(|(id, firstname, surname)| {
                (surname.clone(), firstname.clone(), id.clone())
            });
    }

    fn update_selected_student(&mut self) {
        self.selected_student = self.student_enum_to_selected(self.data.student);
    }
}
