use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::typed_view::list::{RelmListItem, TypedListView};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

/// One line of the warning list: the rendered sentence and its nesting depth.
///
/// Depth 0 is a repair one of the user's own ops needed; a deeper line is a
/// sub-repair of the nearest line above it at the previous depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarningLine {
    pub text: String,
    pub depth: usize,
}

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    warnings: TypedListView<WarningLine, gtk::NoSelection>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(Vec<WarningLine>),
    Continue,
    Cancel,
}

#[derive(Debug)]
pub enum DialogOutput {
    Continue,
    Cancel,
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
        dialog = gtk::Window {
            set_modal: true,
            set_resizable: false,
            #[watch]
            set_visible: !model.hidden,
            connect_close_request[sender] => move |_| {
                sender.input(DialogInput::Cancel);
                gtk::glib::Propagation::Stop
            },
            add_controller = gtk::EventControllerKey {
                connect_key_pressed[sender] => move |_, key, _, _| {
                    if key == gtk::gdk::Key::Escape {
                        sender.input(DialogInput::Cancel);
                        gtk::glib::Propagation::Stop
                    } else {
                        gtk::glib::Propagation::Proceed
                    }
                }
            },
            #[wrap(Some)]
            set_titlebar = &gtk::Box {
                set_visible: false,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 20,
                set_margin_all: 25,

                gtk::Label {
                    set_label: "<big><b>Attention !</b></big>",
                    set_use_markup: true,
                    set_halign: gtk::Align::Center,
                },

                gtk::Label {
                    set_label: "L'opération est potentiellement destructive et aura les conséquences suivantes :",
                    set_wrap: true,
                    set_halign: gtk::Align::Start,
                },

                gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_propagate_natural_height: true,
                    set_max_content_height: 400,
                    set_size_request: (550, -1),

                    #[local_ref]
                    warnings_listview -> gtk::ListView {
                        add_css_class: "boxed-list",
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Annuler",
                        connect_clicked[sender] => move |_| {
                            sender.input(DialogInput::Cancel);
                        },
                    },

                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Poursuivre",
                        add_css_class: "destructive-action",
                        connect_clicked[sender] => move |_| {
                            sender.input(DialogInput::Continue);
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
        let warnings: TypedListView<WarningLine, gtk::NoSelection> = TypedListView::new();

        let model = Dialog {
            hidden: true,
            move_front: false,
            warnings,
        };

        let warnings_listview = &model.warnings.view;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.move_front = false;
        match msg {
            DialogInput::Show(warnings) => {
                self.hidden = false;
                self.move_front = true;
                self.warnings.clear();
                self.warnings.extend_from_iter(warnings);
            }
            DialogInput::Continue => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender.output(DialogOutput::Continue).unwrap()
                }
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender.output(DialogOutput::Cancel).unwrap()
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.dialog.present();
        }
    }
}

/// The per-row widgets `bind` writes into. Public only because it is the
/// associated `Widgets` type of a public `WarningLine`; nothing reads it from
/// outside.
pub struct WarningLineWidgets {
    label: gtk::Label,
}

impl RelmListItem for WarningLine {
    type Root = gtk::Box;
    type Widgets = WarningLineWidgets;

    fn setup(_list_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_end(5)
            .margin_top(5)
            .margin_bottom(5)
            .build();

        // The bullet lives in its own label rather than at the head of the
        // text: the two labels then sit side by side, so a sentence that wraps
        // keeps its whole body aligned past the bullet instead of running back
        // under it. `valign: Start` keeps the bullet on the first line of such
        // a sentence, and the fixed width makes every row's text start at the
        // same place.
        let bullet = gtk::Label::builder()
            .label("•")
            .width_request(14)
            .xalign(0.0)
            .valign(gtk::Align::Start)
            .build();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .xalign(0.0)
            .wrap(true)
            .build();

        root.append(&bullet);
        root.append(&label);

        (root, WarningLineWidgets { label })
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        // Rows are recycled, so the depth margin must be (re)set on every
        // bind, not only when non-zero. The wrapped continuation of a long
        // sentence keeps this margin too — the old flat-text rendering
        // could not do that.
        root.set_margin_start(5 + 24 * self.depth as i32);
        widgets.label.set_label(&self.text);
    }
}
