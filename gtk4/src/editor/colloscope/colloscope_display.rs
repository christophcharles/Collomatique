use crate::tools::dynamic_column_view::{DynamicColumnView, LabelColumn, RelmColumn};
use gtk::prelude::{OrientableExt, WidgetExt};
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
        collomatique_state_colloscopes::colloscopes::Colloscope,
    ),
}

#[derive(Debug)]
pub enum DisplayOutput {}

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
            colloscope: collomatique_state_colloscopes::colloscopes::Colloscope::default(),
            issue: None,
            column_view,
            current_items: vec![],
        };

        let column_view_widget = &model.column_view.view;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            DisplayInput::Update(
                periods,
                subjects,
                slots,
                teachers,
                students,
                group_lists,
                colloscope,
            ) => {
                self.periods = periods;
                self.subjects = subjects;
                self.slots = slots;
                self.teachers = teachers;
                self.students = students;
                self.group_lists = group_lists;
                self.colloscope = colloscope;

                self.update_display_issue();
                self.rebuild_columns();
                self.update_view_wrapper();
            }
        }
    }
}

impl Display {
    fn update_display_issue(&mut self) {
        self.issue = if self.periods.ordered_period_list.is_empty() {
            Some(DisplayIssue::NoPeriods)
        } else if self.periods.count_weeks() == 0 {
            Some(DisplayIssue::NoWeeks)
        } else if self.subjects.ordered_subject_list.is_empty() {
            Some(DisplayIssue::NoSubjects)
        } else if self
            .slots
            .subject_map
            .iter()
            .map(|(_id, subject_slots)| subject_slots.ordered_slots.len())
            .sum::<usize>()
            == 0
        {
            Some(DisplayIssue::NoSlots)
        } else {
            None
        };
    }

    fn rebuild_columns(&mut self) {
        self.column_view.clear_columns();
        self.column_view.append_column(SubjectColumn {});
        self.column_view.append_column(TeacherColumn {});
        self.column_view.append_column(DateTimeColumn {});

        let mut period_first_week = 0usize;
        for (period_id, period_desc) in &self.periods.ordered_period_list {
            for week_in_period in 0..period_desc.len() {
                self.column_view.append_column(WeekColumn {
                    period_id: *period_id,
                    period_first_week,
                    week_in_period,
                });
            }
            period_first_week += period_desc.len();
        }
    }

    fn update_view_wrapper(&mut self) {
        let mut new_items = vec![];

        for (subject_id, subject) in &self.subjects.ordered_subject_list {
            let Some(subject_slots) = self.slots.subject_map.get(subject_id) else {
                continue;
            };

            for (slot_id, slot) in &subject_slots.ordered_slots {
                let mut period_map = BTreeMap::new();

                for (period_id, period) in &self.periods.ordered_period_list {
                    let collo_period = self
                        .colloscope
                        .period_map
                        .get(period_id)
                        .expect("Period ID should be valid");

                    let Some(collo_slot) = collo_period.slot_map.get(slot_id) else {
                        period_map.insert(
                            *period_id,
                            SlotPeriodData {
                                slots: vec![None; period.len()],
                            },
                        );
                        continue;
                    };

                    let group_list_id = match self.group_lists.subjects_associations.get(period_id)
                    {
                        Some(period_associations) => period_associations.get(subject_id),
                        None => None,
                    };

                    let group_list = match group_list_id {
                        Some(id) => self.group_lists.group_list_map.get(id),
                        None => None,
                    };

                    period_map.insert(
                        *period_id,
                        SlotPeriodData {
                            slots: collo_slot
                                .interrogations
                                .iter()
                                .map(|interrogation_opt| match interrogation_opt {
                                    Some(interrogation) => Some(
                                        interrogation
                                            .assigned_groups
                                            .iter()
                                            .map(|num| {
                                                (
                                                    *num,
                                                    match group_list {
                                                        Some(list) => list
                                                            .prefilled_groups
                                                            .groups
                                                            .get(*num as usize)
                                                            .map(|group| group.name.clone())
                                                            .flatten(),
                                                        None => None,
                                                    },
                                                )
                                            })
                                            .collect(),
                                    ),
                                    None => None,
                                })
                                .collect(),
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
                .map(|data| SlotItem { data }),
        );
        self.current_items = new_items;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotPeriodData {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotItem {
    data: SlotItemData,
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
    period_first_week: usize,
    week_in_period: usize,
}

impl RelmColumn for WeekColumn {
    type Root = gtk::MenuButton;
    type Widgets = gtk::Label;
    type Item = SlotItem;

    fn column_name(&self) -> String {
        format!("{}", self.period_first_week + self.week_in_period + 1)
    }

    fn setup(&self, _item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let root = gtk::MenuButton::new();
        root.set_size_request(30, 30);
        let label = gtk::Label::new(None);
        root.set_child(Some(&label));
        label.set_halign(gtk::Align::Center);

        (root, label)
    }

    fn bind(&self, item: &mut Self::Item, label: &mut Self::Widgets, menu_button: &mut Self::Root) {
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
                label.set_label(&group_str.join(","));
                menu_button.set_visible(true);
            }
            None => {
                label.set_label("");
                menu_button.set_visible(false);
            }
        }
    }

    fn unbind(&self, _item: &mut Self::Item, _widgets: &mut Self::Widgets, _root: &mut Self::Root) {
    }
}
