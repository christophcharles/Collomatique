use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_binding_colloscopes::scripts::SimpleProblemError;

type ProblemBuilder = collo_ml::problem::ProblemBuilder<
    collomatique_binding_colloscopes::views::ObjectId,
    collomatique_binding_colloscopes::vars::Var,
>;

mod edit_dialog;
mod modules_dialog;

#[derive(Clone, Debug)]
pub enum ErrorMsg {
    Error(String),
    Warning(String),
}

pub struct MainScript {
    main_script: Option<String>,
    errors: Option<Vec<ErrorMsg>>,
    errors_list: FactoryVecDeque<ErrorEntry>,
    edit_dialog: Controller<edit_dialog::Dialog>,
    modules_dialog: Controller<modules_dialog::Dialog>,
}

#[derive(Debug)]
pub enum MainScriptInput {
    Update(
        Option<String>,
        Option<Result<ProblemBuilder, SimpleProblemError>>,
    ),
    RestoreDefaultClicked,
    EditClicked,
    DialogAccepted(String),
    ShowModulesClicked,
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

    fn update_errors_list(&mut self) {
        let messages = self.errors.as_ref().map(|e| e.clone()).unwrap_or_default();

        crate::tools::factories::update_vec_deque(
            &mut self.errors_list,
            messages.into_iter(),
            |x| ErrorEntryInput::Update(x),
        );
    }
}

#[relm4::component(pub)]
impl Component for MainScript {
    type Init = ();
    type Input = MainScriptInput;
    type Output = collomatique_ops::MainScriptUpdateOp;
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
                        connect_clicked => MainScriptInput::EditClicked,
                    },
                    gtk::Button {
                        set_icon_name: "view-list-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Afficher les modules disponibles"),
                        connect_clicked => MainScriptInput::ShowModulesClicked,
                    },
                    // Spacer to push restore button to far right
                    gtk::Box {
                        set_hexpand: true,
                    },
                    gtk::Button {
                        set_icon_name: "edit-delete-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Restaurer le script par défaut"),
                        #[watch]
                        set_sensitive: !model.is_default(),
                        connect_clicked => MainScriptInput::RestoreDefaultClicked,
                    },
                },

                // Paned: script view (top) + error display (bottom)
                gtk::Paned {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_position: 400,

                    // Top: Script TextView
                    #[wrap(Some)]
                    set_start_child = &gtk::ScrolledWindow {
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

                    // Bottom: Error display (conditional)
                    #[wrap(Some)]
                    set_end_child = &gtk::Box {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,

                        // State 1: Compiling (errors is None)
                        gtk::Label {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_label: "Compilation du script...",
                            #[watch]
                            set_visible: model.errors.is_none(),
                        },

                        // State 2: No errors (errors is Some([]))
                        gtk::Box {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 10,
                            #[watch]
                            set_visible: matches!(&model.errors, Some(e) if e.is_empty()),

                            gtk::Image {
                                set_icon_name: Some("object-select-symbolic"),
                                add_css_class: "success",
                            },
                            gtk::Label {
                                set_label: "Aucune erreur",
                            },
                        },

                        // State 3: Has errors (errors is Some([...]))
                        gtk::ScrolledWindow {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
                            #[watch]
                            set_visible: matches!(&model.errors, Some(e) if !e.is_empty()),

                            #[local_ref]
                            errors_listbox -> gtk::ListBox {
                                set_hexpand: true,
                                set_vexpand: true,
                                add_css_class: "boxed-list",
                                set_selection_mode: gtk::SelectionMode::None,
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let edit_dialog = edit_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                edit_dialog::DialogOutput::Accepted(text) => MainScriptInput::DialogAccepted(text),
            });

        let modules_dialog = modules_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let errors_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .detach();

        let model = MainScript {
            main_script: None,
            errors: None,
            errors_list,
            edit_dialog,
            modules_dialog,
        };

        let errors_listbox = model.errors_list.widget();

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            MainScriptInput::Update(main_script, main_script_ast) => {
                self.main_script = main_script;
                self.errors = match main_script_ast {
                    None => None,
                    Some(Err(e)) => match e {
                        SimpleProblemError::UnexpectedError(e) => Some(vec![ErrorMsg::Error(e)]),
                        SimpleProblemError::ParsingError(e) => {
                            Some(vec![ErrorMsg::Error(e.to_string())])
                        }
                        SimpleProblemError::SemanticErrors { errors, warnings } => Some(
                            errors
                                .into_iter()
                                .map(|e| ErrorMsg::Error(e.to_string()))
                                .chain(
                                    warnings
                                        .into_iter()
                                        .map(|w| ErrorMsg::Warning(w.to_string())),
                                )
                                .collect(),
                        ),
                    },
                    Some(Ok(builder)) => Some(
                        builder
                            .get_warnings()
                            .iter()
                            .map(|w| ErrorMsg::Warning(w.to_string()))
                            .collect(),
                    ),
                };
                self.update_errors_list();
            }
            MainScriptInput::RestoreDefaultClicked => {
                sender
                    .output(collomatique_ops::MainScriptUpdateOp::UpdateScript(None))
                    .unwrap();
            }
            MainScriptInput::EditClicked => {
                self.edit_dialog
                    .sender()
                    .send(edit_dialog::DialogInput::Show(self.get_display_text()))
                    .unwrap();
            }
            MainScriptInput::DialogAccepted(text) => {
                sender
                    .output(collomatique_ops::MainScriptUpdateOp::UpdateScript(Some(
                        text,
                    )))
                    .unwrap();
            }
            MainScriptInput::ShowModulesClicked => {
                self.modules_dialog
                    .sender()
                    .send(modules_dialog::DialogInput::Show)
                    .unwrap();
            }
        }
    }
}

#[derive(Debug)]
struct ErrorEntry {
    message: ErrorMsg,
}

#[derive(Debug)]
enum ErrorEntryInput {
    Update(ErrorMsg),
}

impl ErrorEntry {
    fn get_text(&self) -> &str {
        match &self.message {
            ErrorMsg::Error(e) => e.as_str(),
            ErrorMsg::Warning(w) => w.as_str(),
        }
    }
}

#[relm4::factory]
impl FactoryComponent for ErrorEntry {
    type Init = ErrorMsg;
    type Input = ErrorEntryInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        #[name(root_box)]
        gtk::Box {
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            add_css_class: match &self.message {
                ErrorMsg::Error(_) => "error",
                ErrorMsg::Warning(_) => "warning",
            },
            gtk::Image {
                set_margin_end: 5,
                #[watch]
                set_icon_name: Some(match &self.message {
                    ErrorMsg::Error(_) => "dialog-error-symbolic",
                    ErrorMsg::Warning(_) => "dialog-warning-symbolic",
                }),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: self.get_text(),
            },
        },
    }

    fn init_model(
        message: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self { message }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            ErrorEntryInput::Update(message) => {
                self.message = message;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets.root_box.remove_css_class("error");
        widgets.root_box.remove_css_class("warning");
        widgets.root_box.add_css_class(match &self.message {
            ErrorMsg::Error(_) => "error",
            ErrorMsg::Warning(_) => "warning",
        });
    }
}
