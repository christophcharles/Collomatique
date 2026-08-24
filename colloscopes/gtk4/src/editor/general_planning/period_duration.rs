use adw::prelude::PreferencesRowExt;
use gtk::prelude::{AdjustmentExt, ButtonExt, GtkWindowExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    should_redraw: bool,
    week_count: usize,
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
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_resizable: false,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Configuration de la période"),
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
                            set_title: "Nombre de semaines",
                            #[wrap(Some)]
                            set_adjustment = &gtk::Adjustment {
                                set_lower: 0.,
                                set_upper: 53.,
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
            move_front: false,
            should_redraw: false,
            week_count: 0,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        self.move_front = false;
        match msg {
            DialogInput::Show(week_count) => {
                self.hidden = false;
                self.move_front = true;
                self.should_redraw = true;
                self.week_count = week_count;
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
            }
            DialogInput::Accept => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender
                        .output(DialogOutput::Accepted(self.week_count))
                        .unwrap();
                }
            }
            DialogInput::Select(week_count) => {
                self.week_count = week_count;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
