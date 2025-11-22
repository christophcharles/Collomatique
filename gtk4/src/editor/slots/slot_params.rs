use std::collections::BTreeMap;

use adw::prelude::{ComboRowExt, EditableExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    subject_name: String,
    teachers: BTreeMap<
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::teachers::Teacher,
    >,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns,
    ordered_teachers: Vec<(collomatique_state_colloscopes::TeacherId, String, String)>,
    ordered_week_patterns: Vec<(collomatique_state_colloscopes::WeekPatternId, String)>,

    teacher_selected: u32,
    day_selected: u32,
    hour_selected: u32,
    minute_selected: u32,
    week_pattern_selected: u32,
    cost_selected: i32,
    extra_info: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        String,
        BTreeMap<
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::teachers::Teacher,
        >,
        collomatique_state_colloscopes::week_patterns::WeekPatterns,
        collomatique_state_colloscopes::slots::Slot,
    ),
    Cancel,
    Accept,

    UpdateSelectedTeacher(u32),
    UpdateSelectedDay(u32),
    UpdateSelectedHour(u32),
    UpdateSelectedMinute(u32),
    UpdateSelectedWeekPattern(u32),
    UpdateSelectedCost(i32),
    UpdateExtraInfo(String),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::slots::Slot),
}

impl Dialog {
    fn generate_subject_name(&self) -> String {
        format!("Matière concernée : {}", self.subject_name)
    }

    fn generate_teachers_model(&self) -> gtk::StringList {
        let teacher_names: Vec<_> = self
            .ordered_teachers
            .iter()
            .map(|(_id, firstname, surname)| format!("{} {}", firstname, surname))
            .collect();
        let teacher_names_list: Vec<_> = teacher_names.iter().map(|name| name.as_str()).collect();
        gtk::StringList::new(&teacher_names_list[..])
    }

    fn teacher_id_to_selected(&self, teacher_id: collomatique_state_colloscopes::TeacherId) -> u32 {
        for (i, (id, _, _)) in self.ordered_teachers.iter().enumerate() {
            if *id == teacher_id {
                return i as u32;
            }
        }
        panic!("Teacher ID should be in list");
    }

    fn teacher_selected_to_id(&self, selected: u32) -> collomatique_state_colloscopes::TeacherId {
        self.ordered_teachers[selected as usize].0
    }

    fn generate_days_model() -> gtk::StringList {
        gtk::StringList::new(&[
            "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
        ])
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
            set_title: Some("Configuration du créneau de colle"),
            set_default_size: (500, -1),
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
                    set_margin_all: 5,
                    set_spacing: 10,
                    set_orientation: gtk::Orientation::Vertical,
                    adw::PreferencesGroup {
                        set_title: "Paramètres généraux",
                        set_margin_all: 5,
                        set_hexpand: true,
                        adw::ComboRow {
                            set_title: "Colleur",
                            #[track(model.should_redraw)]
                            set_model: Some(&model.generate_teachers_model()),
                            #[track(model.should_redraw)]
                            set_selected: model.teacher_selected,
                            connect_selected_notify[sender] => move |widget| {
                                let selected = widget.selected() as u32;
                                sender.input(DialogInput::UpdateSelectedTeacher(selected));
                            },
                        },
                    },
                    adw::PreferencesGroup {
                        set_title: "Configuration de l'horaire",
                        set_margin_all: 5,
                        set_hexpand: true,
                        adw::ComboRow {
                            set_title: "Jour de la colle",
                            #[track(model.should_redraw)]
                            set_model: Some(&Self::generate_days_model()),
                            #[track(model.should_redraw)]
                            set_selected: model.day_selected,
                            connect_selected_notify[sender] => move |widget| {
                                let selected = widget.selected() as u32;
                                sender.input(DialogInput::UpdateSelectedDay(selected));
                            },
                        },
                        adw::SpinRow {
                            set_hexpand: true,
                            set_title: "Horaire de début de la colle (heure)",
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
                            #[track(model.should_redraw)]
                            set_value: model.hour_selected as f64,
                            connect_value_notify[sender] => move |widget| {
                                let hour_u32 = widget.value() as u32;
                                sender.input(DialogInput::UpdateSelectedHour(hour_u32));
                            },
                        },
                        adw::SpinRow {
                            set_hexpand: true,
                            set_title: "Horaire de début de la colle (minute)",
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
                            #[track(model.should_redraw)]
                            set_value: model.minute_selected as f64,
                            connect_value_notify[sender] => move |widget| {
                                let minute_u32 = widget.value() as u32;
                                sender.input(DialogInput::UpdateSelectedMinute(minute_u32));
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
                        set_title: "Paramètres supplémentaires",
                        set_margin_all: 5,
                        set_hexpand: true,
                        adw::EntryRow {
                            set_hexpand: true,
                            set_title: "Information supplémentaire",
                            #[track(model.should_redraw)]
                            set_text: &model.extra_info,
                            connect_text_notify[sender] => move |widget| {
                                let text : String = widget.text().into();
                                sender.input(DialogInput::UpdateExtraInfo(text));
                            },
                        },
                        adw::SpinRow {
                            set_hexpand: true,
                            set_title: "Coût pour l'optimisation",
                            #[wrap(Some)]
                            set_adjustment = &gtk::Adjustment {
                                set_lower: i32::MIN as f64,
                                set_upper: i32::MAX as f64,
                                set_step_increment: 1.,
                                set_page_increment: 5.,
                            },
                            set_wrap: false,
                            set_snap_to_ticks: true,
                            set_numeric: true,
                            #[track(model.should_redraw)]
                            set_value: model.cost_selected as f64,
                            connect_value_notify[sender] => move |widget| {
                                let cost_selected_i32 = widget.value() as i32;
                                sender.input(DialogInput::UpdateSelectedCost(cost_selected_i32));
                            },
                        },
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        #[watch]
                        set_label: &model.generate_subject_name(),
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
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            subject_name: String::new(),
            teachers: BTreeMap::new(),
            week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns::default(),
            ordered_teachers: Vec::new(),
            ordered_week_patterns: Vec::new(),
            teacher_selected: gtk::INVALID_LIST_POSITION,
            day_selected: 0,
            hour_selected: 18,
            minute_selected: 0,
            week_pattern_selected: 0,
            cost_selected: 0,
            extra_info: String::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(subject_name, teachers, week_patterns, params) => {
                self.hidden = false;
                self.should_redraw = true;
                self.subject_name = subject_name;
                self.teachers = teachers;
                self.week_patterns = week_patterns;
                self.build_ordered_teachers();
                self.build_ordered_week_patterns();
                self.update_data_from_params(&params);
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
            DialogInput::UpdateSelectedTeacher(teacher_selected) => {
                self.teacher_selected = teacher_selected;
            }
            DialogInput::UpdateSelectedDay(day_selected) => {
                self.day_selected = day_selected;
            }
            DialogInput::UpdateSelectedHour(hour_selected) => {
                self.hour_selected = hour_selected;
            }
            DialogInput::UpdateSelectedMinute(minute_selected) => {
                self.minute_selected = minute_selected;
            }
            DialogInput::UpdateSelectedWeekPattern(week_pattern_selected) => {
                self.week_pattern_selected = week_pattern_selected;
            }
            DialogInput::UpdateSelectedCost(cost_selected) => {
                self.cost_selected = cost_selected;
            }
            DialogInput::UpdateExtraInfo(extra_info) => {
                self.extra_info = extra_info;
            }
        }
    }
}

impl Dialog {
    fn update_data_from_params(&mut self, params: &collomatique_state_colloscopes::slots::Slot) {
        use chrono::Timelike;
        self.teacher_selected = self.teacher_id_to_selected(params.teacher_id);
        self.day_selected = Self::day_enum_to_selected(params.start_time.weekday);
        self.hour_selected = params.start_time.start_time.hour();
        self.minute_selected = params.start_time.start_time.minute();
        self.week_pattern_selected = self.week_pattern_id_to_selected(params.week_pattern);
        self.cost_selected = params.cost;
        self.extra_info = params.extra_info.clone();
    }

    fn build_ordered_teachers(&mut self) {
        let mut teachers: Vec<_> = self
            .teachers
            .iter()
            .map(|(teacher_id, teacher)| {
                (
                    teacher_id.clone(),
                    teacher.desc.firstname.clone(),
                    teacher.desc.surname.clone(),
                )
            })
            .collect();
        teachers.sort_by_key(|(id, first_name, last_name)| {
            (last_name.clone(), first_name.clone(), id.clone())
        });
        self.ordered_teachers = teachers;
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

    fn build_params_from_data(&self) -> collomatique_state_colloscopes::slots::Slot {
        let teacher_id = self.teacher_selected_to_id(self.teacher_selected);
        collomatique_state_colloscopes::slots::Slot {
            teacher_id,
            start_time: collomatique_time::SlotStart {
                weekday: Self::day_selected_to_enum(self.day_selected),
                start_time: collomatique_time::TimeOnMinutes::new(
                    chrono::NaiveTime::from_hms_opt(self.hour_selected, self.minute_selected, 0)
                        .unwrap(),
                )
                .unwrap(),
            },
            extra_info: self.extra_info.clone(),
            week_pattern: self.week_pattern_selected_to_id(self.week_pattern_selected),
            cost: self.cost_selected,
        }
    }
}
