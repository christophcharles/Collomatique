use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

mod options_dialog;

use collomatique_ops::BalancingUpdateOp;
use collomatique_state_colloscopes::balancing::BalancingOptions;
use collomatique_state_colloscopes::ids::SubjectId;

#[derive(Debug)]
pub enum BalancingInput {
    Update(
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::balancing::Balancing,
    ),

    EditGlobalOptions,
    EditSubjectOptions(SubjectId),
    DeleteSubjectOptions(SubjectId),
    OptionsAccepted(BalancingOptions),
    /// A dialog of this panel just closed. The panel hosts no window of its
    /// own, so it passes the request up to the editor.
    PresentParent,
}

#[derive(Debug)]
pub enum BalancingOutput {
    UpdateOp(BalancingUpdateOp),
    /// A dialog of this panel just closed: the window underneath should be
    /// brought back to the front, because Windows will not do it on its own.
    PresentParent,
}

pub struct Balancing {
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    balancing: collomatique_state_colloscopes::balancing::Balancing,

    subject_entries: FactoryVecDeque<SubjectEntry>,
    edit_reason: Option<SubjectId>,
    dialog: Controller<options_dialog::Dialog>,
}

#[relm4::component(pub)]
impl Component for Balancing {
    type Input = BalancingInput;
    type Output = BalancingOutput;
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
                            set_icon_name: "document-edit-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Modifier les paramètres globaux d'équilibrage"),
                            connect_clicked => BalancingInput::EditGlobalOptions,
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_width_chars: 20,
                            set_max_width_chars: 20,
                            set_label: "Paramètres globaux",
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
                            set_label: &options_to_string(&model.balancing.global),
                            set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
                        },
                    },
                },
                gtk::Label {
                    set_margin_top: 30,
                    set_halign: gtk::Align::Start,
                    set_label: "Paramètres par matière",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                },
                gtk::Label {
                    set_margin_top: 10,
                    #[watch]
                    set_visible: !model.has_subjects_with_interrogations(),
                    set_halign: gtk::Align::Start,
                    set_label: "<i>Aucune matière avec des colles à afficher</i>",
                    set_use_markup: true,
                },
                #[local_ref]
                subjects_widget -> gtk::ListBox {
                    set_hexpand: true,
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    #[watch]
                    set_visible: model.has_subjects_with_interrogations(),
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let subject_entries = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                SubjectEntryOutput::EditClicked(id) => BalancingInput::EditSubjectOptions(id),
                SubjectEntryOutput::DeleteClicked(id) => BalancingInput::DeleteSubjectOptions(id),
            });

        let dialog = options_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                options_dialog::DialogOutput::Accepted(options) => {
                    BalancingInput::OptionsAccepted(options)
                }
                options_dialog::DialogOutput::PresentParent => BalancingInput::PresentParent,
            });

        let model = Balancing {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            balancing: collomatique_state_colloscopes::balancing::Balancing::default(),
            subject_entries,
            edit_reason: None,
            dialog,
        };
        let subjects_widget = model.subject_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            BalancingInput::Update(subjects, balancing) => {
                self.subjects = subjects;
                self.balancing = balancing;
                self.update_subject_entries();
            }
            BalancingInput::EditGlobalOptions => {
                self.edit_reason = None;
                self.dialog
                    .sender()
                    .send(options_dialog::DialogInput::Show(
                        self.balancing.global.clone(),
                        None,
                    ))
                    .unwrap();
            }
            BalancingInput::EditSubjectOptions(subject_id) => {
                self.edit_reason = Some(subject_id);
                let subject = self
                    .subjects
                    .ordered_subject_list
                    .iter()
                    .find(|(id, _)| *id == subject_id)
                    .expect("Subject ID should be valid");
                self.dialog
                    .sender()
                    .send(options_dialog::DialogInput::Show(
                        self.balancing
                            .subjects
                            .get(&subject_id)
                            .cloned()
                            .unwrap_or(self.balancing.global.clone()),
                        Some(subject.1.parameters.name.clone()),
                    ))
                    .unwrap();
            }
            BalancingInput::DeleteSubjectOptions(subject_id) => {
                sender
                    .output(BalancingOutput::UpdateOp(
                        BalancingUpdateOp::RemoveSubjectOptions(subject_id),
                    ))
                    .unwrap();
            }
            BalancingInput::OptionsAccepted(options) => match self.edit_reason.take() {
                Some(subject_id) => {
                    sender
                        .output(BalancingOutput::UpdateOp(
                            BalancingUpdateOp::UpdateSubjectOptions(subject_id, options),
                        ))
                        .unwrap();
                }
                None => {
                    sender
                        .output(BalancingOutput::UpdateOp(
                            BalancingUpdateOp::UpdateGlobalOptions(options),
                        ))
                        .unwrap();
                }
            },
            BalancingInput::PresentParent => {
                sender.output(BalancingOutput::PresentParent).unwrap();
            }
        }
    }
}

impl Balancing {
    fn has_subjects_with_interrogations(&self) -> bool {
        self.subjects
            .ordered_subject_list
            .iter()
            .any(|(_, subject)| subject.parameters.interrogation_parameters.is_some())
    }

    fn update_subject_entries(&mut self) {
        let mut subjects: Vec<_> = self
            .subjects
            .ordered_subject_list
            .iter()
            .filter(|(_, subject)| subject.parameters.interrogation_parameters.is_some())
            .map(|(id, subject)| (id, subject.parameters.name.clone()))
            .collect();

        subjects.sort_by_key(|(id, name)| (name.clone(), *id));

        crate::tools::factories::update_vec_deque(
            &mut self.subject_entries,
            subjects
                .into_iter()
                .map(|(subject_id, subject_name)| SubjectEntryData {
                    subject_id,
                    subject_name,
                    options: self.balancing.subjects.get(&subject_id).cloned(),
                }),
            SubjectEntryInput::UpdateData,
        );
    }
}

// The parenthetical qualifies the constraint itself, so it stays feminine
// whatever the goal it is appended to.
fn soft_constraint_symbol(soft: bool) -> &'static str {
    if soft { "(souple)" } else { "(stricte)" }
}

fn options_to_string(options: &BalancingOptions) -> String {
    // Only the goals that are actually pursued get an entry, soft ones
    // included. A goal that is off is not a constraint, so it says nothing.
    let mut parts = vec![];

    if let Some(param) = &options.teacher_rotation {
        parts.push(format!(
            "rotation des colleurs {}",
            soft_constraint_symbol(param.soft)
        ));
    }
    if let Some(param) = &options.slot_rotation {
        parts.push(format!(
            "rotation des créneaux {}",
            soft_constraint_symbol(param.soft)
        ));
    }
    if let Some(param) = &options.avoid_twice_in_a_row {
        parts.push(format!(
            "éviter 2× de suite le même colleur {}",
            soft_constraint_symbol(param.soft)
        ));
    }

    // The year and period rotations are plain strictness booleans with no soft
    // mode, so they carry no symbol.
    if options.year_teacher_rotation {
        parts.push(String::from("rotation annuelle des colleurs"));
    }
    if options.period_teacher_rotation {
        parts.push(String::from("rotation des colleurs par période"));
    }

    if parts.is_empty() {
        String::from("aucune contrainte")
    } else {
        parts.join("    ―    ")
    }
}

#[derive(Debug)]
pub struct SubjectEntryData {
    subject_id: SubjectId,
    subject_name: String,
    options: Option<BalancingOptions>,
}

pub struct SubjectEntry {
    data: SubjectEntryData,
}

#[derive(Debug)]
pub enum SubjectEntryInput {
    UpdateData(SubjectEntryData),

    EditClicked,
    DeleteClicked,
}

#[derive(Debug)]
pub enum SubjectEntryOutput {
    EditClicked(SubjectId),
    DeleteClicked(SubjectId),
}

impl SubjectEntry {
    fn generate_edit_tooltip_text(&self) -> String {
        format!(
            "Modifier les paramètres d'équilibrage strict de {}",
            self.data.subject_name,
        )
    }

    fn generate_delete_tooltip_text(&self) -> String {
        format!(
            "Supprimer les paramètres spécifiques à {}",
            self.data.subject_name,
        )
    }

    fn generate_options_text(&self) -> String {
        match &self.data.options {
            Some(options) => options_to_string(options),
            None => "Paramètres globaux appliqués".into(),
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for SubjectEntry {
    type Init = SubjectEntryData;
    type Input = SubjectEntryInput;
    type Output = SubjectEntryOutput;
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
                set_icon_name: "document-edit-symbolic",
                add_css_class: "flat",
                connect_clicked => SubjectEntryInput::EditClicked,
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
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                set_width_chars: 20,
                set_max_width_chars: 20,
                #[watch]
                set_label: &self.data.subject_name,
                #[watch]
                set_tooltip_text: Some(&self.data.subject_name),
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
                set_label: &self.generate_options_text(),
                set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
            },
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
                #[watch]
                set_visible: self.data.options.is_some(),
            },
            gtk::Button {
                set_icon_name: "edit-delete-symbolic",
                add_css_class: "flat",
                connect_clicked => SubjectEntryInput::DeleteClicked,
                #[watch]
                set_tooltip_text: Some(&self.generate_delete_tooltip_text()),
                #[watch]
                set_visible: self.data.options.is_some(),
            },
        }
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { data }
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
            SubjectEntryInput::UpdateData(new_data) => {
                self.data = new_data;
            }
            SubjectEntryInput::EditClicked => {
                sender
                    .output(SubjectEntryOutput::EditClicked(self.data.subject_id))
                    .unwrap();
            }
            SubjectEntryInput::DeleteClicked => {
                sender
                    .output(SubjectEntryOutput::DeleteClicked(self.data.subject_id))
                    .unwrap();
            }
        }
    }
}
