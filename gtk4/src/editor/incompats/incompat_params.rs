use std::num::NonZeroU32;

use adw::prelude::{
    ActionRowExt, ComboRowExt, EditableExt, PreferencesGroupExt, PreferencesRowExt,
};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns,
    ordered_subjects: Vec<(collomatique_state_colloscopes::SubjectId, String)>,
    ordered_week_patterns: Vec<(collomatique_state_colloscopes::WeekPatternId, String)>,

    incompat_name: String,
    subject_selected: u32,
    minimum_free_slots: u32,
    week_pattern_selected: u32,
    slots_data: Vec<collomatique_time::SlotWithDuration>,
    slots: FactoryVecDeque<Slot>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::week_patterns::WeekPatterns,
        collomatique_state_colloscopes::incompats::Incompatibility,
    ),
    Cancel,
    Accept,

    UpdateIncompatName(String),
    UpdateSelectedSubject(u32),
    UpdateSelectedWeekPattern(u32),
    UpdateSelectedMinimumFreeSlots(u32),
    AddSlot,
    DeleteSlot(usize),
    UpdateSlot(usize, collomatique_time::SlotWithDuration),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::incompats::Incompatibility),
}

impl Dialog {
    fn generate_subjects_model(&self) -> gtk::StringList {
        let subject_names_list: Vec<_> = self
            .ordered_subjects
            .iter()
            .map(|(_id, name)| name.as_str())
            .collect();
        gtk::StringList::new(&subject_names_list[..])
    }

    fn subject_id_to_selected(&self, subject_id: collomatique_state_colloscopes::SubjectId) -> u32 {
        for (i, (id, _)) in self.ordered_subjects.iter().enumerate() {
            if *id == subject_id {
                return i as u32;
            }
        }
        panic!("Subject ID should be in list");
    }

    fn subject_selected_to_id(&self, selected: u32) -> collomatique_state_colloscopes::SubjectId {
        self.ordered_subjects[selected as usize].0
    }

    fn generate_week_patterns_model(&self) -> gtk::StringList {
        let week_pattern_names_list: Vec<_> = ["Aucun (toutes les semaines)"]
            .into_iter()
            .chain(
                self.ordered_week_patterns
                    .iter()
                    .map(|(_id, name)| name.as_str()),
            )
            .collect();
        gtk::StringList::new(&week_pattern_names_list[..])
    }

    fn week_pattern_id_to_selected(
        &self,
        week_pattern_id_opt: Option<collomatique_state_colloscopes::WeekPatternId>,
    ) -> u32 {
        let Some(week_pattern_id) = week_pattern_id_opt else {
            return 0;
        };
        for (i, (id, _)) in self.ordered_week_patterns.iter().enumerate() {
            if *id == week_pattern_id {
                return (i as u32) + 1;
            }
        }
        panic!("Week pattern ID should be in list");
    }

    fn week_pattern_selected_to_id(
        &self,
        selected: u32,
    ) -> Option<collomatique_state_colloscopes::WeekPatternId> {
        if selected == 0 {
            return None;
        }
        Some(self.ordered_week_patterns[(selected - 1) as usize].0)
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
            #[watch]
            set_title: Some("Configuration de l'incompatibilité horaire"),
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
                                set_title: "Nom de l'incompatibilité",
                                #[track(model.should_redraw)]
                                set_text: &model.incompat_name,
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateIncompatName(text));
                                },
                            },
                            adw::ComboRow {
                                set_title: "Matière",
                                #[track(model.should_redraw)]
                                set_model: Some(&model.generate_subjects_model()),
                                #[track(model.should_redraw)]
                                set_selected: model.subject_selected,
                                connect_selected_notify[sender] => move |widget| {
                                    let selected = widget.selected() as u32;
                                    sender.input(DialogInput::UpdateSelectedSubject(selected));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Périodicité",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ComboRow {
                                set_title: "Modèle à utiliser",
                                #[track(model.should_redraw)]
                                set_model: Some(&model.generate_week_patterns_model()),
                                #[track(model.should_redraw)]
                                set_selected: model.week_pattern_selected,
                                connect_selected_notify[sender] => move |widget| {
                                    let selected = widget.selected() as u32;
                                    sender.input(DialogInput::UpdateSelectedWeekPattern(selected));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Créneaux",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Nombre minimum de créneaux libres",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.minimum_free_slots as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let minimum_free_slots = widget.value() as u32;
                                    sender.input(DialogInput::UpdateSelectedMinimumFreeSlots(minimum_free_slots));
                                },
                            },
                        },
                        #[local_ref]
                        slot_list -> gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            #[watch]
                            set_visible: !model.slots_data.is_empty(),
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ButtonRow {
                                set_hexpand: true,
                                set_title: "Ajouter un créneau",
                                set_start_icon_name: Some("edit-add"),
                                connect_activated[sender] => move |_widget| {
                                    sender.input(DialogInput::AddSlot);
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
        let slots = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                SlotOutput::DeleteSlot(slot_num) => DialogInput::DeleteSlot(slot_num),
                SlotOutput::UpdateSlot(slot_num, slot) => DialogInput::UpdateSlot(slot_num, slot),
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns::default(),
            ordered_subjects: Vec::new(),
            ordered_week_patterns: Vec::new(),
            subject_selected: gtk::INVALID_LIST_POSITION,
            week_pattern_selected: 0,
            incompat_name: String::new(),
            minimum_free_slots: 1,
            slots,
            slots_data: Vec::new(),
        };

        let slot_list = model.slots.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(subjects, week_patterns, params) => {
                self.hidden = false;
                self.should_redraw = true;
                self.subjects = subjects;
                self.week_patterns = week_patterns;
                self.build_ordered_subjects();
                self.build_ordered_week_patterns();
                self.update_data_from_params(&params);
                self.rebuild_slots();
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.build_params_from_data()))
                    .unwrap();
            }
            DialogInput::UpdateSelectedSubject(subject_selected) => {
                self.subject_selected = subject_selected;
            }
            DialogInput::UpdateSelectedWeekPattern(week_pattern_selected) => {
                self.week_pattern_selected = week_pattern_selected;
            }
            DialogInput::UpdateIncompatName(incompat_name) => {
                self.incompat_name = incompat_name;
            }
            DialogInput::UpdateSelectedMinimumFreeSlots(minimum_free_slots) => {
                self.minimum_free_slots = minimum_free_slots;
            }
            DialogInput::AddSlot => {
                self.slots_data.push(
                    collomatique_time::SlotWithDuration::new(
                        collomatique_time::SlotStart {
                            weekday: chrono::Weekday::Mon.into(),
                            start_time: collomatique_time::WholeMinuteTime::new(
                                chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                            )
                            .unwrap(),
                        },
                        collomatique_time::NonZeroMinutes::new(60).unwrap(),
                    )
                    .unwrap(),
                );
                self.rebuild_slots();
            }
            DialogInput::UpdateSlot(slot_num, slot) => {
                self.slots_data[slot_num] = slot;
            }
            DialogInput::DeleteSlot(slot_num) => {
                self.slots_data.remove(slot_num);
                self.rebuild_slots();
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
    fn rebuild_slots(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.slots,
            self.slots_data.iter().cloned(),
            |x| SlotInput::UpdateData(x),
        );
    }

    fn update_data_from_params(
        &mut self,
        params: &collomatique_state_colloscopes::incompats::Incompatibility,
    ) {
        self.subject_selected = self.subject_id_to_selected(params.subject_id);
        self.week_pattern_selected = self.week_pattern_id_to_selected(params.week_pattern_id);
        self.incompat_name = params.name.clone();
        self.minimum_free_slots = params.minimum_free_slots.get();
        self.slots_data = params.slots.clone();
    }

    fn build_ordered_subjects(&mut self) {
        let mut subjects: Vec<_> = self
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(subject_id, subject)| (subject_id.clone(), subject.parameters.name.clone()))
            .collect();
        subjects.sort_by_key(|(id, name)| (name.clone(), id.clone()));
        self.ordered_subjects = subjects;
    }

    fn build_ordered_week_patterns(&mut self) {
        let mut week_patterns: Vec<_> = self
            .week_patterns
            .week_pattern_map
            .iter()
            .map(|(week_pattern_id, week_pattern)| {
                (week_pattern_id.clone(), week_pattern.name.clone())
            })
            .collect();
        week_patterns.sort_by_key(|(id, name)| (name.clone(), id.clone()));
        self.ordered_week_patterns = week_patterns;
    }

    fn build_params_from_data(&self) -> collomatique_state_colloscopes::incompats::Incompatibility {
        collomatique_state_colloscopes::incompats::Incompatibility {
            name: self.incompat_name.clone(),
            subject_id: self.subject_selected_to_id(self.subject_selected),
            slots: self.slots_data.clone(),
            minimum_free_slots: NonZeroU32::new(self.minimum_free_slots)
                .expect("Minimum free slots should be in allowed range"),
            week_pattern_id: self.week_pattern_selected_to_id(self.week_pattern_selected),
        }
    }
}

#[derive(Debug)]
pub struct Slot {
    index: DynamicIndex,
    data: collomatique_time::SlotWithDuration,
    should_redraw: bool,
    day_selected: u32,
    hour_selected: u32,
    minute_selected: u32,
    duration_selected: u32,
}

#[derive(Debug, Clone)]
pub enum SlotInput {
    UpdateData(collomatique_time::SlotWithDuration),

    DeleteSlot,
    UpdateSelectedDay(u32),
    UpdateSelectedHour(u32),
    UpdateSelectedMinute(u32),
    UpdateSelectedDuration(u32),
}

#[derive(Debug)]
pub enum SlotOutput {
    DeleteSlot(usize),
    UpdateSlot(usize, collomatique_time::SlotWithDuration),
}

impl Slot {
    fn generate_days_model() -> gtk::StringList {
        gtk::StringList::new(&[
            "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
        ])
    }

    fn generate_title_text(&self) -> String {
        format!("Créneau {}", self.index.current_index() + 1)
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Slot {
    type Init = collomatique_time::SlotWithDuration;
    type Input = SlotInput;
    type Output = SlotOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        root_widget = adw::PreferencesGroup {
            set_margin_all: 5,
            set_hexpand: true,
            adw::ActionRow {
                set_hexpand: true,
                #[watch]
                set_title: &self.generate_title_text(),
                add_suffix = &gtk::Button {
                    add_css_class: "flat",
                    set_icon_name: "edit-delete",
                    connect_clicked[sender] => move |_widget| {
                        sender.input(SlotInput::DeleteSlot);
                    }
                },
            },
            adw::ComboRow {
                set_title: "Jour",
                #[track(self.should_redraw)]
                set_model: Some(&Self::generate_days_model()),
                #[track(self.should_redraw)]
                set_selected: self.day_selected,
                connect_selected_notify[sender] => move |widget| {
                    let selected = widget.selected() as u32;
                    sender.input(SlotInput::UpdateSelectedDay(selected));
                },
            },
            adw::SpinRow {
                set_hexpand: true,
                set_title: "Horaire de début (heure)",
                #[wrap(Some)]
                set_adjustment = &gtk::Adjustment {
                    set_lower: 0.,
                    set_upper: 23. as f64,
                    set_step_increment: 1.,
                    set_page_increment: 5.,
                },
                set_wrap: false,
                set_snap_to_ticks: true,
                set_numeric: true,
                #[track(self.should_redraw)]
                set_value: self.hour_selected as f64,
                connect_value_notify[sender] => move |widget| {
                    let hour_u32 = widget.value() as u32;
                    sender.input(SlotInput::UpdateSelectedHour(hour_u32));
                },
            },
            adw::SpinRow {
                set_hexpand: true,
                set_title: "Horaire de début (minute)",
                #[wrap(Some)]
                set_adjustment = &gtk::Adjustment {
                    set_lower: 0.,
                    set_upper: 59. as f64,
                    set_step_increment: 1.,
                    set_page_increment: 5.,
                },
                set_wrap: false,
                set_snap_to_ticks: true,
                set_numeric: true,
                #[track(self.should_redraw)]
                set_value: self.minute_selected as f64,
                connect_value_notify[sender] => move |widget| {
                    let minute_u32 = widget.value() as u32;
                    sender.input(SlotInput::UpdateSelectedMinute(minute_u32));
                },
            },
            adw::SpinRow {
                set_hexpand: true,
                set_title: "Durée (en minutes)",
                #[wrap(Some)]
                set_adjustment = &gtk::Adjustment {
                    set_lower: 1.,
                    set_upper: u32::MAX as f64,
                    set_step_increment: 1.,
                    set_page_increment: 5.,
                },
                set_wrap: false,
                set_snap_to_ticks: true,
                set_numeric: true,
                #[track(self.should_redraw)]
                set_value: self.duration_selected as f64,
                connect_value_notify[sender] => move |widget| {
                    let value_u32 = widget.value() as u32;
                    sender.input(
                        SlotInput::UpdateSelectedDuration(value_u32)
                    );
                },
            },
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let mut model = Self {
            index: index.clone(),
            data,
            day_selected: 0,
            hour_selected: 14,
            minute_selected: 0,
            duration_selected: 60,
            should_redraw: false,
        };
        model.rebuild_from_data();
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
        self.should_redraw = false;
        match msg {
            SlotInput::UpdateData(new_data) => {
                if new_data != self.data {
                    self.should_redraw = true;
                    self.data = new_data;
                    self.rebuild_from_data();
                }
            }
            SlotInput::DeleteSlot => {
                sender
                    .output_sender()
                    .send(SlotOutput::DeleteSlot(self.index.current_index()))
                    .unwrap();
            }
            SlotInput::UpdateSelectedDay(selected_day) => {
                if self.day_selected == selected_day {
                    return;
                }
                self.day_selected = selected_day;
                self.data = self.build_data();
                sender
                    .output_sender()
                    .send(SlotOutput::UpdateSlot(
                        self.index.current_index(),
                        self.data.clone(),
                    ))
                    .unwrap();
            }
            SlotInput::UpdateSelectedHour(selected_hour) => {
                if self.hour_selected == selected_hour {
                    return;
                }
                self.hour_selected = selected_hour;
                self.data = self.build_data();
                sender
                    .output_sender()
                    .send(SlotOutput::UpdateSlot(
                        self.index.current_index(),
                        self.data.clone(),
                    ))
                    .unwrap();
            }
            SlotInput::UpdateSelectedMinute(selected_minute) => {
                if self.minute_selected == selected_minute {
                    return;
                }
                self.minute_selected = selected_minute;
                self.data = self.build_data();
                sender
                    .output_sender()
                    .send(SlotOutput::UpdateSlot(
                        self.index.current_index(),
                        self.data.clone(),
                    ))
                    .unwrap();
            }
            SlotInput::UpdateSelectedDuration(selected_duration) => {
                if self.duration_selected == selected_duration {
                    return;
                }
                self.duration_selected = selected_duration;
                self.data = self.build_data();
                sender
                    .output_sender()
                    .send(SlotOutput::UpdateSlot(
                        self.index.current_index(),
                        self.data.clone(),
                    ))
                    .unwrap();
            }
        }
    }
}

impl Slot {
    fn rebuild_from_data(&mut self) {
        use chrono::Timelike;
        self.day_selected = Self::day_enum_to_selected(self.data.start().weekday);
        self.hour_selected = self.data.start().start_time.inner().hour();
        self.minute_selected = self.data.start().start_time.inner().minute();
        self.duration_selected = self.data.duration().get().get();
    }

    fn build_data(&self) -> collomatique_time::SlotWithDuration {
        collomatique_time::SlotWithDuration::new(
            collomatique_time::SlotStart {
                weekday: Self::day_selected_to_enum(self.day_selected),
                start_time: collomatique_time::WholeMinuteTime::new(
                    chrono::NaiveTime::from_hms_opt(self.hour_selected, self.minute_selected, 0)
                        .expect("Time should be valid"),
                )
                .expect("Time should fall on a whole minute"),
            },
            collomatique_time::NonZeroMinutes::new(self.duration_selected)
                .expect("Duration should be in range"),
        )
        .unwrap()
    }

    fn day_selected_to_enum(selected: u32) -> collomatique_time::Weekday {
        collomatique_time::Weekday(match selected {
            0 => chrono::Weekday::Mon,
            1 => chrono::Weekday::Tue,
            2 => chrono::Weekday::Wed,
            3 => chrono::Weekday::Thu,
            4 => chrono::Weekday::Fri,
            5 => chrono::Weekday::Sat,
            6 => chrono::Weekday::Sun,
            _ => panic!("Invalid selection for slot day"),
        })
    }

    fn day_enum_to_selected(day: collomatique_time::Weekday) -> u32 {
        match day.inner() {
            chrono::Weekday::Mon => 0,
            chrono::Weekday::Tue => 1,
            chrono::Weekday::Wed => 2,
            chrono::Weekday::Thu => 3,
            chrono::Weekday::Fri => 4,
            chrono::Weekday::Sat => 5,
            chrono::Weekday::Sun => 6,
        }
    }
}
