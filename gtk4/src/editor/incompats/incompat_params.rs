use std::num::NonZeroU32;

use adw::prelude::{ComboRowExt, EditableExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    subjects: collomatique_state_colloscopes::subjects::Subjects<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    >,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns<
        collomatique_state_colloscopes::WeekPatternId,
    >,
    ordered_subjects: Vec<(collomatique_state_colloscopes::SubjectId, String)>,
    ordered_week_patterns: Vec<(collomatique_state_colloscopes::WeekPatternId, String)>,

    incompat_name: String,
    subject_selected: u32,
    minimum_free_slots: u32,
    week_pattern_selected: u32,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::subjects::Subjects<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::week_patterns::WeekPatterns<
            collomatique_state_colloscopes::WeekPatternId,
        >,
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ),
    Cancel,
    Accept,

    UpdateIncompatName(String),
    UpdateSelectedSubject(u32),
    UpdateSelectedWeekPattern(u32),
    UpdateSelectedMinimumFreeSlots(u32),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ),
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
                #[name(scrolled_window)]
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
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns::default(),
            ordered_subjects: Vec::new(),
            ordered_week_patterns: Vec::new(),
            subject_selected: gtk::INVALID_LIST_POSITION,
            week_pattern_selected: 0,
            incompat_name: String::new(),
            minimum_free_slots: 1,
        };

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
        }
    }
}

impl Dialog {
    fn update_data_from_params(
        &mut self,
        params: &collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ) {
        self.subject_selected = self.subject_id_to_selected(params.subject_id);
        self.week_pattern_selected = self.week_pattern_id_to_selected(params.week_pattern_id);
        self.incompat_name = params.name.clone();
        self.minimum_free_slots = params.minimum_free_slots.get();
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

    fn build_params_from_data(
        &self,
    ) -> collomatique_state_colloscopes::incompats::Incompatibility<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::WeekPatternId,
    > {
        collomatique_state_colloscopes::incompats::Incompatibility {
            name: self.incompat_name.clone(),
            subject_id: self.subject_selected_to_id(self.subject_selected),
            slots: Vec::new(),
            minimum_free_slots: NonZeroU32::new(self.minimum_free_slots)
                .expect("Minimum free slots should be in allowed range"),
            week_pattern_id: self.week_pattern_selected_to_id(self.week_pattern_selected),
        }
    }
}
