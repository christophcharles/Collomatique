use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_ops::SettingsUpdateOp;

#[derive(Debug)]
pub enum SettingsInput {
    Update(
        collomatique_state_colloscopes::students::Students<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::settings::Settings<
            collomatique_state_colloscopes::StudentId,
        >,
    ),

    EditGlobalLimits,
    EditStudentLimits(collomatique_state_colloscopes::StudentId),
    DeleteStudentLimits(collomatique_state_colloscopes::StudentId),
}

pub struct Settings {
    students: collomatique_state_colloscopes::students::Students<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    >,
    settings: collomatique_state_colloscopes::settings::Settings<
        collomatique_state_colloscopes::StudentId,
    >,

    student_entries: FactoryVecDeque<StudentEntry>,
}

#[relm4::component(pub)]
impl Component for Settings {
    type Input = SettingsInput;
    type Output = SettingsUpdateOp;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_margin_all: 5,
            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 10,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "Paramètres globaux",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                },
                gtk::ListBox {
                    set_hexpand: true,
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    append = &gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_all: 5,
                        set_spacing: 5,
                        gtk::Button {
                            set_icon_name: "edit-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Modifier les paramètres globaux"),
                            connect_clicked => SettingsInput::EditGlobalLimits,
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_label: "Paramètres globaux",
                            set_size_request: (200, -1),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            #[watch]
                            set_label: &limits_to_string(&model.settings.global),
                            set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
                        },
                    },
                },
                gtk::Label {
                    set_margin_top: 30,
                    set_halign: gtk::Align::Start,
                    set_label: "Paramètres par élève",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                },
                gtk::Label {
                    set_margin_top: 10,
                    #[watch]
                    set_visible: model.students.student_map.is_empty(),
                    set_halign: gtk::Align::Start,
                    set_label: "<i>Aucun élève à afficher</i>",
                    set_use_markup: true,
                },
                #[local_ref]
                students_widget -> gtk::ListBox {
                    set_hexpand: true,
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    #[watch]
                    set_visible: !model.students.student_map.is_empty(),
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
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                StudentEntryOutput::EditClicked(id) => SettingsInput::EditStudentLimits(id),
                StudentEntryOutput::DeleteClicked(id) => SettingsInput::DeleteStudentLimits(id),
            });

        let model = Settings {
            students: collomatique_state_colloscopes::students::Students::default(),
            settings: collomatique_state_colloscopes::settings::Settings::default(),
            student_entries,
        };
        let students_widget = model.student_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SettingsInput::Update(students, settings) => {
                self.students = students;
                self.settings = settings;
                self.update_student_entries();
            }
            SettingsInput::EditGlobalLimits => {}
            SettingsInput::EditStudentLimits(student_id) => {}
            SettingsInput::DeleteStudentLimits(student_id) => {}
        }
    }
}

impl Settings {
    fn update_student_entries(&mut self) {
        let mut students: Vec<_> = self
            .students
            .student_map
            .iter()
            .map(|(id, student)| {
                (
                    id.clone(),
                    student.desc.firstname.clone(),
                    student.desc.surname.clone(),
                )
            })
            .collect();

        students.sort_by_key(|(id, firstname, surname)| {
            (surname.clone(), firstname.clone(), id.clone())
        });

        crate::tools::factories::update_vec_deque(
            &mut self.student_entries,
            students
                .into_iter()
                .map(|(student_id, firstname, surname)| StudentEntryData {
                    student_id,
                    student_name: format!("{} {}", firstname, surname),
                    limits: self.settings.students.get(&student_id).cloned(),
                }),
            |data| StudentEntryInput::UpdateData(data),
        );
    }
}

fn soft_less_than(soft: bool) -> String {
    if soft {
        String::from("⪅")
    } else {
        String::from("⩽")
    }
}

fn limits_to_string(limits: &collomatique_state_colloscopes::settings::Limits) -> String {
    let mut parts = vec![];

    if limits.interrogations_per_week_max.is_some() || limits.interrogations_per_week_min.is_some()
    {
        let mut text = String::new();

        if let Some(min_per_week) = &limits.interrogations_per_week_min {
            text += &format!(
                "{} {} ",
                min_per_week.value,
                soft_less_than(min_per_week.soft),
            );
        }

        text += "colles par semaine";

        if let Some(max_per_week) = &limits.interrogations_per_week_max {
            text += &format!(
                " {} {}",
                soft_less_than(max_per_week.soft),
                max_per_week.value,
            );
        }

        parts.push(text);
    }

    if let Some(max_per_day) = &limits.max_interrogations_per_day {
        parts.push(format!(
            "colles par jour {} {}",
            soft_less_than(max_per_day.soft),
            max_per_day.value,
        ));
    }

    if parts.is_empty() {
        String::from("aucune contrainte")
    } else {
        parts.join("    ―    ")
    }
}

#[derive(Debug)]
pub struct StudentEntryData {
    student_id: collomatique_state_colloscopes::StudentId,
    student_name: String,
    limits: Option<collomatique_state_colloscopes::settings::Limits>,
}

pub struct StudentEntry {
    data: StudentEntryData,
}

#[derive(Debug)]
pub enum StudentEntryInput {
    UpdateData(StudentEntryData),

    EditClicked,
    DeleteClicked,
}

#[derive(Debug)]
pub enum StudentEntryOutput {
    EditClicked(collomatique_state_colloscopes::StudentId),
    DeleteClicked(collomatique_state_colloscopes::StudentId),
}

impl StudentEntry {
    fn generate_edit_tooltip_text(&self) -> String {
        format!("Modifier les paramètres de {}", self.data.student_name,)
    }

    fn generate_delete_tooltip_text(&self) -> String {
        format!(
            "Supprimer les paramètres spécifiques à {}",
            self.data.student_name,
        )
    }

    fn generate_limits_text(&self) -> String {
        match &self.data.limits {
            Some(limits) => limits_to_string(limits),
            None => String::new(),
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for StudentEntry {
    type Init = StudentEntryData;
    type Input = StudentEntryInput;
    type Output = StudentEntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 5,
            gtk::Button {
                set_icon_name: "edit-symbolic",
                add_css_class: "flat",
                connect_clicked => StudentEntryInput::EditClicked,
                #[watch]
                set_tooltip_text: Some(&self.generate_edit_tooltip_text()),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_xalign: 0.,
                set_margin_start: 5,
                set_margin_end: 5,
                #[watch]
                set_label: &self.data.student_name,
                set_size_request: (200, -1),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_xalign: 0.,
                set_margin_start: 5,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_limits_text(),
                set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
            },
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
                #[watch]
                set_visible: self.data.limits.is_some(),
            },
            gtk::Button {
                set_icon_name: "edit-delete",
                add_css_class: "flat",
                connect_clicked => StudentEntryInput::DeleteClicked,
                #[watch]
                set_tooltip_text: Some(&self.generate_delete_tooltip_text()),
                #[watch]
                set_visible: self.data.limits.is_some(),
            },
        }
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let model = Self { data };

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
        match msg {
            StudentEntryInput::UpdateData(new_data) => {
                self.data = new_data;
            }
            StudentEntryInput::EditClicked => {
                sender
                    .output(StudentEntryOutput::EditClicked(
                        self.data.student_id.clone(),
                    ))
                    .unwrap();
            }
            StudentEntryInput::DeleteClicked => {
                sender
                    .output(StudentEntryOutput::DeleteClicked(
                        self.data.student_id.clone(),
                    ))
                    .unwrap();
            }
        }
    }
}
