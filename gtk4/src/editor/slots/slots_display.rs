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
    pub teachers: BTreeMap<
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::SubjectId,
        >,
    >,
    pub week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns<
        collomatique_state_colloscopes::WeekPatternId,
    >,
    pub subject_slots: collomatique_state_colloscopes::slots::SubjectSlots<
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::WeekPatternId,
    >,
}

#[derive(Debug)]
pub struct Entry {
    subject_params: collomatique_state_colloscopes::SubjectParameters,
    subject_id: collomatique_state_colloscopes::SubjectId,
    teachers: BTreeMap<
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::SubjectId,
        >,
    >,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns<
        collomatique_state_colloscopes::WeekPatternId,
    >,
    subject_slots: collomatique_state_colloscopes::slots::SubjectSlots<
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::WeekPatternId,
    >,
    slots: FactoryVecDeque<Slot>,
}

#[derive(Debug, Clone)]
pub enum EntryInput {
    UpdateData(EntryData),

    AddSlotClicked,
}

#[derive(Debug)]
pub enum EntryOutput {
    MoveSlotUp(collomatique_state_colloscopes::SlotId),
    MoveSlotDown(collomatique_state_colloscopes::SlotId),
    DeleteSlot(collomatique_state_colloscopes::SlotId),
    AddSlot(collomatique_state_colloscopes::SubjectId),
    EditSlot(collomatique_state_colloscopes::SlotId),
}

impl Entry {
    fn slot_data_from_slot(
        &self,
        slot_id: collomatique_state_colloscopes::SlotId,
        slot: &collomatique_state_colloscopes::slots::Slot<
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ) -> SlotData {
        let teacher = self
            .teachers
            .get(&slot.teacher_id)
            .expect("Teacher Id should be valid")
            .clone();
        let week_pattern = if let Some(id) = slot.week_pattern {
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
        let slot_start = slot.start_time.clone();
        let slot_count = self.subject_slots.ordered_slots.len();
        SlotData {
            slot_id,
            teacher,
            slot_start,
            slot_count,
            week_pattern_name,
            extra_info: slot.extra_info.clone(),
            cost: slot.cost,
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
                set_label: "<i>Aucun créneau à afficher</i>",
                set_use_markup: true,
                #[watch]
                set_visible: self.slots.is_empty(),
            },
            #[local_ref]
            slots_list -> gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
                #[watch]
                set_visible: !self.slots.is_empty(),
            },
            gtk::Button {
                set_margin_top: 10,
                adw::ButtonContent {
                    set_icon_name: "edit-add",
                    set_label: "Ajouter un créneau",
                },
                #[watch]
                set_sensitive: !self.teachers.is_empty(),
                connect_clicked => EntryInput::AddSlotClicked,
            }
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let slots = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.output_sender(), |msg| match msg {
                SlotOutput::MoveSlotUp(slot_id) => EntryOutput::MoveSlotUp(slot_id),
                SlotOutput::MoveSlotDown(slot_id) => EntryOutput::MoveSlotDown(slot_id),
                SlotOutput::DeleteSlot(slot_id) => EntryOutput::DeleteSlot(slot_id),
                SlotOutput::EditSlot(slot_id) => EntryOutput::EditSlot(slot_id),
            });

        let mut model = Self {
            subject_params: data.subject_params,
            subject_id: data.subject_id,
            teachers: data.teachers,
            week_patterns: data.week_patterns,
            subject_slots: data.subject_slots,
            slots,
        };

        let slots_vec: Vec<_> = model
            .subject_slots
            .ordered_slots
            .iter()
            .map(|(slot_id, slot)| model.slot_data_from_slot(*slot_id, slot))
            .collect();
        crate::tools::factories::update_vec_deque(&mut model.slots, slots_vec.into_iter(), |x| {
            SlotInput::UpdateData(x)
        });

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let slots_list = self.slots.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.subject_params = new_data.subject_params;
                self.subject_id = new_data.subject_id;
                self.teachers = new_data.teachers;
                self.week_patterns = new_data.week_patterns;
                self.subject_slots = new_data.subject_slots;

                let slots_vec: Vec<_> = self
                    .subject_slots
                    .ordered_slots
                    .iter()
                    .map(|(slot_id, slot)| self.slot_data_from_slot(*slot_id, slot))
                    .collect();
                crate::tools::factories::update_vec_deque(
                    &mut self.slots,
                    slots_vec.into_iter(),
                    |x| SlotInput::UpdateData(x),
                );
            }
            EntryInput::AddSlotClicked => {
                sender
                    .output(EntryOutput::AddSlot(self.subject_id))
                    .unwrap();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SlotData {
    pub slot_id: collomatique_state_colloscopes::SlotId,
    pub teacher: collomatique_state_colloscopes::teachers::Teacher<
        collomatique_state_colloscopes::SubjectId,
    >,
    pub slot_start: collomatique_time::SlotStart,
    pub week_pattern_name: String,
    pub slot_count: usize,
    pub extra_info: String,
    pub cost: i32,
}

#[derive(Debug)]
pub struct Slot {
    index: DynamicIndex,
    data: SlotData,
}

#[derive(Debug, Clone)]
pub enum SlotInput {
    UpdateData(SlotData),

    MoveUpClicked,
    MoveDownClicked,
    DeleteClicked,
    EditSlotClicked,
}

#[derive(Debug)]
pub enum SlotOutput {
    MoveSlotUp(collomatique_state_colloscopes::SlotId),
    MoveSlotDown(collomatique_state_colloscopes::SlotId),
    DeleteSlot(collomatique_state_colloscopes::SlotId),
    EditSlot(collomatique_state_colloscopes::SlotId),
}

impl Slot {
    fn generate_teacher_name(&self) -> String {
        format!(
            "{} {}",
            self.data.teacher.desc.firstname, self.data.teacher.desc.surname,
        )
    }

    fn generate_slot_start_text(&self) -> String {
        self.data.slot_start.capitalize()
    }

    fn generate_extra(&self) -> String {
        let mut extra_texts = Vec::new();
        if !self.data.extra_info.is_empty() {
            extra_texts.push(self.data.extra_info.clone());
        }
        if self.data.cost != 0 {
            extra_texts.push(format!("coût : {}", self.data.cost));
        }
        extra_texts.join(", ")
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Slot {
    type Init = SlotData;
    type Input = SlotInput;
    type Output = SlotOutput;
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
                connect_clicked => SlotInput::EditSlotClicked,
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
                set_label: &self.generate_teacher_name(),
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
                set_label: &self.generate_slot_start_text(),
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
                set_size_request: (200, -1),
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
                set_icon_name: "go-up",
                add_css_class: "flat",
                #[watch]
                set_sensitive: self.index.current_index() != 0,
                set_tooltip_text: Some("Remonter dans la liste"),
                connect_clicked => SlotInput::MoveUpClicked,
            },
            gtk::Button {
                set_icon_name: "go-down",
                add_css_class: "flat",
                #[watch]
                set_sensitive: self.index.current_index() < self.data.slot_count-1,
                set_tooltip_text: Some("Descendre dans la liste"),
                connect_clicked => SlotInput::MoveDownClicked,
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Button {
                set_icon_name: "edit-delete",
                add_css_class: "flat",
                connect_clicked => SlotInput::DeleteClicked,
            },
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            index: index.clone(),
            data,
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
        match msg {
            SlotInput::UpdateData(new_data) => {
                self.data = new_data;
            }
            SlotInput::MoveUpClicked => {
                sender
                    .output(SlotOutput::MoveSlotUp(self.data.slot_id))
                    .unwrap();
            }
            SlotInput::MoveDownClicked => {
                sender
                    .output(SlotOutput::MoveSlotDown(self.data.slot_id))
                    .unwrap();
            }
            SlotInput::DeleteClicked => {
                sender
                    .output(SlotOutput::DeleteSlot(self.data.slot_id))
                    .unwrap();
            }
            SlotInput::EditSlotClicked => {
                sender
                    .output(SlotOutput::EditSlot(self.data.slot_id))
                    .unwrap();
            }
        }
    }
}
