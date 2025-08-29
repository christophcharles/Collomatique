use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

#[derive(Debug)]
pub enum SubjectsInput {}

pub struct Subjects {}

#[relm4::component(pub)]
impl Component for Subjects {
    type Input = SubjectsInput;
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
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
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
                                set_label: "<b><big>Mathématiques</big></b>",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Modifier la période"),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                set_icon_name: "go-up",
                                add_css_class: "flat",
                                set_sensitive: false,
                                set_tooltip_text: Some("Remonter dans la liste"),
                            },
                            gtk::Button {
                                set_icon_name: "go-down",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Descendre dans la liste"),
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Supprimer la matière"),
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Élèves par groupes :</b> 2 à 3",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Groupes par colle :</b> 1",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Périodicité :</b> 2 semaines (glissantes)",
                                set_use_markup: true,
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::End,
                                set_label: "<i>60 minutes</i>",
                                set_use_markup: true,
                                add_css_class: "dimmed",
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 1 du 01/09/2025 au 05/10/2025 (semaines 1 à 5)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    add_css_class: "dimmed",
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 2 du 06/10/2025 au 09/11/2025 (semaines 6 à 10)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: false,
                                    },
                                }
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 3 du 10/11/2025 au 14/12/2025 (semaines 11 à 15)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                        },
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
                                set_label: "<b><big>Français</big></b>",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Modifier la période"),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                set_icon_name: "go-up",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Remonter dans la liste"),
                            },
                            gtk::Button {
                                set_icon_name: "go-down",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Descendre dans la liste"),
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Supprimer la matière"),
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Élèves par groupes :</b> 1",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Groupes par colle :</b> 3",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Colles dans l'année :</b> 2 (séparées de 2 semaines)",
                                set_use_markup: true,
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::End,
                                set_label: "<i>60 minutes</i>",
                                set_use_markup: true,
                                add_css_class: "dimmed",
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    add_css_class: "dimmed",
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 1 du 01/09/2025 au 05/10/2025 (semaines 1 à 5)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: false,
                                    },
                                }
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 2 du 06/10/2025 au 09/11/2025 (semaines 6 à 10)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 3 du 10/11/2025 au 14/12/2025 (semaines 11 à 15)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                        },
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
                                set_label: "<b><big>TP Informatique</big></b>",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Modifier la période"),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                set_icon_name: "go-up",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Remonter dans la liste"),
                            },
                            gtk::Button {
                                set_icon_name: "go-down",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Descendre dans la liste"),
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Supprimer la matière"),
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Élèves par groupes :</b> 2 à 3",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Groupes par colle :</b> 3 à 5",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Périodicité :</b> 2 semaines (par bloc)",
                                set_use_markup: true,
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::End,
                                set_label: "<i>120 minutes (non-comptées)</i>",
                                set_use_markup: true,
                                add_css_class: "dimmed",
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 1 du 01/09/2025 au 05/10/2025 (semaines 1 à 5)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 2 du 06/10/2025 au 09/11/2025 (semaines 6 à 10)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 3 du 10/11/2025 au 14/12/2025 (semaines 11 à 15)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                        },
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
                                set_label: "<b><big>Entretiens</big></b>",
                                set_use_markup: true,
                            },
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Modifier la période"),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                set_icon_name: "go-up",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Remonter dans la liste"),
                            },
                            gtk::Button {
                                set_icon_name: "go-down",
                                add_css_class: "flat",
                                set_sensitive: false,
                                set_tooltip_text: Some("Descendre dans la liste"),
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                                set_tooltip_text: Some("Supprimer la matière"),
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Élèves par groupes :</b> 2 à 3",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Groupes par colle :</b> 1",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<b>Colles dans l'année :</b> 2",
                                set_use_markup: true,
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<i>60 minutes</i>",
                                set_use_markup: true,
                                add_css_class: "dimmed",
                            },
                        },
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 1 du 01/09/2025 au 05/10/2025 (semaines 1 à 5)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 2 du 06/10/2025 au 09/11/2025 (semaines 6 à 10)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: true,
                                    },
                                }
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_hexpand: true,
                                    set_margin_all: 5,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    add_css_class: "dimmed",
                                    gtk::Label {
                                        set_margin_all: 5,
                                        set_label: "Période 3 du 10/11/2025 au 14/12/2025 (semaines 11 à 15)",
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                    gtk::Switch {
                                        set_active: false,
                                    },
                                }
                            },
                        },
                    },
                },
                gtk::Button {
                    set_margin_top: 10,
                    adw::ButtonContent {
                        set_icon_name: "edit-add",
                        set_label: "Ajouter une matière",
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Subjects {};
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
