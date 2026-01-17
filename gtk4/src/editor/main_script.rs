use gtk::prelude::{TextBufferExt, TextViewExt, WidgetExt};
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
            set_content = &gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                set_margin_all: 5,
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
