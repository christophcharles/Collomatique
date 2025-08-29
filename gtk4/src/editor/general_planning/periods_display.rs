use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;

#[derive(Debug, Clone)]
pub struct EntryData {
    pub global_first_week: Option<collomatique_time::NaiveMondayDate>,
    pub first_week_num: usize,
    pub desc: Vec<bool>,
    pub period_id: collomatique_state_colloscopes::PeriodId,
}

#[derive(Debug)]
pub struct Entry {
    index: DynamicIndex,
    global_first_week: Option<collomatique_time::NaiveMondayDate>,
    first_week_num: usize,
    desc: Vec<bool>,
    period_id: collomatique_state_colloscopes::PeriodId,
}

#[derive(Debug, Clone)]
pub enum EntryInput {
    UpdateData(EntryData),

    EditClicked,
    DeleteClicked,
    CutClicked,
}

#[derive(Debug)]
pub enum EntryOutput {
    EditClicked(collomatique_state_colloscopes::PeriodId),
    DeleteClicked(collomatique_state_colloscopes::PeriodId),
    CutClicked(collomatique_state_colloscopes::PeriodId),
}

impl Entry {
    fn generate_title_text(&self) -> String {
        let week_count = self.desc.len();
        let index = self.index.current_index() + 1;

        if week_count == 0 {
            return format!("<b><big>Période {} (vide)</big></b>", index,);
        }

        let start_week = self.first_week_num + 1;
        let end_week = self.first_week_num + week_count;

        match &self.global_first_week {
            Some(global_start_date) => {
                let start_date = global_start_date
                    .inner()
                    .checked_add_days(chrono::Days::new(7 * (self.first_week_num as u64)))
                    .expect("Valid start date");
                let end_date = start_date
                    .checked_add_days(chrono::Days::new(7 * (week_count as u64) - 1))
                    .expect("Valid end date");
                format!(
                    "<b><big>Période {} du {} au {} (semaines {} à {})</big></b>",
                    index,
                    start_date.format("%d/%m/%Y").to_string(),
                    end_date.format("%d/%m/%Y").to_string(),
                    start_week,
                    end_week,
                )
            }
            None => {
                format!(
                    "<b><big>Période {} (semaines {} à {})</big></b>",
                    index, start_week, end_week,
                )
            }
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
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.generate_title_text(),
                    set_use_markup: true,
                },
                gtk::Button {
                    set_icon_name: "edit-symbolic",
                    add_css_class: "flat",
                    connect_clicked => EntryInput::EditClicked,
                },
                gtk::Button {
                    set_icon_name: "edit-cut-symbolic",
                    add_css_class: "flat",
                    connect_clicked => EntryInput::CutClicked,
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Button {
                    set_icon_name: "edit-delete",
                    add_css_class: "flat",
                    connect_clicked => EntryInput::DeleteClicked,
                },
            },
            /*gtk::ListBox {
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
            }*/
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            index: index.clone(),
            global_first_week: data.global_first_week,
            first_week_num: data.first_week_num,
            desc: data.desc,
            period_id: data.period_id,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.global_first_week = new_data.global_first_week;
                self.first_week_num = new_data.first_week_num;
                self.desc = new_data.desc;
                self.period_id = new_data.period_id;
            }
            EntryInput::EditClicked => {
                sender
                    .output(EntryOutput::EditClicked(self.period_id))
                    .unwrap();
            }
            EntryInput::CutClicked => {
                sender
                    .output(EntryOutput::CutClicked(self.period_id))
                    .unwrap();
            }
            EntryInput::DeleteClicked => {
                sender
                    .output(EntryOutput::DeleteClicked(self.period_id))
                    .unwrap();
            }
        }
    }
}
