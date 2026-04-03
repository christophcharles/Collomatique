use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::RelmWidgetExt;
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};

#[derive(Debug, Clone)]
pub struct EntryData {
    pub rule_id: collomatique_state_colloscopes::PairingRuleId,
    pub rule: collomatique_state_colloscopes::pairings::PairingRule,
    pub subjects: collomatique_state_colloscopes::subjects::Subjects,
    pub periods: collomatique_state_colloscopes::periods::Periods,
}

#[derive(Debug)]
pub struct Entry {
    data: EntryData,
}

#[derive(Debug, Clone)]
pub enum EntryInput {
    UpdateData(EntryData),
}

#[derive(Debug)]
pub enum EntryOutput {
    DeletePairing(collomatique_state_colloscopes::PairingRuleId),
    EditPairing(collomatique_state_colloscopes::PairingRuleId),
}

impl Entry {
    fn generate_summary(&self) -> String {
        let ant_name = self.subject_name(self.data.rule.antecedent.subject_id);
        let con_name = self.subject_name(self.data.rule.consequent.subject_id);
        let ant_cond = if self.data.rule.antecedent.should_have {
            "Avoir"
        } else {
            "Ne pas avoir"
        };
        let con_cond = if self.data.rule.consequent.should_have {
            "Avoir"
        } else {
            "Ne pas avoir"
        };
        let soft_text = if self.data.rule.soft { " (souple)" } else { "" };
        format!(
            "{} {} \u{27F9} {} {}{}",
            ant_cond, ant_name, con_cond, con_name, soft_text
        )
    }

    fn subject_name(&self, subject_id: collomatique_state_colloscopes::SubjectId) -> String {
        self.data
            .subjects
            .find_subject(subject_id)
            .map(|s| s.parameters.name.clone())
            .unwrap_or_else(|| "???".into())
    }

    fn generate_excluded_periods_info(&self) -> String {
        let mut excluded_period_list: Vec<_> = self
            .data
            .rule
            .excluded_periods
            .iter()
            .map(|period_id| {
                self.data
                    .periods
                    .find_period_position(*period_id)
                    .expect("Period referenced by pairing rule should be valid")
                    + 1
            })
            .collect();

        excluded_period_list.sort();

        let excluded_period_list: Vec<_> = excluded_period_list
            .into_iter()
            .map(|x| x.to_string())
            .collect();

        match excluded_period_list.len() {
            0 => String::new(),
            1 => format!("Désactivée sur la période {}", excluded_period_list[0]),
            _ => format!(
                "Désactivée sur les périodes {} et {}",
                excluded_period_list[..excluded_period_list.len() - 1].join(", "),
                excluded_period_list.last().unwrap()
            ),
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = EntryInput;
    type Output = EntryOutput;
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
                connect_clicked[sender, rule_id = self.data.rule_id] => move |_| {
                    sender
                        .output(EntryOutput::EditPairing(rule_id))
                        .unwrap();
                },
                set_tooltip_text: Some("Modifier l'appariement"),
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
                set_label: &self.generate_summary(),
            },
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Label {
                set_halign: gtk::Align::End,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_excluded_periods_info(),
                set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
                #[watch]
                set_visible: !self.data.rule.excluded_periods.is_empty(),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Button {
                set_icon_name: "edit-delete-symbolic",
                add_css_class: "flat",
                connect_clicked[sender, rule_id = self.data.rule_id] => move |_| {
                    sender
                        .output(EntryOutput::DeletePairing(rule_id))
                        .unwrap();
                },
                set_tooltip_text: Some("Supprimer l'appariement"),
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

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.data = new_data;
            }
        }
    }
}
