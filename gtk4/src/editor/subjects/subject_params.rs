use adw::prelude::{
    ActionRowExt, ComboRowExt, EditableExt, PreferencesGroupExt, PreferencesRowExt,
};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

use std::num::NonZeroU32;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    params: collomatique_state_colloscopes::SubjectParameters,
    has_interrogations: bool,
    interrogation_params: collomatique_state_colloscopes::SubjectInterrogationParameters,
    global_first_week: Option<collomatique_time::WeekStart>,
    periodicity_panel: PeriodicityPanel,
    exactly_periodic_params: NonZeroU32,
    once_for_every_block_of_weeks_params: OnceForEveryBlockOfWeeksParams,
    amount_in_year_params: AmountInYearParams,
    amount_for_every_arbitrary_block_params: AmountForEveryArbitraryBlockParams,
    blocks: FactoryVecDeque<Block>,
}

pub struct OnceForEveryBlockOfWeeksParams {
    minimum_week_separation: NonZeroU32,
    block_size_in_weeks: NonZeroU32,
}

pub struct AmountForEveryArbitraryBlockParams {
    minimum_week_separation: u32,
    blocks: Vec<collomatique_state_colloscopes::subjects::WeekBlock>,
}

pub struct AmountInYearParams {
    interrogation_count_in_year: std::ops::RangeInclusive<u32>,
    minimum_week_separation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodicityPanel {
    OnceForEveryBlockOfWeeks,
    ExactlyPeriodic,
    AmountInYear,
    AmountForEveryArbitraryBlock,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        Option<collomatique_time::WeekStart>,
        collomatique_state_colloscopes::SubjectParameters,
    ),
    Cancel,
    Accept,

    UpdateName(String),
    UpdateDuration(collomatique_time::NonZeroDurationInMinutes),
    UpdateDurationTakenIntoAccount(bool),
    UpdateHasInterrogations(bool),
    UpdateStudentsPerGroupMinimum(NonZeroU32),
    UpdateStudentsPerGroupMaximum(NonZeroU32),
    UpdateGroupsPerInterrogationMinimum(NonZeroU32),
    UpdateGroupsPerInterrogationMaximum(NonZeroU32),
    UpdatePeriodicityType(PeriodicityPanel),
    UpdateExactlyPeriodicParams(NonZeroU32),
    UpdateOnceEveryBlockOfWeeksParamsPeriodicity(NonZeroU32),
    UpdateOnceEveryBlockOfWeeksParamsSeparation(NonZeroU32),
    UpdateAmountInYearCountMinimum(u32),
    UpdateAmountInYearCountMaximum(u32),
    UpdateAmountInYearWeekSeparation(u32),
    AddArbitraryBlock,
    UpdateEmptyWeeksBeforeBlock(usize, u32),
    UpdateDurationInWeeksOfGivenBlock(usize, NonZeroU32),
    UpdateInterrogationCountMinimum(usize, u32),
    UpdateInterrogationCountMaximum(usize, u32),
    UpdateAmountForEveryArbitraryBlockSeparation(u32),
    DeleteBlock(usize),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::SubjectParameters),
}

impl Dialog {
    fn generate_periodicity_type_model() -> gtk::StringList {
        gtk::StringList::new(&[
            "Programme glissant",
            "Par blocs de semaines",
            "Colles à l'année",
            "Par blocs (arbitraires)",
        ])
    }

    fn periocity_selected_to_enum(selected: u32) -> PeriodicityPanel {
        match selected {
            0 => PeriodicityPanel::ExactlyPeriodic,
            1 => PeriodicityPanel::OnceForEveryBlockOfWeeks,
            2 => PeriodicityPanel::AmountInYear,
            3 => PeriodicityPanel::AmountForEveryArbitraryBlock,
            _ => panic!("Invalid selection for periodicity type"),
        }
    }

    fn periodicity_enum_to_selected(panel: PeriodicityPanel) -> u32 {
        match panel {
            PeriodicityPanel::ExactlyPeriodic => 0,
            PeriodicityPanel::OnceForEveryBlockOfWeeks => 1,
            PeriodicityPanel::AmountInYear => 2,
            PeriodicityPanel::AmountForEveryArbitraryBlock => 3,
        }
    }

    fn periodicity_panel_from_params(
        params: &collomatique_state_colloscopes::SubjectParameters,
    ) -> PeriodicityPanel {
        let Some(interrogation_parameters) = &params.interrogation_parameters else {
            return PeriodicityPanel::ExactlyPeriodic;
        };
        use collomatique_state_colloscopes::SubjectPeriodicity;
        match &interrogation_parameters.periodicity {
            SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year: _,
                minimum_week_separation: _,
            } => PeriodicityPanel::AmountInYear,
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks: _,
            } => PeriodicityPanel::ExactlyPeriodic,
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block: _,
                minimum_week_separation: _,
            } => PeriodicityPanel::OnceForEveryBlockOfWeeks,
            SubjectPeriodicity::AmountForEveryArbitraryBlock {
                blocks: _,
                minimum_week_separation: _,
            } => PeriodicityPanel::AmountForEveryArbitraryBlock,
        }
    }

    fn interrogation_params_from_params(
        params: &collomatique_state_colloscopes::SubjectParameters,
    ) -> collomatique_state_colloscopes::SubjectInterrogationParameters {
        params.interrogation_parameters.clone().unwrap_or_default()
    }

    fn has_interrogations_from_params(
        params: &collomatique_state_colloscopes::SubjectParameters,
    ) -> bool {
        params.interrogation_parameters.is_some()
    }

    fn periodicity_from_params(
        params: &collomatique_state_colloscopes::SubjectParameters,
    ) -> NonZeroU32 {
        let Some(interrogation_parameters) = &params.interrogation_parameters else {
            return NonZeroU32::new(2).unwrap();
        };
        use collomatique_state_colloscopes::SubjectPeriodicity;
        match &interrogation_parameters.periodicity {
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => *periodicity_in_weeks,
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block,
                minimum_week_separation: _,
            } => *weeks_per_block,
            _ => NonZeroU32::new(2).unwrap(),
        }
    }

    fn once_for_every_block_of_weeks_params_from_params(
        params: &collomatique_state_colloscopes::SubjectParameters,
    ) -> OnceForEveryBlockOfWeeksParams {
        let Some(interrogation_parameters) = &params.interrogation_parameters else {
            return OnceForEveryBlockOfWeeksParams {
                minimum_week_separation: NonZeroU32::new(1).unwrap(),
                block_size_in_weeks: NonZeroU32::new(2).unwrap(),
            };
        };
        use collomatique_state_colloscopes::SubjectPeriodicity;
        match &interrogation_parameters.periodicity {
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => OnceForEveryBlockOfWeeksParams {
                minimum_week_separation: NonZeroU32::new(1).unwrap(),
                block_size_in_weeks: *periodicity_in_weeks,
            },
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block,
                minimum_week_separation,
            } => OnceForEveryBlockOfWeeksParams {
                minimum_week_separation: *minimum_week_separation,
                block_size_in_weeks: *weeks_per_block,
            },
            _ => OnceForEveryBlockOfWeeksParams {
                minimum_week_separation: NonZeroU32::new(1).unwrap(),
                block_size_in_weeks: NonZeroU32::new(2).unwrap(),
            },
        }
    }

    fn amount_in_year_params_from_params(
        params: &collomatique_state_colloscopes::SubjectParameters,
    ) -> AmountInYearParams {
        let Some(interrogation_parameters) = &params.interrogation_parameters else {
            return AmountInYearParams {
                interrogation_count_in_year: 2..=2,
                minimum_week_separation: 1,
            };
        };
        use collomatique_state_colloscopes::SubjectPeriodicity;
        match &interrogation_parameters.periodicity {
            SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            } => AmountInYearParams {
                interrogation_count_in_year: interrogation_count_in_year.clone(),
                minimum_week_separation: *minimum_week_separation,
            },
            _ => AmountInYearParams {
                interrogation_count_in_year: 2..=2,
                minimum_week_separation: 1,
            },
        }
    }

    fn amount_for_every_arbitrary_block_params_from_params(
        params: &collomatique_state_colloscopes::SubjectParameters,
    ) -> AmountForEveryArbitraryBlockParams {
        let Some(interrogation_parameters) = &params.interrogation_parameters else {
            return AmountForEveryArbitraryBlockParams {
                blocks: vec![],
                minimum_week_separation: 0,
            };
        };

        use collomatique_state_colloscopes::SubjectPeriodicity;

        match &interrogation_parameters.periodicity {
            SubjectPeriodicity::AmountForEveryArbitraryBlock {
                blocks,
                minimum_week_separation,
            } => AmountForEveryArbitraryBlockParams {
                blocks: blocks.clone(),
                minimum_week_separation: *minimum_week_separation,
            },
            _ => AmountForEveryArbitraryBlockParams {
                blocks: vec![],
                minimum_week_separation: 0,
            },
        }
    }

    fn synchronize_block_factory(&mut self) {
        let params: Vec<_> = self
            .amount_for_every_arbitrary_block_params
            .blocks
            .iter()
            .scan(0u32, |first_available_week, block_params| {
                let new_block = BlockData {
                    global_first_week: self.global_first_week.clone(),
                    first_available_week: *first_available_week,
                    block_params: block_params.clone(),
                };
                *first_available_week +=
                    block_params.delay_in_weeks + block_params.size_in_weeks.get();
                Some(new_block)
            })
            .collect();
        crate::tools::factories::update_vec_deque(&mut self.blocks, params.into_iter(), |x| {
            BlockInput::UpdateData(x)
        });
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
            set_title: Some("Configuration de la matière"),
            set_default_size: (500, 600),
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
                                set_title: "Nom de la matière",
                                #[track(model.should_redraw)]
                                set_text: &model.params.name,
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateName(text));
                                },
                            },
                            adw::SwitchRow {
                                set_hexpand: true,
                                set_use_markup: false,
                                set_title: "Pas de colles",
                                set_subtitle: "Cette matière n'a que des cours",
                                #[track(model.should_redraw)]
                                set_active: !model.has_interrogations,
                                connect_active_notify[sender] => move |widget| {
                                    let no_interrogations = widget.is_active();
                                    sender.input(DialogInput::UpdateHasInterrogations(!no_interrogations));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Durée des colles",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: model.has_interrogations,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Durée d'une colle (en minutes)",
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
                                set_value: model.interrogation_params.duration.get().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let duration_u32 = widget.value() as u32;
                                    let duration = collomatique_time::NonZeroDurationInMinutes::new(duration_u32).unwrap();
                                    sender.input(DialogInput::UpdateDuration(duration));
                                },
                            },
                            adw::SwitchRow {
                                set_hexpand: true,
                                set_use_markup: false,
                                set_title: "Durée compatibilisée",
                                set_subtitle: "Pour équilibrer le nombre d'heures par semaine",
                                #[track(model.should_redraw)]
                                set_active: model.interrogation_params.take_duration_into_account,
                                connect_active_notify[sender] => move |widget| {
                                    let duration_taken_into_account = widget.is_active();
                                    sender.input(DialogInput::UpdateDurationTakenIntoAccount(duration_taken_into_account));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Élèves par groupe",
                            set_description: Some("Nombre d'élèves minimum et maximum dans les groupes"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: model.has_interrogations,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Minimum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    #[watch]
                                    set_upper: model.interrogation_params.students_per_group.end().get() as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.interrogation_params.students_per_group.start().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let students_per_group_min_u32 = widget.value() as u32;
                                    let students_per_group_min = NonZeroU32::new(students_per_group_min_u32).unwrap();
                                    sender.input(DialogInput::UpdateStudentsPerGroupMinimum(students_per_group_min));
                                },
                            },
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Maximum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    #[watch]
                                    set_lower: model.interrogation_params.students_per_group.start().get() as f64,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.interrogation_params.students_per_group.end().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let students_per_group_max_u32 = widget.value() as u32;
                                    let students_per_group_max = NonZeroU32::new(students_per_group_max_u32).unwrap();
                                    sender.input(DialogInput::UpdateStudentsPerGroupMaximum(students_per_group_max));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Groupes par colle",
                            set_description: Some("Nombre de groupes à coller simultanément"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: model.has_interrogations,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Minimum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    #[watch]
                                    set_upper: model.interrogation_params.groups_per_interrogation.end().get() as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.interrogation_params.groups_per_interrogation.start().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let groups_per_interrogation_min_u32 = widget.value() as u32;
                                    let groups_per_interrogation_min = NonZeroU32::new(groups_per_interrogation_min_u32).unwrap();
                                    sender.input(DialogInput::UpdateGroupsPerInterrogationMinimum(groups_per_interrogation_min));
                                },
                            },
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Maximum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    #[watch]
                                    set_lower: model.interrogation_params.groups_per_interrogation.start().get() as f64,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.interrogation_params.groups_per_interrogation.end().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let groups_per_interrogation_max_u32 = widget.value() as u32;
                                    let groups_per_interrogation_max = NonZeroU32::new(groups_per_interrogation_max_u32).unwrap();
                                    sender.input(DialogInput::UpdateGroupsPerInterrogationMaximum(groups_per_interrogation_max));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Périodicité",
                            set_description: Some("Périodicité des colles de la matière"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: model.has_interrogations,
                            adw::ComboRow {
                                set_title: "Type de périodicité",
                                set_model: Some(&Self::generate_periodicity_type_model()),
                                #[track(model.should_redraw)]
                                set_selected: Self::periodicity_enum_to_selected(model.periodicity_panel),
                                connect_selected_notify[sender] => move |widget| {
                                    let selected = widget.selected() as u32;
                                    let periodicity_type = Dialog::periocity_selected_to_enum(selected);
                                    sender.input(DialogInput::UpdatePeriodicityType(periodicity_type));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: (model.periodicity_panel == PeriodicityPanel::ExactlyPeriodic) && model.has_interrogations,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Périodicité (en semaines)",
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
                                set_value: model.exactly_periodic_params.get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let periodicity_u32 = widget.value() as u32;
                                    let periodicity = NonZeroU32::new(periodicity_u32).unwrap();
                                    sender.input(DialogInput::UpdateExactlyPeriodicParams(periodicity));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: (model.periodicity_panel == PeriodicityPanel::OnceForEveryBlockOfWeeks) && model.has_interrogations,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Taille des blocs (en semaines)",
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
                                set_value: model.once_for_every_block_of_weeks_params.block_size_in_weeks.get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let periodicity_u32 = widget.value() as u32;
                                    let periodicity = NonZeroU32::new(periodicity_u32).unwrap();
                                    sender.input(DialogInput::UpdateOnceEveryBlockOfWeeksParamsPeriodicity(periodicity));
                                },
                            },
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Séparation minimale (en semaines)",
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
                                set_value: model.once_for_every_block_of_weeks_params.minimum_week_separation.get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let value_u32 = widget.value() as u32;
                                    let value = NonZeroU32::new(value_u32).unwrap();
                                    sender.input(DialogInput::UpdateOnceEveryBlockOfWeeksParamsSeparation(value));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: (model.periodicity_panel == PeriodicityPanel::AmountInYear) && model.has_interrogations,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Minimum de colles dans l'année",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 0.,
                                    #[watch]
                                    set_upper: *model.amount_in_year_params.interrogation_count_in_year.end() as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: *model.amount_in_year_params.interrogation_count_in_year.start() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let value = widget.value() as u32;
                                    sender.input(DialogInput::UpdateAmountInYearCountMinimum(value));
                                },
                            },
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Maximum de colles dans l'année",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    #[watch]
                                    set_lower: *model.amount_in_year_params.interrogation_count_in_year.start() as f64,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: *model.amount_in_year_params.interrogation_count_in_year.end() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let value = widget.value() as u32;
                                    sender.input(DialogInput::UpdateAmountInYearCountMaximum(value));
                                },
                            },
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Séparation minimale (en semaines)",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 0.,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.amount_in_year_params.minimum_week_separation as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let value = widget.value() as u32;
                                    sender.input(DialogInput::UpdateAmountInYearWeekSeparation(value));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: (model.periodicity_panel == PeriodicityPanel::AmountForEveryArbitraryBlock) && model.has_interrogations,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Séparation minimale (en semaines)",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 0.,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.should_redraw)]
                                set_value: model.amount_for_every_arbitrary_block_params.minimum_week_separation as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let value_u32 = widget.value() as u32;
                                    sender.input(DialogInput::UpdateAmountForEveryArbitraryBlockSeparation(value_u32));
                                },
                            },
                        },
                        #[local_ref]
                        block_list -> gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            #[watch]
                            set_visible: (model.periodicity_panel == PeriodicityPanel::AmountForEveryArbitraryBlock) &&
                                (!model.amount_for_every_arbitrary_block_params.blocks.is_empty()) &&
                                model.has_interrogations,
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: (model.periodicity_panel == PeriodicityPanel::AmountForEveryArbitraryBlock) &&
                                model.has_interrogations,
                            adw::ButtonRow {
                                set_hexpand: true,
                                set_title: "Ajouter un bloc",
                                set_start_icon_name: Some("edit-add"),
                                connect_activated[sender] => move |_widget| {
                                    sender.input(DialogInput::AddArbitraryBlock);
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
        let params = collomatique_state_colloscopes::SubjectParameters::default();

        let blocks = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                BlockOutput::UpdateEmptyWeeks(block_num, new_count) => {
                    DialogInput::UpdateEmptyWeeksBeforeBlock(block_num, new_count)
                }
                BlockOutput::UpdateDurationInWeeks(block_num, new_duration) => {
                    DialogInput::UpdateDurationInWeeksOfGivenBlock(block_num, new_duration)
                }
                BlockOutput::DeleteBlock(block_num) => DialogInput::DeleteBlock(block_num),
                BlockOutput::UpdateInterrogationCountMinimum(block_num, new_min) => {
                    DialogInput::UpdateInterrogationCountMinimum(block_num, new_min)
                }
                BlockOutput::UpdateInterrogationCountMaximum(block_num, new_max) => {
                    DialogInput::UpdateInterrogationCountMaximum(block_num, new_max)
                }
            });

        let mut model = Dialog {
            hidden: true,
            should_redraw: false,
            params: params.clone(),
            interrogation_params: Self::interrogation_params_from_params(&params),
            has_interrogations: Self::has_interrogations_from_params(&params),
            global_first_week: None,
            periodicity_panel: Self::periodicity_panel_from_params(&params),
            exactly_periodic_params: Self::periodicity_from_params(&params),
            once_for_every_block_of_weeks_params:
                Self::once_for_every_block_of_weeks_params_from_params(&params),
            amount_in_year_params: Self::amount_in_year_params_from_params(&params),
            amount_for_every_arbitrary_block_params:
                Self::amount_for_every_arbitrary_block_params_from_params(&params),
            blocks,
        };

        model.synchronize_block_factory();

        let block_list = model.blocks.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(global_first_week, params) => {
                self.hidden = false;
                self.should_redraw = true;
                self.periodicity_panel = Self::periodicity_panel_from_params(&params);
                self.exactly_periodic_params = Self::periodicity_from_params(&params);
                self.once_for_every_block_of_weeks_params =
                    Self::once_for_every_block_of_weeks_params_from_params(&params);
                self.amount_in_year_params = Self::amount_in_year_params_from_params(&params);
                self.amount_for_every_arbitrary_block_params =
                    Self::amount_for_every_arbitrary_block_params_from_params(&params);
                self.interrogation_params = Self::interrogation_params_from_params(&params);
                self.has_interrogations = Self::has_interrogations_from_params(&params);
                self.params = params;
                self.global_first_week = global_first_week;
                self.synchronize_block_factory();
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                self.interrogation_params.periodicity = match self.periodicity_panel {
                    PeriodicityPanel::ExactlyPeriodic => {
                        collomatique_state_colloscopes::SubjectPeriodicity::ExactlyPeriodic {
                            periodicity_in_weeks: self.exactly_periodic_params,
                        }
                    }
                    PeriodicityPanel::OnceForEveryBlockOfWeeks => {
                        collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                            weeks_per_block: self.once_for_every_block_of_weeks_params.block_size_in_weeks,
                            minimum_week_separation: self.once_for_every_block_of_weeks_params.minimum_week_separation,
                        }
                    }
                    PeriodicityPanel::AmountInYear => {
                        collomatique_state_colloscopes::SubjectPeriodicity::AmountInYear {
                            interrogation_count_in_year: self.amount_in_year_params.interrogation_count_in_year.clone(),
                            minimum_week_separation: self.amount_in_year_params.minimum_week_separation,
                        }
                    }
                    PeriodicityPanel::AmountForEveryArbitraryBlock => {
                        collomatique_state_colloscopes::SubjectPeriodicity::AmountForEveryArbitraryBlock {
                            minimum_week_separation: self.amount_for_every_arbitrary_block_params.minimum_week_separation,
                            blocks: self.amount_for_every_arbitrary_block_params.blocks.clone(),
                        }
                    }
                };
                self.params.interrogation_parameters = if self.has_interrogations {
                    Some(self.interrogation_params.clone())
                } else {
                    None
                };
                sender
                    .output(DialogOutput::Accepted(self.params.clone()))
                    .unwrap();
            }
            DialogInput::UpdateName(new_name) => {
                if self.params.name == new_name {
                    return;
                }
                self.params.name = new_name;
            }
            DialogInput::UpdateDuration(new_duration) => {
                if self.interrogation_params.duration == new_duration {
                    return;
                }
                self.interrogation_params.duration = new_duration;
            }
            DialogInput::UpdateDurationTakenIntoAccount(duration_taken_into_account) => {
                if self.interrogation_params.take_duration_into_account
                    == duration_taken_into_account
                {
                    return;
                }
                self.interrogation_params.take_duration_into_account = duration_taken_into_account;
            }
            DialogInput::UpdateHasInterrogations(has_interrogations) => {
                if self.has_interrogations == has_interrogations {
                    return;
                }
                self.has_interrogations = has_interrogations;
            }
            DialogInput::UpdateStudentsPerGroupMinimum(new_min) => {
                if *self.interrogation_params.students_per_group.start() == new_min {
                    return;
                }
                let old_max = self.interrogation_params.students_per_group.end().clone();
                assert!(new_min <= old_max);
                self.interrogation_params.students_per_group = new_min..=old_max;
            }
            DialogInput::UpdateStudentsPerGroupMaximum(new_max) => {
                if *self.interrogation_params.students_per_group.end() == new_max {
                    return;
                }
                let old_min = self.interrogation_params.students_per_group.start().clone();
                assert!(old_min <= new_max);
                self.interrogation_params.students_per_group = old_min..=new_max;
            }
            DialogInput::UpdateGroupsPerInterrogationMinimum(new_min) => {
                if *self.interrogation_params.groups_per_interrogation.start() == new_min {
                    return;
                }
                let old_max = self
                    .interrogation_params
                    .groups_per_interrogation
                    .end()
                    .clone();
                assert!(new_min <= old_max);
                self.interrogation_params.groups_per_interrogation = new_min..=old_max;
            }
            DialogInput::UpdateGroupsPerInterrogationMaximum(new_max) => {
                if *self.interrogation_params.groups_per_interrogation.end() == new_max {
                    return;
                }
                let old_min = self
                    .interrogation_params
                    .groups_per_interrogation
                    .start()
                    .clone();
                assert!(old_min <= new_max);
                self.interrogation_params.groups_per_interrogation = old_min..=new_max;
            }
            DialogInput::UpdatePeriodicityType(new_periodicity_type) => {
                if self.periodicity_panel == new_periodicity_type {
                    return;
                }
                self.periodicity_panel = new_periodicity_type;
            }
            DialogInput::UpdateExactlyPeriodicParams(new_periodicity) => {
                if self.exactly_periodic_params == new_periodicity {
                    return;
                }
                self.exactly_periodic_params = new_periodicity;
            }
            DialogInput::UpdateOnceEveryBlockOfWeeksParamsPeriodicity(new_periodicity) => {
                if self
                    .once_for_every_block_of_weeks_params
                    .block_size_in_weeks
                    == new_periodicity
                {
                    return;
                }
                self.once_for_every_block_of_weeks_params
                    .block_size_in_weeks = new_periodicity;
            }
            DialogInput::UpdateOnceEveryBlockOfWeeksParamsSeparation(new_separation) => {
                if self
                    .once_for_every_block_of_weeks_params
                    .minimum_week_separation
                    == new_separation
                {
                    return;
                }
                self.once_for_every_block_of_weeks_params
                    .minimum_week_separation = new_separation;
            }
            DialogInput::UpdateAmountInYearCountMinimum(new_min) => {
                if *self
                    .amount_in_year_params
                    .interrogation_count_in_year
                    .start()
                    == new_min
                {
                    return;
                }
                let old_max = *self.amount_in_year_params.interrogation_count_in_year.end();
                assert!(new_min <= old_max);
                self.amount_in_year_params.interrogation_count_in_year = new_min..=old_max;
            }
            DialogInput::UpdateAmountInYearCountMaximum(new_max) => {
                if *self.amount_in_year_params.interrogation_count_in_year.end() == new_max {
                    return;
                }
                let old_min = *self
                    .amount_in_year_params
                    .interrogation_count_in_year
                    .start();
                assert!(old_min <= new_max);
                self.amount_in_year_params.interrogation_count_in_year = old_min..=new_max;
            }
            DialogInput::UpdateAmountInYearWeekSeparation(new_sep) => {
                if self.amount_in_year_params.minimum_week_separation == new_sep {
                    return;
                }
                self.amount_in_year_params.minimum_week_separation = new_sep;
            }
            DialogInput::AddArbitraryBlock => {
                self.amount_for_every_arbitrary_block_params.blocks.push(
                    collomatique_state_colloscopes::subjects::WeekBlock {
                        delay_in_weeks: 0,
                        size_in_weeks: NonZeroU32::new(1).unwrap(),
                        interrogation_count_in_block: 1..=1,
                    },
                );
                self.synchronize_block_factory();
            }
            DialogInput::UpdateEmptyWeeksBeforeBlock(block_num, new_count) => {
                self.amount_for_every_arbitrary_block_params.blocks[block_num].delay_in_weeks =
                    new_count;
                self.synchronize_block_factory();
            }
            DialogInput::UpdateDurationInWeeksOfGivenBlock(block_num, new_duration) => {
                self.amount_for_every_arbitrary_block_params.blocks[block_num].size_in_weeks =
                    new_duration;
                self.synchronize_block_factory();
            }
            DialogInput::UpdateInterrogationCountMinimum(block_num, new_min) => {
                let old_max = *self.amount_for_every_arbitrary_block_params.blocks[block_num]
                    .interrogation_count_in_block
                    .end();
                assert!(new_min <= old_max);
                self.amount_for_every_arbitrary_block_params.blocks[block_num]
                    .interrogation_count_in_block = new_min..=old_max;
            }
            DialogInput::UpdateInterrogationCountMaximum(block_num, new_max) => {
                let old_min = *self.amount_for_every_arbitrary_block_params.blocks[block_num]
                    .interrogation_count_in_block
                    .start();
                assert!(old_min <= new_max);
                self.amount_for_every_arbitrary_block_params.blocks[block_num]
                    .interrogation_count_in_block = old_min..=new_max;
            }
            DialogInput::DeleteBlock(block_num) => {
                self.amount_for_every_arbitrary_block_params
                    .blocks
                    .remove(block_num);
                self.synchronize_block_factory();
            }
            DialogInput::UpdateAmountForEveryArbitraryBlockSeparation(new_separation) => {
                if self
                    .amount_for_every_arbitrary_block_params
                    .minimum_week_separation
                    == new_separation
                {
                    return;
                }
                self.amount_for_every_arbitrary_block_params
                    .minimum_week_separation = new_separation;
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

#[derive(Debug, Clone)]
pub struct BlockData {
    pub global_first_week: Option<collomatique_time::WeekStart>,
    pub first_available_week: u32,
    pub block_params: collomatique_state_colloscopes::subjects::WeekBlock,
}

#[derive(Debug)]
pub struct Block {
    data: BlockData,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
pub enum BlockInput {
    UpdateData(BlockData),

    DeleteBlock,
    UpdateEmptyWeeks(u32),
    UpdateDurationInWeeks(NonZeroU32),
    UpdateInterrogationCountMinimum(u32),
    UpdateInterrogationCountMaximum(u32),
}

#[derive(Debug)]
pub enum BlockOutput {
    UpdateEmptyWeeks(usize, u32),
    UpdateDurationInWeeks(usize, NonZeroU32),
    DeleteBlock(usize),
    UpdateInterrogationCountMinimum(usize, u32),
    UpdateInterrogationCountMaximum(usize, u32),
}

impl Block {
    fn generate_title_text(&self) -> String {
        let week_count = self.data.block_params.size_in_weeks.get() as usize;
        let first_week_num =
            (self.data.first_available_week + self.data.block_params.delay_in_weeks) as usize;

        super::super::generate_week_succession_title(
            "Bloc",
            &self.data.global_first_week,
            self.index.current_index(),
            first_week_num,
            week_count,
        )
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Block {
    type Init = BlockData;
    type Input = BlockInput;
    type Output = BlockOutput;
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
                        sender.input(BlockInput::DeleteBlock);
                    }
                },
            },
            adw::SpinRow {
                set_hexpand: true,
                set_title: "Semaines vides qui précèdent",
                #[wrap(Some)]
                set_adjustment = &gtk::Adjustment {
                    set_lower: 0.,
                    set_upper: u32::MAX as f64,
                    set_step_increment: 1.,
                    set_page_increment: 5.,
                },
                set_wrap: false,
                set_snap_to_ticks: true,
                set_numeric: true,
                #[track(self.should_redraw)]
                set_value: self.data.block_params.delay_in_weeks as f64,
                connect_value_notify[sender] => move |widget| {
                    let value_u32 = widget.value() as u32;
                    sender.input(
                        BlockInput::UpdateEmptyWeeks(value_u32)
                    );
                },
            },
            adw::SpinRow {
                set_hexpand: true,
                set_title: "Durée du bloc (en semaines)",
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
                set_value: self.data.block_params.size_in_weeks.get() as f64,
                connect_value_notify[sender] => move |widget| {
                    let value_u32 = widget.value() as u32;
                    let value = NonZeroU32::new(value_u32).unwrap();
                    sender.input(
                        BlockInput::UpdateDurationInWeeks(value)
                    );
                },
            },
            adw::SpinRow {
                set_hexpand: true,
                set_title: "Nombre de colles (minimum)",
                #[wrap(Some)]
                set_adjustment = &gtk::Adjustment {
                    set_lower: 0.,
                    #[watch]
                    set_upper: *self.data.block_params.interrogation_count_in_block.end() as f64,
                    set_step_increment: 1.,
                    set_page_increment: 5.,
                },
                set_wrap: false,
                set_snap_to_ticks: true,
                set_numeric: true,
                #[track(self.should_redraw)]
                set_value: *self.data.block_params.interrogation_count_in_block.start() as f64,
                connect_value_notify[sender] => move |widget| {
                    let value_u32 = widget.value() as u32;
                    sender.input(
                        BlockInput::UpdateInterrogationCountMinimum(value_u32)
                    );
                },
            },
            adw::SpinRow {
                set_hexpand: true,
                set_title: "Nombre de colles (maximum)",
                #[wrap(Some)]
                set_adjustment = &gtk::Adjustment {
                    #[watch]
                    set_lower: *self.data.block_params.interrogation_count_in_block.start() as f64,
                    set_upper: u32::MAX as f64,
                    set_step_increment: 1.,
                    set_page_increment: 5.,
                },
                set_wrap: false,
                set_snap_to_ticks: true,
                set_numeric: true,
                #[track(self.should_redraw)]
                set_value: *self.data.block_params.interrogation_count_in_block.end() as f64,
                connect_value_notify[sender] => move |widget| {
                    let value_u32 = widget.value() as u32;
                    sender.input(
                        BlockInput::UpdateInterrogationCountMaximum(value_u32)
                    );
                },
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
            BlockInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            BlockInput::DeleteBlock => {
                sender
                    .output_sender()
                    .send(BlockOutput::DeleteBlock(self.index.current_index()))
                    .unwrap();
            }
            BlockInput::UpdateDurationInWeeks(new_duration) => {
                if self.data.block_params.size_in_weeks == new_duration {
                    return;
                }
                self.data.block_params.size_in_weeks = new_duration;
                sender
                    .output_sender()
                    .send(BlockOutput::UpdateDurationInWeeks(
                        self.index.current_index(),
                        new_duration,
                    ))
                    .unwrap();
            }
            BlockInput::UpdateEmptyWeeks(new_count) => {
                if self.data.block_params.delay_in_weeks == new_count {
                    return;
                }
                self.data.block_params.delay_in_weeks = new_count;
                sender
                    .output_sender()
                    .send(BlockOutput::UpdateEmptyWeeks(
                        self.index.current_index(),
                        new_count,
                    ))
                    .unwrap();
            }
            BlockInput::UpdateInterrogationCountMinimum(new_min) => {
                if *self.data.block_params.interrogation_count_in_block.start() == new_min {
                    return;
                }
                let old_max = *self.data.block_params.interrogation_count_in_block.end();
                assert!(new_min <= old_max);
                self.data.block_params.interrogation_count_in_block = new_min..=old_max;
                sender
                    .output_sender()
                    .send(BlockOutput::UpdateInterrogationCountMinimum(
                        self.index.current_index(),
                        new_min,
                    ))
                    .unwrap();
            }
            BlockInput::UpdateInterrogationCountMaximum(new_max) => {
                if *self.data.block_params.interrogation_count_in_block.end() == new_max {
                    return;
                }
                let old_min = *self.data.block_params.interrogation_count_in_block.start();
                assert!(old_min <= new_max);
                self.data.block_params.interrogation_count_in_block = old_min..=new_max;
                sender
                    .output_sender()
                    .send(BlockOutput::UpdateInterrogationCountMaximum(
                        self.index.current_index(),
                        new_max,
                    ))
                    .unwrap();
            }
        }
    }
}
