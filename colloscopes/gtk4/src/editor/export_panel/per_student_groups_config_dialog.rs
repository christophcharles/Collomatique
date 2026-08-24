use adw::prelude::{ComboRowExt, EditableExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::export_config;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    should_redraw: bool,
    sheet_display_name: String,
    config: export_config::PerStudentGroupsConfig,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(export_config::PerStudentGroupsConfig),
    Cancel,
    Accept,

    UpdateSheetName(String),
    UpdateOrientation(Option<export_config::PageOrientation>),
    UpdateShowEmails(bool),
    UpdateShowTel(bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(export_config::PerStudentGroupsConfig),
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

impl Dialog {
    fn generate_orientation_model() -> gtk::StringList {
        gtk::StringList::new(&["Automatique", "Portrait", "Paysage"])
    }

    fn orientation_to_selected(orientation: Option<&export_config::PageOrientation>) -> u32 {
        match orientation {
            None => 0,
            Some(export_config::PageOrientation::Portrait) => 1,
            Some(export_config::PageOrientation::Landscape) => 2,
        }
    }

    fn selected_to_orientation(selected: u32) -> Option<export_config::PageOrientation> {
        match selected {
            0 => None,
            1 => Some(export_config::PageOrientation::Portrait),
            2 => Some(export_config::PageOrientation::Landscape),
            _ => panic!("Invalid selection for orientation"),
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = String;

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            #[watch]
            set_title: Some(&format!("Configuration : {}", model.sheet_display_name)),
            set_default_size: (500, 285),
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

                            #[name(sheet_name_entry)]
                            adw::EntryRow {
                                set_title: "Nom de la feuille",
                                #[track(model.should_redraw)]
                                set_text: &model.config.sheet_name,
                                connect_text_notify[sender] => move |widget| {
                                    let text: String = widget.text().into();
                                    sender.input(DialogInput::UpdateSheetName(text));
                                },
                            },

                            #[name(orientation_combo)]
                            adw::ComboRow {
                                set_title: "Orientation de la page",
                                set_model: Some(&Self::generate_orientation_model()),
                                #[track(Self::orientation_to_selected(model.config.orientation.as_ref()) != orientation_combo.selected())]
                                set_selected: Self::orientation_to_selected(model.config.orientation.as_ref()),
                                connect_selected_notify[sender] => move |widget| {
                                    let selected = widget.selected();
                                    sender.input(DialogInput::UpdateOrientation(
                                        Self::selected_to_orientation(selected)
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
                        },
                    },
                },
            }
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            hidden: true,
            move_front: false,
            should_redraw: false,
            sheet_display_name: params,
            config: export_config::PerStudentGroupsConfig::default_all_groups(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        self.move_front = false;
        match msg {
            DialogInput::Show(config) => {
                self.config = config;
                self.hidden = false;
                self.move_front = true;
                self.should_redraw = true;
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
            }
            DialogInput::Accept => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender
                        .output(DialogOutput::Accepted(self.config.clone()))
                        .unwrap();
                }
            }
            DialogInput::UpdateSheetName(new_name) => {
                if self.config.sheet_name == new_name {
                    return;
                }
                self.config.sheet_name = new_name;
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
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
