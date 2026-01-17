use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

pub struct MainScript {
    main_script: Option<String>,
}

#[derive(Debug)]
pub enum MainScriptInput {
    Update(Option<String>),
}

impl MainScript {
    fn get_display_text(&self) -> String {
        match &self.main_script {
            Some(content) => content.clone(),
            None => {
                collomatique_binding_colloscopes::scripts::get_default_main_module().to_string()
            }
        }
    }

    fn is_default(&self) -> bool {
        self.main_script.is_none()
    }
}

#[relm4::component(pub)]
impl Component for MainScript {
    type Init = ();
    type Input = MainScriptInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        adw::ToolbarView {
            set_hexpand: true,
            set_vexpand: true,
            add_top_bar = &adw::Banner {
                set_title: "Script par défaut",
                #[watch]
                set_revealed: model.is_default(),
            },
            #[wrap(Some)]
            set_content = &gtk::Box {
                set_hexpand: true,
                set_vexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 10,

                // Title row with buttons
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_label: "Script de génération des contraintes",
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                    },
                    gtk::Button {
                        set_icon_name: "document-edit-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Modifier le script"),
                    },
                    gtk::Button {
                        set_icon_name: "view-list-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Afficher les modules disponibles"),
                    },
                    // Spacer to push restore button to far right
                    gtk::Box {
                        set_hexpand: true,
                    },
                    gtk::Button {
                        set_icon_name: "edit-undo-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Restaurer le script par défaut"),
                    },
                },

                // Script text view
                gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
                    gtk::TextView {
                        set_editable: false,
                        set_monospace: true,
                        #[wrap(Some)]
                        set_buffer = &gtk::TextBuffer {
                            #[watch]
                            set_text: &model.get_display_text(),
                        },
                    }
                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = MainScript { main_script: None };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            MainScriptInput::Update(main_script) => {
                self.main_script = main_script;
            }
        }
    }
}
