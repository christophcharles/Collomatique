use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use super::StrategyDisplayInput;

pub struct StrategyStatusBar {
    show_debug: bool,
    is_running: bool,
}

#[derive(Debug)]
pub enum StrategyStatusBarOutput {
    ToggleDebug(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for StrategyStatusBar {
    type Init = ();
    type Input = StrategyDisplayInput;
    type Output = StrategyStatusBarOutput;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 10,
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: model.is_running,
                set_spacing: 5,
                adw::Spinner {},
                gtk::Label {
                    set_label: "Exécution en cours",
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: !model.is_running,
                gtk::Label {
                    set_label: "À l'arrêt",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
            },
            gtk::ToggleButton {
                set_icon_name: "utilities-terminal-symbolic",
                #[watch]
                set_active: model.show_debug,
                connect_toggled[sender] => move |btn| {
                    sender.input(StrategyDisplayInput::ToggleDebug(btn.is_active()));

                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = StrategyStatusBar {
            show_debug: false,
            is_running: false,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            StrategyDisplayInput::Clear => {
                self.show_debug = false;
                self.is_running = true;
            }
            StrategyDisplayInput::Assigned(assignment) => {
                self.is_running = assignment.is_some();
            }
            StrategyDisplayInput::ToggleDebug(toggle) => {
                if self.show_debug == toggle {
                    return;
                }
                self.show_debug = toggle;
                sender
                    .output(StrategyStatusBarOutput::ToggleDebug(toggle))
                    .unwrap();
            }
            StrategyDisplayInput::Echo(_) | StrategyDisplayInput::StrategyUpdate(_) => {}
        }
    }
}
