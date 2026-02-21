use adw::prelude::{ComboRowExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::export_config;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    config: export_config::PerGroupListConfig,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(export_config::PerGroupListConfig),
    Cancel,
    Accept,

    UpdateOrientation(export_config::PageOrientation),
    UpdateShowEmails(bool),
    UpdateShowTel(bool),
    UpdateCenterVertically(bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(export_config::PerGroupListConfig),
}

impl Dialog {
    fn generate_orientation_model() -> gtk::StringList {
        gtk::StringList::new(&["Portrait", "Paysage"])
    }

    fn mandatory_orientation_to_selected(orientation: &export_config::PageOrientation) -> u32 {
        match orientation {
            export_config::PageOrientation::Portrait => 0,
            export_config::PageOrientation::Landscape => 1,
        }
    }

    fn selected_to_mandatory_orientation(selected: u32) -> export_config::PageOrientation {
        match selected {
            0 => export_config::PageOrientation::Portrait,
            1 => export_config::PageOrientation::Landscape,
            _ => panic!("Invalid selection for mandatory orientation"),
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Configuration : par liste de groupes"),
            set_default_size: (500, 400),
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
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                    gtk::Box {
                        set_hexpand: true,
                        set_margin_all: 5,
                        set_spacing: 10,
                        set_orientation: gtk::Orientation::Vertical,
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,

                            #[name(orientation_combo)]
                            adw::ComboRow {
                                set_title: "Orientation de la page",
                                set_model: Some(&Self::generate_orientation_model()),
                                #[track(Self::mandatory_orientation_to_selected(&model.config.orientation) != orientation_combo.selected())]
                                set_selected: Self::mandatory_orientation_to_selected(&model.config.orientation),
                                connect_selected_notify[sender] => move |widget| {
                                    let selected = widget.selected();
                                    sender.input(DialogInput::UpdateOrientation(
                                        Self::selected_to_mandatory_orientation(selected)
                                    ));
                                },
                            },

                            #[name(show_emails_switch)]
                            adw::SwitchRow {
                                set_title: "Afficher les emails",
                                #[track(model.config.show_emails != show_emails_switch.is_active())]
                                set_active: model.config.show_emails,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateShowEmails(widget.is_active()));
                                },
                            },

                            #[name(show_tel_switch)]
                            adw::SwitchRow {
                                set_title: "Afficher les téléphones",
                                #[track(model.config.show_tel != show_tel_switch.is_active())]
                                set_active: model.config.show_tel,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateShowTel(widget.is_active()));
                                },
                            },

                            #[name(center_vertically_switch)]
                            adw::SwitchRow {
                                set_title: "Centrer verticalement",
                                #[track(model.config.center_vertically != center_vertically_switch.is_active())]
                                set_active: model.config.center_vertically,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateCenterVertically(widget.is_active()));
                                },
                            },
                        },
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
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            config: export_config::PerGroupListConfig::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(config) => {
                self.config = config;
                self.hidden = false;
                self.should_redraw = true;
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.config.clone()))
                    .unwrap();
            }
            DialogInput::UpdateOrientation(orientation) => {
                if self.config.orientation == orientation {
                    return;
                }
                self.config.orientation = orientation;
            }
            DialogInput::UpdateShowEmails(show_emails) => {
                if self.config.show_emails == show_emails {
                    return;
                }
                self.config.show_emails = show_emails;
            }
            DialogInput::UpdateShowTel(show_tel) => {
                if self.config.show_tel == show_tel {
                    return;
                }
                self.config.show_tel = show_tel;
            }
            DialogInput::UpdateCenterVertically(center_vertically) => {
                if self.config.center_vertically == center_vertically {
                    return;
                }
                self.config.center_vertically = center_vertically;
            }
        }
    }
}
