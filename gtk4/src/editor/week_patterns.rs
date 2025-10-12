use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_ops::WeekPatternsUpdateOp;

mod dialog;

#[derive(Debug)]
pub enum WeekPatternsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
        collomatique_state_colloscopes::week_patterns::WeekPatterns<
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ),
    EditWeekPatternClicked(collomatique_state_colloscopes::WeekPatternId),
    DeleteWeekPatternClicked(collomatique_state_colloscopes::WeekPatternId),
    AddWeekPatternClicked,
    WeekPatternEditResult(collomatique_state_colloscopes::week_patterns::WeekPattern),
}

#[derive(Debug)]
enum WeekPatternModificationReason {
    New,
    Edit(collomatique_state_colloscopes::WeekPatternId),
}

pub struct WeekPatterns {
    periods:
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns<
        collomatique_state_colloscopes::WeekPatternId,
    >,

    week_pattern_modification_reason: WeekPatternModificationReason,

    week_pattern_entries: FactoryVecDeque<Entry>,
    dialog: Controller<dialog::Dialog>,
}

#[relm4::component(pub)]
impl Component for WeekPatterns {
    type Input = WeekPatternsInput;
    type Output = WeekPatternsUpdateOp;
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
                #[local_ref]
                week_patterns_widget -> gtk::ListBox {
                    set_hexpand: true,
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    #[watch]
                    set_visible: !model.week_patterns.week_pattern_map.is_empty(),
                },
                gtk::Button {
                    set_margin_top: 10,
                    connect_clicked => WeekPatternsInput::AddWeekPatternClicked,
                    adw::ButtonContent {
                        set_icon_name: "edit-add",
                        set_label: "Ajouter un modèle de périodicité",
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
        let week_pattern_entries = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                EntryOutput::EditWeekPattern(id) => WeekPatternsInput::EditWeekPatternClicked(id),
                EntryOutput::DeleteWeekPattern(id) => {
                    WeekPatternsInput::DeleteWeekPatternClicked(id)
                }
            });
        let dialog = dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                dialog::DialogOutput::Accepted(week_pattern) => {
                    WeekPatternsInput::WeekPatternEditResult(week_pattern)
                }
            });
        let model = WeekPatterns {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns::default(),
            week_pattern_modification_reason: WeekPatternModificationReason::New,
            week_pattern_entries,
            dialog,
        };
        let week_patterns_widget = model.week_pattern_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            WeekPatternsInput::Update(new_periods, new_week_patterns) => {
                self.periods = new_periods;
                self.week_patterns = new_week_patterns;
                self.update_factory();
            }
            WeekPatternsInput::DeleteWeekPatternClicked(id) => {
                sender
                    .output(WeekPatternsUpdateOp::DeleteWeekPattern(id))
                    .unwrap();
            }
            WeekPatternsInput::EditWeekPatternClicked(id) => {
                self.week_pattern_modification_reason = WeekPatternModificationReason::Edit(id);
                let week_pattern_data = self
                    .week_patterns
                    .week_pattern_map
                    .get(&id)
                    .expect("Week pattern id should be valid on edit");
                self.dialog
                    .sender()
                    .send(dialog::DialogInput::Show(
                        self.periods.clone(),
                        week_pattern_data.clone(),
                    ))
                    .unwrap();
            }
            WeekPatternsInput::AddWeekPatternClicked => {
                self.week_pattern_modification_reason = WeekPatternModificationReason::New;
                self.dialog
                    .sender()
                    .send(dialog::DialogInput::Show(
                        self.periods.clone(),
                        collomatique_state_colloscopes::week_patterns::WeekPattern {
                            name: "Nouveau modèle".into(),
                            weeks: vec![],
                        },
                    ))
                    .unwrap();
            }
            WeekPatternsInput::WeekPatternEditResult(week_pattern_data) => {
                sender
                    .output(match self.week_pattern_modification_reason {
                        WeekPatternModificationReason::New => {
                            WeekPatternsUpdateOp::AddNewWeekPattern(week_pattern_data)
                        }
                        WeekPatternModificationReason::Edit(week_pattern_id) => {
                            WeekPatternsUpdateOp::UpdateWeekPattern(
                                week_pattern_id,
                                week_pattern_data,
                            )
                        }
                    })
                    .unwrap();
            }
        }
    }
}

impl WeekPatterns {
    fn update_factory(&mut self) {
        let mut week_patterns_vec: Vec<_> = self
            .week_patterns
            .week_pattern_map
            .iter()
            .map(|(id, week_pattern)| EntryData {
                id: id.clone(),
                name: week_pattern.name.clone(),
            })
            .collect();

        week_patterns_vec.sort_by(|a, b| a.name.cmp(&b.name));

        crate::tools::factories::update_vec_deque(
            &mut self.week_pattern_entries,
            week_patterns_vec.into_iter(),
            |data| EntryInput::UpdateData(data),
        );
    }
}

#[derive(Debug)]
struct EntryData {
    id: collomatique_state_colloscopes::WeekPatternId,
    name: String,
}

struct Entry {
    data: EntryData,
}

#[derive(Debug)]
enum EntryInput {
    UpdateData(EntryData),

    EditClicked,
    DeleteClicked,
}

#[derive(Debug)]
enum EntryOutput {
    EditWeekPattern(collomatique_state_colloscopes::WeekPatternId),
    DeleteWeekPattern(collomatique_state_colloscopes::WeekPatternId),
}

#[relm4::factory]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = EntryInput;
    type Output = EntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Horizontal,
            gtk::Button {
                set_icon_name: "edit-symbolic",
                add_css_class: "flat",
                connect_clicked => EntryInput::EditClicked,
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
                set_label: &self.data.name,
                set_size_request: (200, -1),
            },
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Button {
                set_icon_name: "edit-delete",
                add_css_class: "flat",
                connect_clicked => EntryInput::DeleteClicked,
            },
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let model = Self { data };

        model
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
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.data = new_data;
            }
            EntryInput::EditClicked => {
                sender
                    .output(EntryOutput::EditWeekPattern(self.data.id.clone()))
                    .unwrap();
            }
            EntryInput::DeleteClicked => {
                sender
                    .output(EntryOutput::DeleteWeekPattern(self.data.id.clone()))
                    .unwrap();
            }
        }
    }
}
