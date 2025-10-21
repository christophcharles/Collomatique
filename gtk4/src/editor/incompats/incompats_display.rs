use std::collections::BTreeMap;

use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::RelmWidgetExt;
use relm4::{adw, gtk};

#[derive(Debug, Clone)]
pub struct EntryData {
    pub subject_params: collomatique_state_colloscopes::SubjectParameters,
    pub subject_id: collomatique_state_colloscopes::SubjectId,
    pub week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns<
        collomatique_state_colloscopes::WeekPatternId,
    >,
    pub subject_incompats: BTreeMap<
        collomatique_state_colloscopes::IncompatId,
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    >,
}

#[derive(Debug)]
pub struct Entry {
    subject_params: collomatique_state_colloscopes::SubjectParameters,
    subject_id: collomatique_state_colloscopes::SubjectId,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns<
        collomatique_state_colloscopes::WeekPatternId,
    >,
    subject_incompats: BTreeMap<
        collomatique_state_colloscopes::IncompatId,
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    >,
    incompats: FactoryVecDeque<Incompat>,
}

#[derive(Debug, Clone)]
pub enum EntryInput {
    UpdateData(EntryData),

    AddIncompatClicked,
}

#[derive(Debug)]
pub enum EntryOutput {
    DeleteIncompat(collomatique_state_colloscopes::IncompatId),
    AddIncompat(collomatique_state_colloscopes::SubjectId),
    EditIncompat(collomatique_state_colloscopes::IncompatId),
}

impl Entry {
    fn incompat_data_from_slot(
        &self,
        incompat_id: collomatique_state_colloscopes::IncompatId,
        incompat: &collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ) -> IncompatData {
        let week_pattern = if let Some(id) = incompat.week_pattern_id {
            Some(
                self.week_patterns
                    .week_pattern_map
                    .get(&id)
                    .expect("Week pattern ID should be valid"),
            )
        } else {
            None
        };
        let week_pattern_name = match week_pattern {
            Some(pattern) => pattern.name.clone(),
            None => "Toutes les semaines".into(),
        };
        IncompatData {
            incompat_id,
            incompat_name: incompat.name.clone(),
            week_pattern_name,
            slots: incompat.slots.clone(),
            minimum_free_slots: incompat.minimum_free_slots,
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = EntryInput;
    type Output = EntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: &self.subject_params.name,
                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: "Aucune incompatibilité à afficher",
                #[watch]
                set_visible: self.incompats.is_empty(),
            },
            #[local_ref]
            incompats_list -> gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
                #[watch]
                set_visible: !self.incompats.is_empty(),
            },
            gtk::Button {
                set_margin_top: 10,
                adw::ButtonContent {
                    set_icon_name: "edit-add",
                    set_label: "Ajouter une incompatibilité",
                },
                connect_clicked => EntryInput::AddIncompatClicked,
            }
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let incompats = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.output_sender(), |msg| match msg {
                IncompatOutput::DeleteIncompat(slot_id) => EntryOutput::DeleteIncompat(slot_id),
                IncompatOutput::EditIncompat(slot_id) => EntryOutput::EditIncompat(slot_id),
            });

        let mut model = Self {
            subject_params: data.subject_params,
            subject_id: data.subject_id,
            week_patterns: data.week_patterns,
            subject_incompats: data.subject_incompats,
            incompats,
        };

        let incompats_vec: Vec<_> = model
            .subject_incompats
            .iter()
            .map(|(incompat_id, incompat)| model.incompat_data_from_slot(*incompat_id, incompat))
            .collect();
        crate::tools::factories::update_vec_deque(
            &mut model.incompats,
            incompats_vec.into_iter(),
            |x| IncompatInput::UpdateData(x),
        );

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let incompats_list = self.incompats.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.subject_params = new_data.subject_params;
                self.subject_id = new_data.subject_id;
                self.subject_incompats = new_data.subject_incompats;
                self.week_patterns = new_data.week_patterns;

                let incompats_vec: Vec<_> = self
                    .subject_incompats
                    .iter()
                    .map(|(incompat_id, incompat)| {
                        self.incompat_data_from_slot(*incompat_id, incompat)
                    })
                    .collect();
                crate::tools::factories::update_vec_deque(
                    &mut self.incompats,
                    incompats_vec.into_iter(),
                    |x| IncompatInput::UpdateData(x),
                );
            }
            EntryInput::AddIncompatClicked => {
                sender
                    .output(EntryOutput::AddIncompat(self.subject_id))
                    .unwrap();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct IncompatData {
    pub incompat_id: collomatique_state_colloscopes::IncompatId,
    pub incompat_name: String,
    pub week_pattern_name: String,
    pub slots: Vec<collomatique_time::SlotWithDuration>,
    pub minimum_free_slots: std::num::NonZeroU32,
}

#[derive(Debug)]
pub struct Incompat {
    data: IncompatData,
}

#[derive(Debug, Clone)]
pub enum IncompatInput {
    UpdateData(IncompatData),

    DeleteClicked,
    EditIncompatClicked,
}

#[derive(Debug)]
pub enum IncompatOutput {
    DeleteIncompat(collomatique_state_colloscopes::IncompatId),
    EditIncompat(collomatique_state_colloscopes::IncompatId),
}

impl Incompat {
    fn generate_extra(&self) -> String {
        if self.data.minimum_free_slots.get() == 1 {
            "1 créneau libre".into()
        } else {
            format!("{} créneaux libres", self.data.minimum_free_slots)
        }
    }

    fn generate_slots(&self) -> String {
        let slots: Vec<_> = self
            .data
            .slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                if i == 0 {
                    slot.capitalize()
                } else {
                    slot.to_string()
                }
            })
            .collect();
        slots.join(", ")
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Incompat {
    type Init = IncompatData;
    type Input = IncompatInput;
    type Output = IncompatOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        root_widget = gtk::Box {
            set_hexpand: true,
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 5,
            gtk::Button {
                set_icon_name: "edit-symbolic",
                add_css_class: "flat",
                connect_clicked => IncompatInput::EditIncompatClicked,
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
                set_label: &self.data.incompat_name,
                set_size_request: (130, -1),
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
                set_label: &self.data.week_pattern_name,
                set_size_request: (150, -1),
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
                set_label: &self.generate_slots(),
            },
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Label {
                set_halign: gtk::Align::End,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_extra(),
                set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Button {
                set_icon_name: "edit-delete",
                add_css_class: "flat",
                connect_clicked => IncompatInput::DeleteClicked,
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
            IncompatInput::UpdateData(new_data) => {
                self.data = new_data;
            }
            IncompatInput::DeleteClicked => {
                sender
                    .output(IncompatOutput::DeleteIncompat(self.data.incompat_id))
                    .unwrap();
            }
            IncompatInput::EditIncompatClicked => {
                sender
                    .output(IncompatOutput::EditIncompat(self.data.incompat_id))
                    .unwrap();
            }
        }
    }
}
