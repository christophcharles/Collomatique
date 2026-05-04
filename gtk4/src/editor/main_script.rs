use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::gtk;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

mod modules_dialog;

pub struct MainScript {
    modules_dialog: Controller<modules_dialog::Dialog>,
}

#[derive(Debug)]
pub enum MainScriptInput {
    Update,
    ShowModulesClicked,
}

#[relm4::component(pub)]
impl Component for MainScript {
    type Init = ();
    type Input = MainScriptInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_vexpand: true,
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 5,
            set_spacing: 10,

            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "Script de génération des contraintes",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                },
                gtk::Button {
                    set_icon_name: "view-list-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Afficher les modules disponibles"),
                    connect_clicked => MainScriptInput::ShowModulesClicked,
                },
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
                gtk::TextView {
                    set_editable: false,
                    set_monospace: true,
                    set_sensitive: false,
                    #[wrap(Some)]
                    set_buffer = &gtk::TextBuffer {
                        set_text: collomatique_binding_colloscopes::scripts::get_default_main_module(),
                    },
                }
            },
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let modules_dialog = modules_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let model = MainScript { modules_dialog };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            MainScriptInput::Update => {}
            MainScriptInput::ShowModulesClicked => {
                self.modules_dialog
                    .sender()
                    .send(modules_dialog::DialogInput::Show)
                    .unwrap();
            }
        }
    }
}
