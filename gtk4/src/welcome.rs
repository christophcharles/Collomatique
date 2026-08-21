use adw::prelude::{ActionRowExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, ListBoxRowExt, OrientableExt, WidgetExt};
use relm4::RelmWidgetExt;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{ComponentParts, ComponentSender, FactorySender, SimpleComponent};
use relm4::{adw, gtk};
use std::path::PathBuf;

use collomatique_settings::recent_files::{self, Entry};

pub struct WelcomePanel {
    recents: FactoryVecDeque<RecentRow>,
}

/// What the welcome screen asks the application for
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WelcomeMessage {
    OpenNewColloscope,
    OpenExistingColloscope,
    /// A file from the recent list, by the path to open it with -- which under
    /// a sandbox is not the path shown in the row.
    OpenRecentColloscope(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WelcomeInput {
    OpenNewColloscope,
    OpenExistingColloscope,
    OpenRecent(PathBuf),
    /// Re-reads the recent files from disk. Worth sending whenever the welcome
    /// screen comes back into view: another running instance may have opened
    /// something meanwhile, and so may this one.
    Refresh,
}

relm4::new_action_group!(WelcomeActionGroup, "welcome");

relm4::new_stateless_action!(NewAction, WelcomeActionGroup, "new");
relm4::new_stateless_action!(OpenAction, WelcomeActionGroup, "open");
relm4::new_stateless_action!(AboutAction, WelcomeActionGroup, "about");

#[relm4::component(pub)]
impl SimpleComponent for WelcomePanel {
    type Input = WelcomeInput;
    type Output = WelcomeMessage;
    type Init = ();

    view! {
        #[root]
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                pack_end = &gtk::MenuButton {
                    set_icon_name: "open-menu-symbolic",
                    set_menu_model: Some(&main_menu),
                },
                pack_end = &gtk::Image {
                    set_icon_name: Some("dialog-warning-symbolic"),
                    set_tooltip: &super::in_dev_tooltip(),
                    set_visible: super::in_dev_shown(),
                },
            },
            #[wrap(Some)]
            set_content = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_hexpand: true,
                set_vexpand: true,

                gtk::Button::with_label("Commencer un nouveau colloscope") {
                    set_margin_all: 5,
                    add_css_class: "suggested-action",
                    connect_clicked => WelcomeInput::OpenNewColloscope,
                },
                gtk::Button::with_label("Ouvrir un colloscope existant") {
                    set_margin_all: 5,
                    add_css_class: "suggested-action",
                    connect_clicked => WelcomeInput::OpenExistingColloscope,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 30,
                    set_spacing: 10,
                    // Nothing opened yet, nothing to show: a first run gets the
                    // two buttons alone, as before.
                    #[watch]
                    set_visible: !model.recents.is_empty(),

                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_label: "Fichiers récents",
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                    },
                    #[local_ref]
                    recents_widget -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        set_width_request: 400,
                    },
                },
            },
        },
    }

    menu! {
        main_menu: {
            section! {
                "Nouveau" => super::NewAction,
                "Ouvrir" => super::OpenAction,
            },
            section! {
                "À propos" => super::AboutAction
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let recents = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                RecentRowOutput::Open(path) => WelcomeInput::OpenRecent(path),
            });

        let model = WelcomePanel { recents };
        let recents_widget = model.recents.widget();
        let widgets = view_output!();

        // The list is read from disk rather than passed in: nothing above this
        // component knows or should know where it is kept.
        sender.input(WelcomeInput::Refresh);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            WelcomeInput::OpenNewColloscope => {
                sender.output(WelcomeMessage::OpenNewColloscope).unwrap();
            }
            WelcomeInput::OpenExistingColloscope => {
                sender
                    .output(WelcomeMessage::OpenExistingColloscope)
                    .unwrap();
            }
            WelcomeInput::OpenRecent(path) => {
                sender
                    .output(WelcomeMessage::OpenRecentColloscope(path))
                    .unwrap();
            }
            WelcomeInput::Refresh => {
                // Rebuilt wholesale rather than diffed: five rows, and each one
                // is re-checked against the filesystem on the way in, which is
                // the point of refreshing at all.
                let mut guard = self.recents.guard();
                guard.clear();
                for entry in recent_files::list() {
                    guard.push_back(entry);
                }
            }
        }
    }
}

/// One remembered file in the list
///
/// A row that cannot be opened -- the file was moved, deleted, or lives on a
/// drive that is not plugged in right now -- is shown all the same, greyed and
/// saying so. Forgetting it would be worse: a colloscope on a USB key is not
/// gone, and the user is the one who knows that.
struct RecentRow {
    entry: Entry,
    available: bool,
}

#[derive(Debug)]
enum RecentRowOutput {
    /// The access path of the file to open
    Open(PathBuf),
}

#[relm4::factory]
impl FactoryComponent for RecentRow {
    type Init = Entry;
    type Input = ();
    type Output = RecentRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        adw::ActionRow {
            // A file name is a file name, not markup: one containing `&` or `<`
            // would otherwise come out mangled or empty.
            set_use_markup: false,
            set_title: &self.title(),
            set_subtitle: &self.subtitle(),
            set_activatable: self.available,
            set_sensitive: self.available,
            connect_activated[sender, path = self.entry.access.clone()] => move |_| {
                sender.output(RecentRowOutput::Open(path.clone())).unwrap();
            },
        }
    }

    fn init_model(entry: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        // Asked once, when the row is built. That is not a cache to be
        // invalidated: the whole list is rebuilt on every refresh, so the
        // answer is never older than the last time the screen came into view.
        let available = std::fs::metadata(&entry.access).is_ok();

        Self { entry, available }
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

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

impl RecentRow {
    /// The file name alone, which is what tells two colloscopes apart at a
    /// glance. A path with no file name to speak of falls back to the whole of
    /// it rather than to an empty row.
    fn title(&self) -> String {
        match self.entry.display.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => self.entry.display.to_string_lossy().to_string(),
        }
    }

    /// Where the file is -- or why it cannot be opened, which the user needs
    /// more than the folder at that point.
    fn subtitle(&self) -> String {
        if !self.available {
            return String::from("Fichier déplacé ou supprimé");
        }
        match self.entry.display.parent() {
            Some(dir) => dir.to_string_lossy().to_string(),
            None => String::new(),
        }
    }
}
