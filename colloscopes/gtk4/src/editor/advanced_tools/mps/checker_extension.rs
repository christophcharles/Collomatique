use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

use crate::editor::build_model::{ConfigExtension, ExtensionOutput};

/// What the MPS export writes out of the built model. One build carries both problems, so the
/// choice belongs here, next to the export, and not to the model configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MpsExportOptions {
    /// Write the checker problem (the constraints alone) instead of the full problem.
    pub checker: bool,
}

/// Exported-problem slot of the model-configuration dialog: full problem or checker problem, as
/// a linked pair of toggles.
pub struct Extension {
    options: MpsExportOptions,
}

#[derive(Debug)]
pub enum ExtensionInput {
    /// Seed the widgets from the options the configuration dialog was shown with. Does not
    /// announce a change: the dialog already holds this value.
    SetOptions(MpsExportOptions),
    /// The user picked the other problem.
    UpdateChecker(bool),
    IgnoreOrRefresh,
}

impl Extension {
    fn is_full(&self) -> bool {
        !self.options.checker
    }

    fn is_checker(&self) -> bool {
        self.options.checker
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Extension {
    type Init = gtk::Window;

    type Input = ExtensionInput;
    type Output = ExtensionOutput<MpsExportOptions>;

    view! {
        #[root]
        gtk::Frame {
            set_hexpand: true,
            set_margin_all: 5,
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 5,
                gtk::Label {
                    set_margin_start: 10,
                    set_margin_all: 5,
                    set_label: "<b>Problème exporté :</b>",
                    set_use_markup: true,
                },
                gtk::Box {
                    set_spacing: 0,
                    add_css_class: "linked",
                    #[name(full_toggle_btn)]
                    gtk::ToggleButton {
                        set_margin_top: 5,
                        set_margin_bottom: 5,
                        set_label: "Complet",
                        #[track(full_toggle_btn.is_active() != model.is_full())]
                        set_active: model.is_full(),
                        connect_toggled[sender] => move |widget| {
                            let new_state = widget.is_active();
                            sender.input(if new_state {
                                ExtensionInput::UpdateChecker(false)
                            } else {
                                ExtensionInput::IgnoreOrRefresh
                            });
                        }
                    },
                    #[name(checker_toggle_btn)]
                    gtk::ToggleButton {
                        set_margin_top: 5,
                        set_margin_bottom: 5,
                        set_label: "Vérification seule",
                        set_tooltip: "Les contraintes seules : ni objectif, ni les variables qui n'existaient que pour lui. Le fichier dit alors « ce colloscope est-il autorisé ? » au lieu de « quel est le meilleur colloscope ? ».",
                        #[track(checker_toggle_btn.is_active() != model.is_checker())]
                        set_active: model.is_checker(),
                        connect_toggled[sender] => move |widget| {
                            let new_state = widget.is_active();
                            sender.input(if new_state {
                                ExtensionInput::UpdateChecker(true)
                            } else {
                                ExtensionInput::IgnoreOrRefresh
                            });
                        }
                    },
                },
                gtk::Box {
                    set_hexpand: true,
                },
            },
        }
    }

    fn init(
        _window: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Extension {
            options: MpsExportOptions::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ExtensionInput::SetOptions(options) => {
                self.options = options;
            }
            ExtensionInput::UpdateChecker(checker) => {
                self.options.checker = checker;
                sender
                    .output(ExtensionOutput::ValueChanged(self.options))
                    .unwrap();
            }
            ExtensionInput::IgnoreOrRefresh => {}
        }
    }
}

impl ConfigExtension for Extension {
    type Value = MpsExportOptions;

    const WINDOW_TITLE: &'static str = "Configuration du modèle à exporter";

    fn set_value_msg(value: MpsExportOptions) -> ExtensionInput {
        ExtensionInput::SetOptions(value)
    }
}
