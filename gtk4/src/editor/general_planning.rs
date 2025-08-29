use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

#[derive(Debug)]
pub enum GeneralPlanningInput {}

pub struct GeneralPlanning {}

#[relm4::component(pub)]
impl Component for GeneralPlanning {
    type Input = GeneralPlanningInput;
    type Output = ();
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_margin_all: 5,
            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b><big>Début de la première semaine de colles :</big></b> 01/09/2025",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                            },
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "<b>Nombre total de semaines de colles :</b> 4",
                            set_use_markup: true,
                        },
                    },
                    gtk::Box {
                        set_hexpand: true,
                    },
                    gtk::Button {
                        add_css_class: "suggested-action",
                        set_sensitive: false,
                        adw::ButtonContent {
                            set_icon_name: "preferences-other-symbolic",
                            set_label: "Générer automatiquement",
                        }
                    }
                },
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 30,
                    set_spacing: 30,
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b><big>Période 1 du 01/09/2025 au 21/09/2025 (semaines 1 à 3)</big></b>",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Button {
                                set_icon_name: "edit-cut-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 1 du 01/09/2025 au 07/09/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: true,
                                },
                            },
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 2 du 08/09/2025 au 14/09/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: true,
                                },
                            },
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "dimmed",
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 3 du 15/09/2025 au 21/09/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: false,
                                },
                            },
                        }
                    },
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b><big>Période 2 du 22/09/2025 au 05/10/2025  (semaines 4 à 5)</big></b>",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Button {
                                set_icon_name: "edit-cut-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 4 du 22/09/2025 au 28/09/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: true,
                                },
                            },
                            append = &gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 5,
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "Semaine 5 du 29/09/2025 au 05/10/2025"
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Switch {
                                    set_active: true,
                                },
                            },
                        }
                    },
                    gtk::Button {
                        adw::ButtonContent {
                            set_icon_name: "edit-add",
                            set_label: "Ajouter une période",
                        }
                    },
                }
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = GeneralPlanning {};
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        _message: Self::Input,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }
}
