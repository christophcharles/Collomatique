use crate::tools::dynamic_column_view::{DynamicColumnView, LabelColumn, RelmColumn};
use gtk::prelude::{ButtonExt, ObjectExt, OrientableExt, WidgetExt};
use libadwaita::glib::SignalHandlerId;
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender};

use std::collections::BTreeMap;

#[derive(Debug)]
pub enum DisplayInput {
    Update(
        collomatique_state_colloscopes::colloscope_params::Parameters,
        collomatique_state_colloscopes::colloscopes::Colloscope,
    ),

    InterrogationClicked(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::WeekId,
    ),
}

#[derive(Debug)]
pub enum DisplayOutput {
    InterrogationClicked(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::WeekId,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DisplayIssue {
    NoPeriods,
    NoWeeks,
    NoSubjects,
    NoSlots,
}

pub struct Display {
    params: collomatique_state_colloscopes::colloscope_params::Parameters,
    colloscope: collomatique_state_colloscopes::colloscopes::Colloscope,

    issue: Option<DisplayIssue>,
    column_view: DynamicColumnView<SlotItem, gtk::SingleSelection>,
    current_items: Vec<SlotItemData>,
}

#[relm4::component(pub)]
impl Component for Display {
    type Input = DisplayInput;
    type Output = DisplayOutput;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
                #[watch]
                set_visible: model.issue.is_none(),
                #[local_ref]
                column_view_widget -> gtk::ColumnView {
                    add_css_class: "frame",
                },
            },
            gtk::Box {
                set_hexpand: true,
                set_vexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                #[watch]
                set_visible: model.issue.is_some(),
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "<i>Aucune période à afficher</i>",
                    set_use_markup: true,
                    #[watch]
                    set_visible: model.issue == Some(DisplayIssue::NoPeriods),
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "<i>Aucune semaine de colle à afficher</i>",
                    set_use_markup: true,
                    #[watch]
                    set_visible: model.issue == Some(DisplayIssue::NoWeeks),
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "<i>Aucune matière à afficher</i>",
                    set_use_markup: true,
                    #[watch]
                    set_visible: model.issue == Some(DisplayIssue::NoSubjects),
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "<i>Aucun créneau de colles à afficher</i>",
                    set_use_markup: true,
                    #[watch]
                    set_visible: model.issue == Some(DisplayIssue::NoSlots),
                },
                gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                },
            },
        },
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let column_view = DynamicColumnView::new();

        let model = Display {
            params: collomatique_state_colloscopes::colloscope_params::Parameters::default(),
            colloscope: collomatique_state_colloscopes::colloscopes::Colloscope::default(),
            issue: None,
            column_view,
            current_items: vec![],
        };

        let column_view_widget = &model.column_view.view;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            DisplayInput::Update(params, colloscope) => {
                self.params = params;
                self.colloscope = colloscope;

                self.update_display_issue();
                self.clear_columns();
                self.update_view_wrapper(sender);
                self.build_columns();
            }
            DisplayInput::InterrogationClicked(slot_id, week_id) => {
                sender
                    .output(DisplayOutput::InterrogationClicked(slot_id, week_id))
                    .unwrap();
            }
        }
    }
}

impl Display {
    fn update_display_issue(&mut self) {
        self.issue = if self.params.periods.is_empty() {
            Some(DisplayIssue::NoPeriods)
        } else if self.params.count_weeks() == 0 {
            Some(DisplayIssue::NoWeeks)
        } else if self.params.subjects.ordered_subject_list.is_empty() {
            Some(DisplayIssue::NoSubjects)
        } else if self.params.slots.all_slots().next().is_none() {
            Some(DisplayIssue::NoSlots)
        } else {
            None
        };
    }

    fn clear_columns(&mut self) {
        self.column_view.clear_columns();
    }

    fn build_columns(&mut self) {
        self.column_view.append_column(SubjectColumn {});
        self.column_view.append_column(TeacherColumn {});
        self.column_view.append_column(DateTimeColumn {});

        let mut period_first_week = 0usize;
        let period_specs: Vec<_> = self
            .params
            .periods
            .period_ids()
            .map(|period_id| {
                (
                    period_id,
                    self.params
                        .weeks
                        .week_count_for_period(period_id)
                        .unwrap_or(0),
                )
            })
            .collect();
        for (period_id, period_len) in period_specs {
            for week_in_period in 0..period_len {
                let week_id = self
                    .params
                    .weeks
                    .week_id_at(period_id, week_in_period)
                    .expect("position within the period is valid");
                self.column_view.append_column(WeekColumn {
                    period_id,
                    week_id,
                    period_first_week,
                    week_in_period,
                });
            }
            period_first_week += period_len;
        }
    }

    fn update_view_wrapper(&mut self, sender: ComponentSender<Self>) {
        let mut new_items = vec![];

        for (subject_id, subject) in self.params.subjects.ordered_subject_list.iter() {
            let subject_id = &subject_id;
            let Some(subject_slots) = self.params.slots.slots_for_subject(*subject_id) else {
                continue;
            };

            for (slot_id, slot) in subject_slots {
                let mut period_map = BTreeMap::new();

                for period_id in self.params.periods.period_ids() {
                    let period_len = self
                        .params
                        .weeks
                        .week_count_for_period(period_id)
                        .unwrap_or(0);

                    let group_list_id = self
                        .params
                        .group_lists
                        .subjects_associations
                        .get(&(period_id, *subject_id));

                    let group_list = match group_list_id {
                        Some(id) => self.params.group_lists.group_list_map.get(id),
                        None => None,
                    };

                    let slots = (0..period_len)
                        .map(|week_in_period| {
                            let week_id = self
                                .params
                                .weeks
                                .week_id_at(period_id, week_in_period)
                                .expect("position within the period is valid");
                            // An impossible cell carries no button, matching the
                            // old dense `None`. The rule is the state layer's, so
                            // the grid cannot drift from the parent's click guard.
                            if !self.params.is_interrogation_possible(*slot_id, week_id) {
                                return None;
                            }
                            let assigned = self
                                .colloscope
                                .interrogation(*slot_id, week_id)
                                .cloned()
                                .unwrap_or_default();
                            Some(
                                assigned
                                    .iter()
                                    .map(|num| match group_list_id {
                                        Some(list_id) => collomatique_ui_text::rendering::render_group(
                                            &self.params.group_lists,
                                            *list_id,
                                            *num,
                                        )
                                        .expect(
                                            "a cell's groups are bounded by the list associated \
                                             at its coordinate",
                                        ),
                                        // No association means no bound, so no
                                        // name to read: the raw number is all
                                        // there is to show.
                                        None => (*num + 1).to_string(),
                                    })
                                    .collect(),
                            )
                        })
                        .collect();

                    period_map.insert(
                        period_id,
                        SlotPeriodData {
                            has_group_list: group_list.is_some(),
                            slots,
                        },
                    );
                }

                let teacher_desc = &self
                    .params
                    .teachers
                    .teacher_map
                    .get(&slot.teacher_id)
                    .expect("Teacher ID should be valid")
                    .desc;
                new_items.push(SlotItemData {
                    subject: subject.parameters.name.clone(),
                    slot_id: *slot_id,
                    teacher: format!("{} {}", teacher_desc.firstname, teacher_desc.surname),
                    date_time: slot.start_time.clone(),
                    period_map,
                })
            }
        }

        let Some((first_modified, to_remove_count, to_add_count)) =
            crate::tools::dynamic_column_view::compute_update_data(&self.current_items, &new_items)
        else {
            return;
        };

        self.column_view.splice(
            first_modified as u32,
            to_remove_count as u32,
            new_items
                .clone()
                .into_iter()
                .skip(first_modified)
                .take(to_add_count)
                .map(|data| SlotItem {
                    data,
                    sender: sender.clone(),
                    handler_ids: BTreeMap::new(),
                }),
        );
        self.current_items = new_items;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotPeriodData {
    has_group_list: bool,
    /// One entry per week of the period: `None` for an impossible cell,
    /// otherwise the cell's groups already rendered, in group order.
    slots: Vec<Option<Vec<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotItemData {
    subject: String,
    slot_id: collomatique_state_colloscopes::SlotId,
    teacher: String,
    date_time: collomatique_time::SlotStart,
    period_map: BTreeMap<collomatique_state_colloscopes::PeriodId, SlotPeriodData>,
}

struct SlotItem {
    data: SlotItemData,
    sender: ComponentSender<Display>,
    handler_ids: BTreeMap<(collomatique_state_colloscopes::PeriodId, usize), SignalHandlerId>,
}

#[derive(Debug, Clone)]
struct SubjectColumn {}

impl LabelColumn for SubjectColumn {
    type Item = SlotItem;
    type Value = String;

    fn column_name(&self) -> String {
        "Matière".into()
    }
    fn sort_enabled(&self) -> bool {
        false
    }
    fn resize_enabled(&self) -> bool {
        true
    }

    fn get_cell_value(&self, item: &Self::Item) -> Self::Value {
        item.data.subject.clone()
    }
}

#[derive(Debug, Clone)]
struct TeacherColumn {}

impl LabelColumn for TeacherColumn {
    type Item = SlotItem;
    type Value = String;

    fn column_name(&self) -> String {
        "Colleur".into()
    }
    fn sort_enabled(&self) -> bool {
        false
    }
    fn resize_enabled(&self) -> bool {
        true
    }

    fn get_cell_value(&self, item: &Self::Item) -> Self::Value {
        item.data.teacher.clone()
    }
}

#[derive(Debug, Clone)]
struct DateTimeColumn {}

impl LabelColumn for DateTimeColumn {
    type Item = SlotItem;
    type Value = String;

    fn column_name(&self) -> String {
        "Horaire".into()
    }
    fn sort_enabled(&self) -> bool {
        false
    }
    fn resize_enabled(&self) -> bool {
        true
    }

    fn get_cell_value(&self, item: &Self::Item) -> Self::Value {
        item.data.date_time.capitalize()
    }
}

#[derive(Debug, Clone)]
struct WeekColumn {
    period_id: collomatique_state_colloscopes::PeriodId,
    week_id: collomatique_state_colloscopes::WeekId,
    period_first_week: usize,
    week_in_period: usize,
}

impl RelmColumn for WeekColumn {
    type Root = gtk::Button;
    type Widgets = ();
    type Item = SlotItem;

    fn column_name(&self) -> String {
        format!("{}", self.period_first_week + self.week_in_period + 1)
    }

    fn setup(&self, _item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            root = gtk::Button {
                set_size_request: (50,30),
            },
        }

        (root, ())
    }

    fn bind(&self, item: &mut Self::Item, _widgets: &mut Self::Widgets, root: &mut Self::Root) {
        let period_slots = item
            .data
            .period_map
            .get(&self.period_id)
            .expect("Period ID should be valid");
        let groups_opt = period_slots
            .slots
            .get(self.week_in_period)
            .expect("Index for week should be valid");

        match groups_opt {
            Some(groups) => {
                root.set_label(&groups.join(","));
                root.set_visible(true);
                root.set_sensitive(period_slots.has_group_list);
                if period_slots.has_group_list {
                    root.set_tooltip_text(Some("Modifier la colle"));
                } else {
                    root.set_tooltip_text(Some("Aucune liste de groupes définie pour cette colle"));
                }

                let sender = item.sender.clone();
                let slot_id = item.data.slot_id;
                let period_id = self.period_id;
                let week_id = self.week_id;
                let week_in_period = self.week_in_period;
                item.handler_ids.insert(
                    (period_id, week_in_period),
                    root.connect_clicked(move |_widget| {
                        sender.input(DisplayInput::InterrogationClicked(slot_id, week_id));
                    }),
                );
            }
            None => {
                root.set_label("");
                root.set_visible(false);
            }
        }
    }

    fn unbind(&self, item: &mut Self::Item, _widgets: &mut Self::Widgets, root: &mut Self::Root) {
        if let Some(id) = item
            .handler_ids
            .remove(&(self.period_id, self.week_in_period))
        {
            root.disconnect(id);
        }
    }
}
