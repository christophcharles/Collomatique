use gtk::prelude::{ButtonExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};
use relm4::{adw, gtk};
use std::path::PathBuf;

use collomatique_state_colloscopes::export_config;

use crate::tools;

pub struct ExportPanel {
    export_config: export_config::ExportConfig,
}

#[derive(Debug)]
pub enum ExportPanelInput {
    Update(export_config::ExportConfig),
    ExportClicked,
}

#[derive(Debug)]
pub enum ExportPanelOutput {
    ExportColloscopeAs(PathBuf, collomatique_xlsx::Config),
}

#[derive(Debug)]
pub enum ExportPanelCommandOutput {
    FileChosen(PathBuf),
    FileNotChosen,
}

#[relm4::component(pub)]
impl Component for ExportPanel {
    type Input = ExportPanelInput;
    type Output = ExportPanelOutput;
    type Init = ();
    type CommandOutput = ExportPanelCommandOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_vexpand: true,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                gtk::Button {
                    add_css_class: "frame",
                    add_css_class: "accent",
                    set_hexpand: true,
                    set_margin_all: 5,
                    adw::ButtonContent {
                        set_icon_name: "document-export-symbolic",
                        set_label: "Exporter le colloscope",
                    },
                    connect_clicked => ExportPanelInput::ExportClicked,
                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ExportPanel {
            export_config: export_config::ExportConfig::default(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            ExportPanelInput::Update(config) => {
                self.export_config = config;
            }
            ExportPanelInput::ExportClicked => {
                sender.oneshot_command(async move {
                    match tools::open_save::generic_save_dialog(
                        "Exporter en XLSX",
                        &[
                            ("Fichiers XLSX (*.xlsx)", "xlsx"),
                            ("Tous les fichiers", "*"),
                        ],
                        Some("colloscope.xlsx"),
                    )
                    .await
                    {
                        Some(path) => ExportPanelCommandOutput::FileChosen(path),
                        None => ExportPanelCommandOutput::FileNotChosen,
                    }
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ExportPanelCommandOutput::FileNotChosen => {}
            ExportPanelCommandOutput::FileChosen(path) => {
                let xlsx_config = super::export::to_xlsx_config(&self.export_config);
                sender
                    .output(ExportPanelOutput::ExportColloscopeAs(path, xlsx_config))
                    .unwrap();
            }
        }
    }
}
