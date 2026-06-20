use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use collomatique_subprocesses::StrategyStatus;

use super::StrategyDisplayInput;

pub struct StrategyStatusBar {
    show_debug: bool,
    is_running: bool,
    end_with_error: bool,
    ipc_error: Option<String>,
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
            gtk::Image::from_icon_name("dialog-warning-symbolic") {
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: model.ipc_error.is_some(),
                #[watch]
                set_tooltip_text: model.ipc_error.as_deref(),
            },
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
                set_visible: !model.is_running && !model.end_with_error,
                gtk::Image::from_icon_name("emblem-ok-symbolic") {
                    set_size_request: (30, 30),
                    set_icon_size: gtk::IconSize::Normal,
                },
                gtk::Label {
                    set_label: "Exécution terminée",
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: !model.is_running && model.end_with_error,
                gtk::Image::from_icon_name("dialog-error-symbolic") {
                    set_size_request: (30, 30),
                    set_icon_size: gtk::IconSize::Normal,
                },
                gtk::Label {
                    set_label: "Erreur pendant l'exécution",
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
            end_with_error: false,
            ipc_error: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            StrategyDisplayInput::Clear(_) => {
                self.show_debug = false;
                self.is_running = true;
                self.end_with_error = false;
                self.ipc_error = None;
            }
            StrategyDisplayInput::StrategyUpdate(progress) => {
                if let Err(e) = progress {
                    self.ipc_error = Some(format!("Erreur IPC : {e}"));
                }
            }
            StrategyDisplayInput::Finished(result) => {
                self.is_running = false;
                self.end_with_error = matches!(
                    result.status,
                    StrategyStatus::Error | StrategyStatus::Infeasible
                );
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
            StrategyDisplayInput::Echo(_) => {}
        }
    }
}
