use adw::prelude::{ActionRowExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, ButtonExt, GtkWindowExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    week_count: usize,
    max_week_count: usize,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(usize),
    Cancel,
    Accept,
    Select(usize),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(usize),
}

impl Dialog {
    fn generate_remainder_text(&self) -> String {
        format!("{}", self.max_week_count - self.week_count)
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
            set_resizable: false,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Configuration du découpage"),
            set_size_request: (-1, -1),

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
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_margin_all: 5,
                    gtk::ListBox {
                        set_hexpand: true,
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        adw::SpinRow {
                            set_hexpand: true,
                            set_title: "Nombre de semaines à garder",
                            #[wrap(Some)]
                            set_adjustment = &gtk::Adjustment {
                                set_lower: 0.,
                                #[watch]
                                set_upper: model.max_week_count as f64,
                                set_step_increment: 1.,
                                set_page_increment: 5.,
                            },
                            set_wrap: false,
                            set_snap_to_ticks: true,
                            set_numeric: true,
                            #[track(model.should_redraw)]
                            set_value: model.week_count as f64,
                            connect_value_notify[sender] => move |widget| {
                                let week_count = widget.value() as usize;
                                sender.input(DialogInput::Select(week_count));
                            },
                        },
                        adw::ActionRow {
                            set_hexpand: true,
                            set_title: "Semaines dans la nouvelle période",
                            add_css_class: "dimmed",
                            add_suffix = &gtk::Label {
                                #[watch]
                                set_label: &model.generate_remainder_text(),
                            },
                        }
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            week_count: 0,
            max_week_count: 10,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(week_count) => {
                self.hidden = false;
                self.should_redraw = true;
                self.max_week_count = week_count;
                self.week_count = week_count / 2;
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.week_count))
                    .unwrap();
            }
            DialogInput::Select(week_count) => {
                self.week_count = week_count;
            }
        }
    }
}
