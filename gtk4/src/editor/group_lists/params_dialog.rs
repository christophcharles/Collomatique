use adw::prelude::{EditableExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

use std::collections::BTreeSet;
use std::num::NonZeroU32;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    students: collomatique_state_colloscopes::students::Students<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    >,

    ordered_students: Vec<(collomatique_state_colloscopes::StudentId, String, String)>,

    selected_name: String,
    selected_students_per_group_minimum: u32,
    selected_students_per_group_maximum: u32,
    selected_group_count_minimum: u32,
    selected_group_count_maximum: u32,
    excluded_students: BTreeSet<collomatique_state_colloscopes::StudentId>,

    student_entries: FactoryVecDeque<StudentEntry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::group_lists::GroupListParameters<
            collomatique_state_colloscopes::StudentId,
        >,
        collomatique_state_colloscopes::students::Students<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::PeriodId,
        >,
    ),
    Cancel,
    Accept,

    UpdateSelectedName(String),
    UpdateStudentsPerGroupMinimum(u32),
    UpdateStudentsPerGroupMaximum(u32),
    UpdateGroupCountMinimum(u32),
    UpdateGroupCountMaximum(u32),

    UpdateStudentStatus(usize, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(
        collomatique_state_colloscopes::group_lists::GroupListParameters<
            collomatique_state_colloscopes::StudentId,
        >,
    ),
}

impl Dialog {}

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
            set_default_size: (500, 500),
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
                #[name(scrolled_window)]
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
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
                            set_title: "Nombre de groupes",
                            set_description: Some("Nombre total de groupes dans la liste"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Minimum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 0.,
                                    #[watch]
                                    set_upper: model.selected_group_count_maximum as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.selected_group_count_minimum as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let groups_count_min_u32 = widget.value() as u32;
                                    sender.input(DialogInput::UpdateGroupCountMinimum(groups_count_min_u32));
                                },
                            },
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Maximum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    #[watch]
                                    set_lower: model.selected_group_count_minimum as f64,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.selected_group_count_maximum as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let groups_count_max_u32 = widget.value() as u32;
                                    sender.input(DialogInput::UpdateGroupCountMaximum(groups_count_max_u32));
                                },
                            },
                        },
                        #[local_ref]
                        student_entries_widget -> adw::PreferencesGroup {
                            set_title: "Élèves dans la liste",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.ordered_students.is_empty(),
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
        let student_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                StudentOutput::UpdateStatus(num, status) => {
                    DialogInput::UpdateStudentStatus(num, status)
                }
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            students: collomatique_state_colloscopes::students::Students::default(),
            ordered_students: vec![],
            selected_name: String::new(),
            selected_students_per_group_minimum: 1,
            selected_group_count_maximum: 3,
            selected_group_count_minimum: 0,
            selected_students_per_group_maximum: u32::MAX,
            student_entries,
            excluded_students: BTreeSet::new(),
        };

        let student_entries_widget = model.student_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(group_list_data, students) => {
                self.hidden = false;
                self.should_redraw = true;
                self.students = students;
                self.update_ordered_students();
                self.update_from_data(group_list_data);

                crate::tools::factories::update_vec_deque(
                    &mut self.student_entries,
                    self.ordered_students
                        .iter()
                        .map(|(id, firstname, surname)| StudentData {
                            name: format!("{} {}", firstname, surname),
                            included: !self.excluded_students.contains(id),
                        }),
                    |data| StudentInput::UpdateData(data),
                );
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
            DialogInput::UpdateGroupCountMinimum(selected_group_count_minimum) => {
                if self.selected_group_count_minimum == selected_group_count_minimum {
                    return;
                }
                self.selected_group_count_minimum = selected_group_count_minimum;
            }
            DialogInput::UpdateGroupCountMaximum(selected_group_count_maximum) => {
                if self.selected_group_count_maximum == selected_group_count_maximum {
                    return;
                }
                self.selected_group_count_maximum = selected_group_count_maximum;
            }
            DialogInput::UpdateStudentStatus(student_num, new_status) => {
                assert!(student_num < self.ordered_students.len());
                let student_id = self.ordered_students[student_num].0;

                if new_status {
                    self.excluded_students.remove(&student_id);
                } else {
                    self.excluded_students.insert(student_id);
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
            widgets.name_entry.grab_focus();
        }
    }
}

impl Dialog {
    fn update_ordered_students(&mut self) {
        self.ordered_students = self
            .students
            .student_map
            .iter()
            .map(|(student_id, student)| {
                (
                    student_id.clone(),
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

    fn update_from_data(
        &mut self,
        data: collomatique_state_colloscopes::group_lists::GroupListParameters<
            collomatique_state_colloscopes::StudentId,
        >,
    ) {
        self.selected_name = data.name;
        self.selected_students_per_group_minimum = data.students_per_group.start().get();
        self.selected_students_per_group_maximum = data.students_per_group.end().get();
        self.selected_group_count_minimum = *data.group_count.start();
        self.selected_group_count_maximum = *data.group_count.end();
        self.excluded_students = data.excluded_students;
    }

    fn generate_data(
        &self,
    ) -> collomatique_state_colloscopes::group_lists::GroupListParameters<
        collomatique_state_colloscopes::StudentId,
    > {
        collomatique_state_colloscopes::group_lists::GroupListParameters {
            name: self.selected_name.clone(),
            students_per_group: NonZeroU32::new(self.selected_students_per_group_minimum).unwrap()
                ..=NonZeroU32::new(self.selected_students_per_group_maximum).unwrap(),
            group_count: self.selected_group_count_minimum..=self.selected_group_count_maximum,
            excluded_students: self.excluded_students.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct StudentData {
    name: String,
    included: bool,
}

#[derive(Debug)]
struct StudentEntry {
    data: StudentData,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum StudentInput {
    UpdateData(StudentData),

    UpdateStatus(bool),
}

#[derive(Debug)]
enum StudentOutput {
    UpdateStatus(usize, bool),
}

#[relm4::factory]
impl FactoryComponent for StudentEntry {
    type Init = StudentData;
    type Input = StudentInput;
    type Output = StudentOutput;
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
                sender.input(
                    StudentInput::UpdateStatus(status)
                );
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
            StudentInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            StudentInput::UpdateStatus(new_status) => {
                if self.data.included == new_status {
                    return;
                }
                self.data.included = new_status;
                sender
                    .output(StudentOutput::UpdateStatus(
                        self.index.current_index(),
                        new_status,
                    ))
                    .unwrap();
            }
        }
    }
}
