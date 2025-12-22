use gtk::prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    move_front: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Close,
    Update,
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = ();

    view! {
        #[root]
        root_window = adw::Window {
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Erreurs dans le colloscope"),
            set_size_request: (-1, -1),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_end = &gtk::Button {
                        set_label: "Fermer",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Close,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 5,
                    gtk::ListBox {
                        set_hexpand: true,
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        gtk::ListBoxRow {
                            gtk::Box {
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "warning",
                                gtk::Image {
                                    set_margin_end: 5,
                                    set_icon_name: Some("emblem-warning"),
                                },
                                gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_label: "Contrainte n°3",
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
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        self.move_front = false;
        match msg {
            DialogInput::Show => {
                self.hidden = false;
                self.move_front = true;
            }
            DialogInput::Close => {
                self.hidden = true;
            }
            DialogInput::Update => {}
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
