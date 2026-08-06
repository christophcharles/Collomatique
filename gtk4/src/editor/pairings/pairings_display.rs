use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::RelmWidgetExt;
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};

use crate::tools::messages::MessageIcon;

#[derive(Debug, Clone)]
pub struct EntryData {
    pub rule_id: collomatique_state_colloscopes::PairingRuleId,
    pub rule: collomatique_state_colloscopes::pairings::PairingRule,
    /// The rule as [collomatique_ops::rendering::render_pairing_rule] names it.
    /// Softness is not part of it — this row appends « (souple) » itself.
    pub summary: String,
    pub periods: collomatique_state_colloscopes::periods::Periods,
}

#[derive(Debug)]
pub struct Entry {
    data: EntryData,
    messages: FactoryVecDeque<MessageIcon>,
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
        let soft_text = if self.data.rule.soft() {
            " (souple)"
        } else {
            ""
        };
        format!("{}{}", self.data.summary, soft_text)
    }

    fn generate_excluded_periods_info(&self) -> String {
        let mut excluded_period_list: Vec<_> = self
            .data
            .rule
            .excluded_periods()
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
                "Désactivée sur les périodes {}",
                collomatique_ops::rendering::join_french(&excluded_period_list)
            ),
        }
    }

    /// Refills the icon strip, so it always describes the rule currently shown.
    ///
    /// The remarks are the ones the edition dialog spells out in full; here they
    /// are only icons, with the text as tooltip. A recorded rule always names
    /// two distinct subjects, hence `subjects_are_same = false` — the error
    /// variant cannot fire on a row.
    fn update_messages(&mut self) {
        let messages: Vec<_> = super::rule_messages(super::rule_shape(&self.data.rule), false)
            .into_iter()
            .map(|message| (message.severity(), message.text().to_string()))
            .collect();

        crate::tools::factories::refill_vec_deque(&mut self.messages, messages);
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
                set_icon_name: "document-edit-symbolic",
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
                set_visible: !self.data.rule.excluded_periods().is_empty(),
            },
            #[local_ref]
            messages_box -> gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 5,
                set_margin_end: 5,
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
        let mut model = Self {
            data,
            messages: FactoryVecDeque::builder()
                .launch(gtk::Box::default())
                .detach(),
        };
        model.update_messages();

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let messages_box = self.messages.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.data = new_data;
                self.update_messages();
            }
        }
    }
}
