use adw::prelude::{ActionRowExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::export_config;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    should_redraw: bool,
    config: export_config::GlobalConfig,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(export_config::GlobalConfig),
    Cancel,
    Accept,

    UpdateBackgroundColor(export_config::Color),
    UpdateStripesEnabled(bool),
    UpdateStripesColor(export_config::Color),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(export_config::GlobalConfig),
}

impl Dialog {
    fn compute_gtk_color(color: &export_config::Color) -> gtk::gdk::RGBA {
        gtk::gdk::RGBA::new(
            color.red as f32 / 255.0f32,
            color.green as f32 / 255.0f32,
            color.blue as f32 / 255.0f32,
            1.0f32,
        )
    }

    fn compute_internal_color(gtk_color: &gtk::gdk::RGBA) -> export_config::Color {
        export_config::Color {
            red: (gtk_color.red() * 255.0f32) as u8,
            green: (gtk_color.green() * 255.0f32) as u8,
            blue: (gtk_color.blue() * 255.0f32) as u8,
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
        root_window = adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Configuration globale"),
            set_default_size: (500, 235),
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

                            adw::ActionRow {
                                set_title: "Couleur de fond (par défaut)",
                                add_suffix = &gtk::ColorDialogButton {
                                    set_margin_all: 5,
                                    #[watch]
                                    set_rgba: &Self::compute_gtk_color(&model.config.background_color),
                                    set_dialog = &gtk::ColorDialog {
                                        set_title: "Choisir la couleur de fond par défaut",
                                        set_with_alpha: false,
                                    },
                                    connect_rgba_notify[sender] => move |widget| {
                                        let rgba = widget.rgba();
                                        sender.input(DialogInput::UpdateBackgroundColor(
                                            Self::compute_internal_color(&rgba)
                                        ));
                                    },
                                },
                            },

                            #[name(stripes_switch)]
                            adw::SwitchRow {
                                set_title: "Activer le zébrage",
                                #[track(model.config.stripes_color_enabled != stripes_switch.is_active())]
                                set_active: model.config.stripes_color_enabled,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateStripesEnabled(widget.is_active()));
                                },
                            },

                            adw::ActionRow {
                                set_title: "Couleur de zébrage",
                                add_suffix = &gtk::ColorDialogButton {
                                    set_margin_all: 5,
                                    #[watch]
                                    set_rgba: &Self::compute_gtk_color(&model.config.stripes_color),
                                    set_dialog = &gtk::ColorDialog {
                                        set_title: "Choisir la couleur de zébrage",
                                        set_with_alpha: false,
                                    },
                                    connect_rgba_notify[sender] => move |widget| {
                                        let rgba = widget.rgba();
                                        sender.input(DialogInput::UpdateStripesColor(
                                            Self::compute_internal_color(&rgba)
                                        ));
                                    },
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
            move_front: false,
            should_redraw: false,
            config: export_config::GlobalConfig::default(),
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
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.config.clone()))
                    .unwrap();
            }
            DialogInput::UpdateBackgroundColor(background_color) => {
                if self.config.background_color == background_color {
                    return;
                }
                self.config.background_color = background_color;
            }
            DialogInput::UpdateStripesEnabled(stripes_enabled) => {
                if self.config.stripes_color_enabled == stripes_enabled {
                    return;
                }
                self.config.stripes_color_enabled = stripes_enabled;
            }
            DialogInput::UpdateStripesColor(stripes_color) => {
                if self.config.stripes_color == stripes_color {
                    return;
                }
                self.config.stripes_color = stripes_color;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
