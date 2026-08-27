use adw::prelude::{ActionRowExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
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
    /// Why no group list can be built for that pair — the group sizes the subject asks for
    /// cannot split the students registered on it. Filled in on `Show` by the parent, which
    /// blocks "Valider" while a *selected* subject carries one.
    pub error: Option<String>,
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
    /// Every subject of this period takes the given value.
    SetAll(bool),
}

#[derive(Debug)]
pub enum PeriodGroupOutput {
    /// (period index, subject index, new value)
    SubjectToggled(usize, usize, bool),
    /// (period index, new value for every subject of that period)
    SetAll(usize, bool),
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
            // The period name used to be the preferences group's own title, which left
            // nowhere to hang a button. It moves out onto a label line of its own, the
            // way the association panel already lays out a period header.
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 5,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.data.title,
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Button {
                    set_icon_name: "edit-select-all-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Activer toutes les listes de la période"),
                    connect_clicked => PeriodGroupInput::SetAll(true),
                },
                gtk::Button {
                    set_icon_name: "edit-clear-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Désactiver toutes les listes de la période"),
                    connect_clicked => PeriodGroupInput::SetAll(false),
                },
            },
            #[local_ref]
            subject_group -> adw::PreferencesGroup {
                set_hexpand: true,
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
        sender: FactorySender<Self>,
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
            // Like a single toggle: the parent owns the values and pushes them back
            // through `UpdateData`, so nothing is written into `self.data` here.
            PeriodGroupInput::SetAll(value) => {
                sender
                    .output(PeriodGroupOutput::SetAll(self.index.current_index(), value))
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

impl SubjectRow {
    /// The error message to display, if any. An unselected subject is not part of the request,
    /// so its error is moot and stays hidden.
    fn shown_error(&self) -> Option<&str> {
        if self.data.rebuild {
            self.data.error.as_deref()
        } else {
            None
        }
    }

    /// The message replaces the association subtitle: when a subject cannot be built at all,
    /// why it cannot matters more than which list it currently uses.
    fn shown_subtitle(&self) -> &str {
        self.shown_error().unwrap_or(&self.data.subtitle)
    }

    /// The css class carrying the error colors. GTK rejects an empty class name, so the
    /// "no error" case names a class no stylesheet defines rather than nothing.
    fn error_css_class(&self) -> &'static str {
        match self.shown_error() {
            Some(_) => "error",
            None => "no-error",
        }
    }
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
            // The error, when there is one, takes the subtitle line — right below the subject
            // name — in error colors, with the icon in front of the row.
            #[watch]
            set_subtitle: self.shown_subtitle(),
            set_subtitle_lines: 0,
            #[watch]
            remove_css_class: "error",
            #[watch]
            add_css_class: self.error_css_class(),
            add_prefix = &gtk::Image {
                set_icon_name: Some("dialog-error-symbolic"),
                #[watch]
                set_visible: self.shown_error().is_some(),
            },
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
