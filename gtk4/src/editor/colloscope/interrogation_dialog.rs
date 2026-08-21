use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    should_redraw: bool,
    assigned_groups: std::collections::BTreeSet<u32>,
    /// One row title per group of the edited list, in group order — built by
    /// the panel that holds the document on top of
    /// [collomatique_ui_text::rendering::render_group_name].
    group_titles: Vec<String>,

    group_entries: FactoryVecDeque<GroupEntry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(Vec<String>, std::collections::BTreeSet<u32>),
    Cancel,
    Accept,

    UpdateGroupStatus(u32, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(std::collections::BTreeSet<u32>),
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Modifier la colle"),
            set_default_size: (500, 700),
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[name(scrolled_window)]
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 5,
                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                    gtk::Box {
                        set_hexpand: true,
                        set_margin_all: 5,
                        set_spacing: 10,
                        set_orientation: gtk::Orientation::Vertical,
                        #[local_ref]
                        group_entries_widget -> adw::PreferencesGroup {
                            set_title: "",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.group_titles.is_empty(),
                        },
                        gtk::Label {
                            set_label: "Aucun groupe disponible pour cette colle",
                            #[watch]
                            set_visible: model.group_titles.is_empty(),
                        }
                    },
                },
            }
        },
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let group_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                GroupOutput::UpdateStatus(num, status) => {
                    DialogInput::UpdateGroupStatus(num, status)
                }
            });

        let model = Dialog {
            hidden: true,
            move_front: false,
            should_redraw: false,
            assigned_groups: std::collections::BTreeSet::new(),
            group_titles: Vec::new(),
            group_entries,
        };

        let group_entries_widget = model.group_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        self.move_front = false;
        match msg {
            DialogInput::Show(group_titles, assigned_groups) => {
                self.hidden = false;
                self.move_front = true;
                self.should_redraw = true;
                self.group_titles = group_titles;
                self.assigned_groups = assigned_groups;

                crate::tools::factories::update_vec_deque(
                    &mut self.group_entries,
                    self.group_titles
                        .iter()
                        .enumerate()
                        .map(|(num, title)| GroupData {
                            num: num as u32,
                            title: title.clone(),
                            status: self.assigned_groups.contains(&(num as u32)),
                        }),
                    GroupInput::UpdateData,
                );
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.assigned_groups.clone()))
                    .unwrap();
            }
            DialogInput::UpdateGroupStatus(group_num, new_status) => {
                if new_status {
                    self.assigned_groups.insert(group_num);
                } else {
                    self.assigned_groups.remove(&group_num);
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
        }
        if self.move_front {
            widgets.root_window.present();
        }
    }
}

#[derive(Debug, Clone)]
struct GroupData {
    num: u32,
    /// The row's title, « Groupe 3 » or « Groupe 3 : B2 ».
    title: String,
    status: bool,
}

#[derive(Debug)]
struct GroupEntry {
    data: GroupData,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum GroupInput {
    UpdateData(GroupData),

    UpdateStatus(bool),
}

#[derive(Debug)]
enum GroupOutput {
    UpdateStatus(u32, bool),
}

#[relm4::factory]
impl FactoryComponent for GroupEntry {
    type Init = GroupData;
    type Input = GroupInput;
    type Output = GroupOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::SwitchRow {
            set_hexpand: true,
            set_use_markup: false,
            #[watch]
            set_title: &self.data.title,
            #[track(self.should_redraw)]
            set_active: self.data.status,
            connect_active_notify[sender] => move |widget| {
                let status = widget.is_active();
                sender.input(
                    GroupInput::UpdateStatus(status)
                );
            },
        }
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            should_redraw: false,
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
        self.should_redraw = false;
        match msg {
            GroupInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            GroupInput::UpdateStatus(new_status) => {
                if self.data.status == new_status {
                    return;
                }
                self.data.status = new_status;
                sender
                    .output(GroupOutput::UpdateStatus(self.data.num, new_status))
                    .unwrap();
            }
        }
    }
}
