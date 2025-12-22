use collo_ml::eval::Origin;
use collomatique_binding_colloscopes::views::ObjectId;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    warnings: Option<Vec<Origin<ObjectId>>>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Close,
    Update(Option<Vec<Origin<ObjectId>>>),
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
            set_size_request: (500, 400),

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
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        gtk::Label {
                            set_hexpand: true,
                            set_vexpand: true,
                            #[watch]
                            set_visible: model.warnings.is_none(),
                            set_label: "Constraintes en cours de reconstruction...",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                        },
                        gtk::Box {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 10,
                            gtk::Box {
                                set_hexpand: true,
                            },
                            #[watch]
                            set_visible: model.warnings.as_ref().map(|x| x.is_empty()).unwrap_or(false),
                            gtk::Image {
                                set_icon_size: gtk::IconSize::Large,
                                set_icon_name: Some("emblem-success"),
                            },
                            gtk::Label {
                                set_label: "Toutes les contraintes sont satisfaites",
                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            set_vexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            #[watch]
                            set_visible: model.warnings.as_ref().map(|x| !x.is_empty()).unwrap_or(false),
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
            warnings: None,
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
            DialogInput::Update(warnings) => {
                self.warnings = warnings;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
