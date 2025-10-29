use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    student_name: Option<String>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::settings::Limits,
        Option<String>,
    ),
    Cancel,
    Accept,
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::settings::Limits),
}

impl Dialog {
    fn generate_params_name(&self) -> String {
        match &self.student_name {
            Some(name) => format!("Élève concerné : {}", name),
            None => "Paramètres globaux".into(),
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Paramètres supplémentaires"),
            set_default_size: (500, 300),
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Accept,
                        set_sensitive: false,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 5,
                    set_spacing: 10,
                    set_orientation: gtk::Orientation::Vertical,
                    #[name(scrolled_window)]
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        gtk::Box {
                            set_hexpand: true,
                            set_margin_all: 5,
                            set_spacing: 10,
                            set_orientation: gtk::Orientation::Vertical,
                            gtk::Label {
                                set_label: "Test",
                            },
                        },
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        #[watch]
                        set_label: &model.generate_params_name(),
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
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
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            student_name: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(limits, student_name) => {
                self.hidden = false;
                self.should_redraw = true;
                self.student_name = student_name;
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.build_limits()))
                    .unwrap();
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
        }
    }
}

impl Dialog {
    fn build_limits(&self) -> collomatique_state_colloscopes::settings::Limits {
        todo!()
    }
}
