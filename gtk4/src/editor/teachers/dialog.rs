use adw::prelude::{EditableExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    teacher_data: collomatique_state_colloscopes::teachers::Teacher,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    subject_entries: FactoryVecDeque<SubjectEntry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::teachers::Teacher,
    ),
    Cancel,
    Accept,

    UpdateFirstname(String),
    UpdateSurname(String),
    UpdateTelephone(String),
    UpdateEmail(String),
    UpdateSubjectStatus(usize, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::teachers::Teacher),
}

impl Dialog {
    fn generate_email_text(&self) -> String {
        match &self.teacher_data.desc.email {
            None => String::new(),
            Some(text) => text.clone().into_inner(),
        }
    }

    fn generate_telephone_text(&self) -> String {
        match &self.teacher_data.desc.tel {
            None => String::new(),
            Some(text) => text.clone().into_inner(),
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
            set_title: Some("Configuration du colleur"),
            set_size_request: (500, 600),
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
                            set_title: "Nom du colleur",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[name(firstname_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Prénom",
                                #[track(model.should_redraw)]
                                set_text: &model.teacher_data.desc.firstname,
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateFirstname(text));
                                },
                            },
                            #[name(surname_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Nom de famille",
                                #[track(model.should_redraw)]
                                set_text: &model.teacher_data.desc.surname,
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateSurname(text));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Contact",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[name(tel_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Téléphone",
                                #[track(model.should_redraw)]
                                set_text: &model.generate_telephone_text(),
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateTelephone(text));
                                },
                            },
                            #[name(email_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "E-mail",
                                #[track(model.should_redraw)]
                                set_text: &model.generate_email_text(),
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateEmail(text));
                                },
                            },
                        },
                        #[local_ref]
                        subject_entries_widget -> adw::PreferencesGroup {
                            set_title: "Matières concernées",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.subjects.ordered_subject_list.is_empty(),
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
        let teacher_data = collomatique_state_colloscopes::teachers::Teacher::default();
        let subjects = collomatique_state_colloscopes::subjects::Subjects::default();

        let subject_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                SubjectOutput::UpdateStatus(num, status) => {
                    DialogInput::UpdateSubjectStatus(num, status)
                }
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            teacher_data,
            subjects,
            subject_entries,
        };

        let subject_entries_widget = model.subject_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(subjects, teacher_data) => {
                self.hidden = false;
                self.should_redraw = true;
                self.subjects = subjects;
                self.teacher_data = teacher_data;
                crate::tools::factories::update_vec_deque(
                    &mut self.subject_entries,
                    self.subjects
                        .ordered_subject_list
                        .iter()
                        .map(|(id, sub)| SubjectData {
                            name: sub.parameters.name.clone(),
                            enable: self.teacher_data.subjects.contains(id),
                        }),
                    |data| SubjectInput::UpdateData(data),
                );
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.teacher_data.clone()))
                    .unwrap();
            }
            DialogInput::UpdateFirstname(new_firstname) => {
                if self.teacher_data.desc.firstname == new_firstname {
                    return;
                }
                self.teacher_data.desc.firstname = new_firstname;
            }
            DialogInput::UpdateSurname(new_surname) => {
                if self.teacher_data.desc.surname == new_surname {
                    return;
                }
                self.teacher_data.desc.surname = new_surname;
            }
            DialogInput::UpdateTelephone(new_tel) => {
                let tel_opt = non_empty_string::NonEmptyString::new(new_tel).ok();
                if self.teacher_data.desc.tel == tel_opt {
                    return;
                }
                self.teacher_data.desc.tel = tel_opt;
            }
            DialogInput::UpdateEmail(new_email) => {
                let email_opt = non_empty_string::NonEmptyString::new(new_email).ok();
                if self.teacher_data.desc.email == email_opt {
                    return;
                }
                self.teacher_data.desc.email = email_opt;
            }
            DialogInput::UpdateSubjectStatus(subject_num, new_status) => {
                assert!(subject_num < self.subjects.ordered_subject_list.len());
                let subject_id = self.subjects.ordered_subject_list[subject_num].0;

                if new_status {
                    self.teacher_data.subjects.insert(subject_id);
                } else {
                    self.teacher_data.subjects.remove(&subject_id);
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
            widgets.firstname_entry.grab_focus();
        }
    }
}

#[derive(Debug, Clone)]
struct SubjectData {
    name: String,
    enable: bool,
}

#[derive(Debug)]
struct SubjectEntry {
    data: SubjectData,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum SubjectInput {
    UpdateData(SubjectData),

    UpdateStatus(bool),
}

#[derive(Debug)]
enum SubjectOutput {
    UpdateStatus(usize, bool),
}

#[relm4::factory]
impl FactoryComponent for SubjectEntry {
    type Init = SubjectData;
    type Input = SubjectInput;
    type Output = SubjectOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::SwitchRow {
            set_hexpand: true,
            #[watch]
            set_title: &self.data.name,
            #[track(self.should_redraw)]
            set_active: self.data.enable,
            connect_active_notify[sender] => move |widget| {
                let status = widget.is_active();
                sender.input(
                    SubjectInput::UpdateStatus(status)
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
            SubjectInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            SubjectInput::UpdateStatus(new_status) => {
                if self.data.enable == new_status {
                    return;
                }
                self.data.enable = new_status;
                sender
                    .output(SubjectOutput::UpdateStatus(
                        self.index.current_index(),
                        new_status,
                    ))
                    .unwrap();
            }
        }
    }
}
