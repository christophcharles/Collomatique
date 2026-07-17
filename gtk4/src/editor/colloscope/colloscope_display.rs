use crate::tools::dynamic_column_view::{DynamicColumnView, LabelColumn, RelmColumn};
use gtk::prelude::{ButtonExt, ObjectExt, OrientableExt, WidgetExt};
use libadwaita::glib::SignalHandlerId;
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender};

use std::collections::BTreeMap;

#[derive(Debug)]
pub enum DisplayInput {
    Update(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::slots::Slots,
        collomatique_state_colloscopes::teachers::Teachers,
        collomatique_state_colloscopes::students::Students,
        collomatique_state_colloscopes::group_lists::GroupLists,
        collomatique_state_colloscopes::week_patterns::WeekPatterns,
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
    periods: collomatique_state_colloscopes::periods::Periods,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    slots: collomatique_state_colloscopes::slots::Slots,
    teachers: collomatique_state_colloscopes::teachers::Teachers,
    students: collomatique_state_colloscopes::students::Students,
    group_lists: collomatique_state_colloscopes::group_lists::GroupLists,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns,
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
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            slots: collomatique_state_colloscopes::slots::Slots::default(),
            teachers: collomatique_state_colloscopes::teachers::Teachers::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            group_lists: collomatique_state_colloscopes::group_lists::GroupLists::default(),
            week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns::default(),
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
            DisplayInput::Update(
                periods,
                subjects,
                slots,
                teachers,
                students,
                group_lists,
                week_patterns,
                colloscope,
            ) => {
                self.periods = periods;
                self.subjects = subjects;
                self.slots = slots;
                self.teachers = teachers;
                self.students = students;
                self.group_lists = group_lists;
                self.week_patterns = week_patterns;
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
        self.issue = if self.periods.is_empty() {
            Some(DisplayIssue::NoPeriods)
        } else if self.periods.count_weeks() == 0 {
            Some(DisplayIssue::NoWeeks)
        } else if self.subjects.ordered_subject_list.is_empty() {
            Some(DisplayIssue::NoSubjects)
        } else if self.slots.all_slots().next().is_none() {
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
            .periods
            .period_ids()
            .map(|period_id| {
                (
                    period_id,
                    self.periods
                        .week_count_of(period_id)
                        .expect("period id from period_ids is valid"),
                )
            })
            .collect();
        for (period_id, period_len) in period_specs {
            for week_in_period in 0..period_len {
                let week_id = self
                    .periods
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

        for (subject_id, subject) in self.subjects.ordered_subject_list.iter() {
            let subject_id = &subject_id;
            let Some(subject_slots) = self.slots.slots_for_subject(*subject_id) else {
                continue;
            };

            for (slot_id, slot) in subject_slots {
                let mut period_map = BTreeMap::new();

                for period_id in self.periods.period_ids() {
                    let period_len = self
                        .periods
                        .week_count_of(period_id)
                        .expect("period id from period_ids is valid");

                    // The slot runs in this period iff its subject does — not
                    // excluded and has interrogations. Otherwise every cell is
                    // impossible (the old dense skeleton had no slot entry here).
                    let subject_runs = !subject.excluded_periods.contains(&period_id)
                        && subject.parameters.interrogation_parameters.is_some();
                    if !subject_runs {
                        period_map.insert(
                            period_id,
                            SlotPeriodData {
                                has_group_list: false,
                                slots: vec![None; period_len],
                            },
                        );
                        continue;
                    }

                    let group_list_id = self
                        .group_lists
                        .subjects_associations
                        .get(&(period_id, *subject_id));

                    let group_list = match group_list_id {
                        Some(id) => self.group_lists.group_list_map.get(id),
                        None => None,
                    };

                    let slots = (0..period_len)
                        .map(|week_in_period| {
                            let week_id = self
                                .periods
                                .week_id_at(period_id, week_in_period)
                                .expect("position within the period is valid");
                            // Impossible week (pattern-excluded or no interrogations)
                            // → no cell, matching the old dense `None`.
                            if !self.week_patterns.is_week_active(
                                &self.periods,
                                week_id,
                                slot.week_pattern,
                            ) {
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
                                    .map(|num| {
                                        (
                                            *num,
                                            match group_list {
                                                Some(list) => list
                                                    .params
                                                    .group_names
                                                    .get(*num as usize)
                                                    .cloned()
                                                    .flatten(),
                                                None => None,
                                            },
                                        )
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
    slots: Vec<Option<BTreeMap<u32, Option<non_empty_string::NonEmptyString>>>>,
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
                let group_str: Vec<_> = groups
                    .iter()
                    .map(|(num, name_opt)| match name_opt {
                        Some(name) => name.clone().into_inner(),
                        None => (*num + 1).to_string(),
                    })
                    .collect();
                root.set_label(&group_str.join(","));
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
