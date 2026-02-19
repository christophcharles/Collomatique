use gtk::prelude::{ButtonExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};
use relm4::{adw, gtk};
use std::path::PathBuf;

use collomatique_state_colloscopes::export_config;

use crate::tools;

pub struct ExportPanel {
    export_config: export_config::ExportConfig,
    file_name: Option<PathBuf>,
}

#[derive(Debug)]
pub enum ExportPanelInput {
    Update(export_config::ExportConfig, Option<PathBuf>),
    ExportClicked,
    ExportSqliteClicked,
}

#[derive(Debug)]
pub enum ExportPanelOutput {
    ExportColloscopeAs(PathBuf, collomatique_xlsx::Config),
    ExportSqliteAs(PathBuf),
}

#[derive(Debug)]
pub enum ExportPanelCommandOutput {
    FileChosen(PathBuf),
    FileNotChosen,
    SqliteFileChosen(PathBuf),
    SqliteFileNotChosen,
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
                gtk::Box {
                    set_vexpand: true,
                    set_size_request: (-1,300),
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "Informations de débogage",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                    set_margin_all: 5,
                },
                gtk::Button {
                    add_css_class: "frame",
                    add_css_class: "warning",
                    set_hexpand: true,
                    set_margin_all: 5,
                    adw::ButtonContent {
                        set_icon_name: "document-export-symbolic",
                        set_label: "Exporter la base de donnée SQL",
                    },
                    connect_clicked => ExportPanelInput::ExportSqliteClicked,
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
            file_name: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            ExportPanelInput::Update(config, file_name) => {
                self.export_config = config;
                self.file_name = file_name;
            }
            ExportPanelInput::ExportClicked => {
                let default = match &self.file_name {
                    Some(path) => {
                        let mut xlsx_path = path.clone();
                        xlsx_path.set_extension("xlsx");
                        tools::open_save::DefaultSaveFile::ExistingFile(xlsx_path)
                    }
                    None => tools::open_save::DefaultSaveFile::SuggestedName(
                        format!("{}.xlsx", super::DEFAULT_FILE_STEM).into(),
                    ),
                };
                sender.oneshot_command(async move {
                    match tools::open_save::save_xlsx_dialog(default).await {
                        Some(path) => ExportPanelCommandOutput::FileChosen(path),
                        None => ExportPanelCommandOutput::FileNotChosen,
                    }
                });
            }
            ExportPanelInput::ExportSqliteClicked => {
                let default = match &self.file_name {
                    Some(path) => {
                        let mut sqlite_path = path.clone();
                        sqlite_path.set_extension("sqlite");
                        tools::open_save::DefaultSaveFile::ExistingFile(sqlite_path)
                    }
                    None => tools::open_save::DefaultSaveFile::SuggestedName(
                        format!("{}.sqlite", super::DEFAULT_FILE_STEM).into(),
                    ),
                };
                sender.oneshot_command(async move {
                    match tools::open_save::save_sqlite_dialog(default).await {
                        Some(path) => ExportPanelCommandOutput::SqliteFileChosen(path),
                        None => ExportPanelCommandOutput::SqliteFileNotChosen,
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
            ExportPanelCommandOutput::FileNotChosen
            | ExportPanelCommandOutput::SqliteFileNotChosen => {}
            ExportPanelCommandOutput::FileChosen(path) => {
                let xlsx_config = super::export::to_xlsx_config(&self.export_config);
                sender
                    .output(ExportPanelOutput::ExportColloscopeAs(path, xlsx_config))
                    .unwrap();
            }
            ExportPanelCommandOutput::SqliteFileChosen(path) => {
                sender
                    .output(ExportPanelOutput::ExportSqliteAs(path))
                    .unwrap();
            }
        }
    }
}
