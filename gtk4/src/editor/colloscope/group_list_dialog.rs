use adw::prelude::{ComboRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{
    AdjustmentExt, BoxExt, ButtonExt, GridExt, GtkWindowExt, OrientableExt, WidgetExt,
};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    students: collomatique_state_colloscopes::students::Students,
    group_list: collomatique_state_colloscopes::group_lists::GroupList,
    collo_group_list: collomatique_state_colloscopes::colloscopes::ColloscopeGroupList,
    student_entries: FactoryVecDeque<StudentEntry>,
    list_model: gtk::StringList,
    students_to_display: Vec<(collomatique_state_colloscopes::StudentId, String, String)>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::students::Students,
        collomatique_state_colloscopes::group_lists::GroupList,
        collomatique_state_colloscopes::colloscopes::ColloscopeGroupList,
    ),
    Cancel,
    Accept,

    UsePrefillClicked,
    UpdateStudentGroup(collomatique_state_colloscopes::StudentId, u32),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::colloscopes::ColloscopeGroupList),
}

impl Dialog {
    fn selected_to_group_opt(selected: u32) -> Option<u32> {
        if selected == 0 {
            return None;
        }
        Some(selected - 1)
    }

    fn group_opt_to_selected(group_opt: Option<u32>) -> u32 {
        match group_opt {
            None => 0,
            Some(group) => group + 1,
        }
    }

    fn generate_list_name(&self) -> String {
        format!("Liste concernée : {}", self.group_list.params.name)
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
            set_title: Some("Édition de la liste de groupes"),
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
                            set_margin_all: 5,
                            set_spacing: 10,
                            set_orientation: gtk::Orientation::Vertical,
                            gtk::Label {
                                set_label: "<b>Options de préremplissage</b>",
                                set_use_markup: true,
                                set_halign: gtk::Align::Start,
                            },
                            #[name(btn_grid)]
                            gtk::Grid {
                                set_hexpand: true,
                                set_column_homogeneous: true,
                                set_row_homogeneous: true,
                                set_column_spacing: 5,
                                set_row_spacing: 5,
                            },
                            #[local_ref]
                            student_entries_widget -> adw::PreferencesGroup {
                                set_title: "Affectation des élèves dans les groupes",
                                set_margin_all: 5,
                                set_hexpand: true,
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
        },
        use_prefill_btn = gtk::Button {
            set_label: "Préremplir",
            set_hexpand: true,
            connect_clicked => DialogInput::UsePrefillClicked,
        },
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let student_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                StudentOutput::UpdateStudentGroup(student_id, new_selected) => {
                    DialogInput::UpdateStudentGroup(student_id, new_selected)
                }
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            students: collomatique_state_colloscopes::students::Students::default(),
            group_list: collomatique_state_colloscopes::group_lists::GroupList::default(),
            collo_group_list:
                collomatique_state_colloscopes::colloscopes::ColloscopeGroupList::default(),
            student_entries,
            list_model: gtk::StringList::default(),
            students_to_display: vec![],
        };

        let student_entries_widget = model.student_entries.widget();
        let widgets = view_output!();

        widgets
            .btn_grid
            .attach(&widgets.use_prefill_btn, 0, 0, 1, 1);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(students, group_list, collo_group_list) => {
                self.hidden = false;
                self.should_redraw = true;
                self.students = students;
                self.group_list = group_list;
                self.collo_group_list = collo_group_list;

                self.update_students_to_display();
                self.update_list_model();

                self.update_factory();
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.collo_group_list.clone()))
                    .unwrap();
            }
            DialogInput::UsePrefillClicked => {
                self.fill_with_prefill();
                self.update_factory();
            }
            DialogInput::UpdateStudentGroup(student_id, selected) => {
                match Self::selected_to_group_opt(selected) {
                    Some(group) => {
                        self.collo_group_list
                            .groups_for_students
                            .insert(student_id, group);
                    }
                    None => {
                        self.collo_group_list
                            .groups_for_students
                            .remove(&student_id);
                    }
                }
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
    fn fill_with_prefill(&mut self) {
        if let Some(prefilled) = &self.group_list.prefilled_groups {
            for (group_num, prefilled_group) in prefilled.groups.iter().enumerate() {
                for student_id in &prefilled_group.students {
                    self.collo_group_list
                        .groups_for_students
                        .insert(*student_id, group_num as u32);
                }
            }
        }
    }

    fn update_students_to_display(&mut self) {
        self.students_to_display = self
            .students
            .student_map
            .iter()
            .filter_map(|(id, student)| {
                if self.group_list.params.excluded_students.contains(id) {
                    return None;
                }
                Some((
                    *id,
                    student.desc.firstname.clone(),
                    student.desc.surname.clone(),
                ))
            })
            .collect();

        self.students_to_display
            .sort_by_key(|(id, firstname, surname)| (surname.clone(), firstname.clone(), *id));
    }

    fn update_list_model(&mut self) {
        let group_names_list: Vec<_> = ["(Aucun groupe)".into()]
            .into_iter()
            .chain(self.group_list.params.group_names.iter().enumerate().map(
                |(num, group_name)| match group_name {
                    Some(name) => {
                        format!("Groupe {} : {}", num + 1, name)
                    }
                    None => {
                        format!("Groupe {}", num + 1)
                    }
                },
            ))
            .collect();
        let group_names_list_ref: Vec<_> = group_names_list.iter().map(|x| x.as_str()).collect();
        self.list_model = gtk::StringList::new(&group_names_list_ref[..]);
    }

    fn update_factory(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.student_entries,
            self.students_to_display
                .iter()
                .map(|(id, firstname, surname)| {
                    let group_opt = self.collo_group_list.groups_for_students.get(id).copied();

                    StudentData {
                        list_model: self.list_model.clone(),
                        student_id: *id,
                        student_name: format!("{} {}", firstname, surname),
                        selected_group: Self::group_opt_to_selected(group_opt),
                    }
                }),
            |data| StudentInput::UpdateData(data),
        );
    }
}

#[derive(Debug, Clone)]
struct StudentData {
    list_model: gtk::StringList,
    student_id: collomatique_state_colloscopes::StudentId,
    student_name: String,
    selected_group: u32,
}

#[derive(Debug)]
struct StudentEntry {
    data: StudentData,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum StudentInput {
    UpdateData(StudentData),

    StudentGroupChanged(u32),
}

#[derive(Debug, Clone)]
enum StudentOutput {
    UpdateStudentGroup(collomatique_state_colloscopes::StudentId, u32),
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
        root_widget = adw::ComboRow {
            #[watch]
            set_title: &self.data.student_name,
            #[track(self.should_redraw)]
            set_model: Some(&self.data.list_model),
            #[track(self.should_redraw)]
            set_selected: self.data.selected_group,
            connect_selected_notify[sender] => move |widget| {
                let selected = widget.selected() as u32;
                sender.input(StudentInput::StudentGroupChanged(selected));
            },
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
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
            StudentInput::StudentGroupChanged(selected) => {
                if self.data.selected_group == selected {
                    return;
                }
                self.data.selected_group = selected;
                sender
                    .output(StudentOutput::UpdateStudentGroup(
                        self.data.student_id,
                        selected,
                    ))
                    .unwrap();
            }
        }
    }
}
