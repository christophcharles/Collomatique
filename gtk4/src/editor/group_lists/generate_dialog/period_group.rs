use adw::prelude::{ActionRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::{PeriodId, SubjectId};

/// One subject row of one period: whether a new group list should be built for that
/// (period, subject) pair. The subtitle spells out the current association, which is also what
/// the default value is derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectData {
    pub subject_id: SubjectId,
    pub title: String,
    pub subtitle: String,
    pub rebuild: bool,
}

/// One period group of the left panel. Carries its own `PeriodId` so the parent reads the
/// request back without re-deriving any ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    pub period_id: PeriodId,
    pub title: String,
    pub subjects: Vec<SubjectData>,
}

pub struct PeriodGroup {
    data: Data,
    index: DynamicIndex,
    subject_rows: FactoryVecDeque<SubjectRow>,
}

#[derive(Debug)]
pub enum PeriodGroupInput {
    UpdateData(Data),
    /// (subject index within this period, new value) — relayed from a `SubjectRow`.
    SubjectToggled(usize, bool),
}

#[derive(Debug)]
pub enum PeriodGroupOutput {
    /// (period index, subject index, new value)
    SubjectToggled(usize, usize, bool),
}

#[relm4::factory(pub)]
impl FactoryComponent for PeriodGroup {
    type Init = Data;
    type Input = PeriodGroupInput;
    type Output = PeriodGroupOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            #[local_ref]
            subject_group -> adw::PreferencesGroup {
                set_hexpand: true,
                #[watch]
                set_title: &self.data.title,
            },
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let subject_rows = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                SubjectRowOutput::Toggled(subject_index, value) => {
                    PeriodGroupInput::SubjectToggled(subject_index, value)
                }
            });

        let mut model = Self {
            data,
            index: index.clone(),
            subject_rows,
        };
        model.update_subject_rows();
        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let subject_group = self.subject_rows.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            PeriodGroupInput::UpdateData(data) => {
                self.data = data;
                self.update_subject_rows();
            }
            PeriodGroupInput::SubjectToggled(subject_index, value) => {
                sender
                    .output(PeriodGroupOutput::SubjectToggled(
                        self.index.current_index(),
                        subject_index,
                        value,
                    ))
                    .unwrap();
            }
        }
    }
}

impl PeriodGroup {
    fn update_subject_rows(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.subject_rows,
            self.data.subjects.iter().cloned(),
            SubjectRowInput::UpdateData,
        );
    }
}

/// One subject switch. An implementation detail of the period group above — it has no user
/// outside this file, so it stays private.
struct SubjectRow {
    data: SubjectData,
    index: DynamicIndex,
}

#[derive(Debug)]
enum SubjectRowInput {
    UpdateData(SubjectData),
    Toggled(bool),
}

#[derive(Debug)]
enum SubjectRowOutput {
    Toggled(usize, bool),
}

#[relm4::factory]
impl FactoryComponent for SubjectRow {
    type Init = SubjectData;
    type Input = SubjectRowInput;
    type Output = SubjectRowOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        #[name(switch_row)]
        adw::SwitchRow {
            #[watch]
            set_title: &self.data.title,
            #[watch]
            set_subtitle: &self.data.subtitle,
            #[track(switch_row.is_active() != self.data.rebuild)]
            #[block_signal(rebuild_handler)]
            set_active: self.data.rebuild,
            connect_active_notify[sender] => move |widget| {
                sender.input(SubjectRowInput::Toggled(widget.is_active()));
            } @rebuild_handler,
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            index: index.clone(),
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
        match msg {
            SubjectRowInput::UpdateData(data) => {
                self.data = data;
            }
            SubjectRowInput::Toggled(value) => {
                sender
                    .output(SubjectRowOutput::Toggled(self.index.current_index(), value))
                    .unwrap();
            }
        }
    }
}
