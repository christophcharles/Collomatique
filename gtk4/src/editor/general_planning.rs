use collomatique_state::traits::Manager;
use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use select_start_date::DialogOutput;

use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;

mod select_start_date;

#[derive(Debug)]
pub enum GeneralPlanningInput {
    Update(collomatique_state_colloscopes::periods::Periods),

    DeleteFirstWeekClicked,
    EditFirstWeekClicked,
    FirstWeekChanged(collomatique_time::NaiveMondayDate),
}

#[derive(Debug)]
pub enum GeneralPlanningUpdateOp {
    DeleteFirstWeek,
    UpdateFirstWeek(collomatique_time::NaiveMondayDate),
}

impl GeneralPlanningUpdateOp {
    pub fn apply(
        &self,
        data: &mut AppState<Data>,
    ) -> Result<(), collomatique_state_colloscopes::Error> {
        data.apply(match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => collomatique_state_colloscopes::Op::Period(
                collomatique_state_colloscopes::PeriodOp::ChangeStartDate(None),
            ),
            GeneralPlanningUpdateOp::UpdateFirstWeek(date) => {
                collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::ChangeStartDate(Some(date.clone())),
                )
            }
        })
    }
}

pub struct GeneralPlanning {
    periods: collomatique_state_colloscopes::periods::Periods,

    select_start_date_dialog: Controller<select_start_date::Dialog>,
}

impl GeneralPlanning {
    fn generate_first_week_text(&self) -> String {
        format!(
            "<b><big>Début de la première semaine de colles :</big></b> {}",
            match &self.periods.first_week {
                Some(date) => {
                    date.inner().to_string()
                }
                None => "non sélectionné".to_string(),
            }
        )
    }

    fn count_interrogation_weeks(&self) -> usize {
        let mut count = 0usize;
        for (_id, desc) in &self.periods.ordered_period_list {
            for v in desc {
                if *v {
                    count += 1;
                }
            }
        }
        count
    }

    fn generate_interrogation_week_count_text(&self) -> String {
        format!(
            "<b>Nombre total de semaines de colles :</b> {}",
            self.count_interrogation_weeks()
        )
    }
}

#[relm4::component(pub)]
impl Component for GeneralPlanning {
    type Input = GeneralPlanningInput;
    type Output = GeneralPlanningUpdateOp;
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
                set_spacing: 5,
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &model.generate_first_week_text(),
                            set_use_markup: true,
                        },
                        gtk::Button {
                            set_icon_name: "edit-symbolic",
                            add_css_class: "flat",
                            connect_clicked => GeneralPlanningInput::EditFirstWeekClicked,
                        },
                        gtk::Button {
                            #[watch]
                            set_sensitive: model.periods.first_week.is_some(),
                            set_icon_name: "edit-delete",
                            add_css_class: "flat",
                            connect_clicked => GeneralPlanningInput::DeleteFirstWeekClicked,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        #[watch]
                        set_label: &model.generate_interrogation_week_count_text(),
                        set_use_markup: true,
                    },
                },
                /*gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 30,
                    set_spacing: 30,
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b><big>Période 1 du 01/09/2025 au 21/09/2025 (semaines 1 à 3)</big></b>",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Button {
                                set_icon_name: "edit-cut-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 1 du 01/09/2025 au 07/09/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: true,
                                },
                            },
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 2 du 08/09/2025 au 14/09/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: true,
                                },
                            },
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "dimmed",
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 3 du 15/09/2025 au 21/09/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: false,
                                },
                            },
                        }
                    },
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b><big>Période 2 du 22/09/2025 au 05/10/2025  (semaines 4 à 5)</big></b>",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Button {
                                set_icon_name: "edit-cut-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 4 du 22/09/2025 au 28/09/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: true,
                                },
                            },
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 5 du 29/09/2025 au 05/10/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: true,
                                },
                            },
                        }
                    },
                },*/
                gtk::Button {
                    adw::ButtonContent {
                        set_icon_name: "edit-add",
                        set_label: "Ajouter une période",
                    }
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let select_start_date_dialog = select_start_date::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                DialogOutput::Accepted(date) => GeneralPlanningInput::FirstWeekChanged(date),
            });
        let model = GeneralPlanning {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            select_start_date_dialog,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            GeneralPlanningInput::Update(new_periods) => {
                self.periods = new_periods;
            }
            GeneralPlanningInput::DeleteFirstWeekClicked => {
                sender
                    .output(GeneralPlanningUpdateOp::DeleteFirstWeek)
                    .unwrap();
            }
            GeneralPlanningInput::EditFirstWeekClicked => {
                self.select_start_date_dialog
                    .sender()
                    .send(select_start_date::DialogInput::Show(
                        match &self.periods.first_week {
                            Some(date) => date.clone(),
                            None => collomatique_time::NaiveMondayDate::from_today(),
                        },
                    ))
                    .unwrap();
            }
            GeneralPlanningInput::FirstWeekChanged(date) => {
                sender
                    .output(GeneralPlanningUpdateOp::UpdateFirstWeek(date))
                    .unwrap();
            }
        }
    }
}
