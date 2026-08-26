use adw::prelude::{ExpanderRowExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ListBoxRowExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{adw, gtk};

/// One generated list, as the naming dialog shows it once the greedy has answered: a collapsed
/// [`adw::ExpanderRow`] the user can open to read the groups. Purely a display — the list name
/// is edited in the naming rows above, not here, so the title stays the coverage label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    /// e.g. "Maths et Physique (périodes 1 et 2)".
    pub title: String,
    /// e.g. "4 groupes, 13 élèves".
    pub subtitle: String,
    /// One entry per group, in the list's own group order.
    pub groups: Vec<GroupData>,
}

/// One group of one list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupData {
    /// e.g. "Groupe 2".
    pub title: String,
    /// The members' names, already joined for display.
    pub students: String,
}

pub struct ListRow {
    data: Data,
    group_rows: FactoryVecDeque<GroupRow>,
}

#[derive(Debug)]
pub enum ListRowInput {
    UpdateData(Data),
}

#[relm4::factory(pub)]
impl FactoryComponent for ListRow {
    type Init = Data;
    type Input = ListRowInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::ExpanderRow {
            set_hexpand: true,
            #[watch]
            set_title: &self.data.title,
            #[watch]
            set_subtitle: &self.data.subtitle,
        }
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let list_box = gtk::ListBox::default();
        list_box.set_selection_mode(gtk::SelectionMode::None);
        let group_rows = FactoryVecDeque::builder().launch(list_box).detach();

        let mut model = Self { data, group_rows };
        model.update_group_rows();
        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        // The group count varies from list to list, so the rows are held by a nested factory and
        // attached here: the view macro below can only spell a fixed set of children.
        root.add_row(self.group_rows.widget());

        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            ListRowInput::UpdateData(data) => {
                self.data = data;
                self.update_group_rows();
            }
        }
    }
}

impl ListRow {
    fn update_group_rows(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.group_rows,
            self.data.groups.iter().cloned(),
            GroupRowInput::UpdateData,
        );
    }
}

/// One group row inside an expanded list. An implementation detail of the list row above — it
/// has no user outside this file, so it stays private.
struct GroupRow {
    data: GroupData,
}

#[derive(Debug)]
enum GroupRowInput {
    UpdateData(GroupData),
}

#[relm4::factory]
impl FactoryComponent for GroupRow {
    type Init = GroupData;
    type Input = GroupRowInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_start: 15,
            set_margin_end: 15,
            set_margin_top: 8,
            set_margin_bottom: 8,
            set_spacing: 2,
            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: &self.data.title,
                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
            },
            gtk::Label {
                set_hexpand: true,
                set_wrap: true,
                set_xalign: 0.,
                #[watch]
                set_label: &self.data.students,
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
        returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        // A group row is read, never picked: the wrapping list-box row must not react to clicks.
        returned_widget.set_activatable(false);

        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            GroupRowInput::UpdateData(data) => {
                self.data = data;
            }
        }
    }
}
