use gtk::prelude::{ButtonExt, GtkWindowExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    move_front: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Close,
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();
    type Input = DialogInput;
    type Output = ();

    view! {
        #[root]
        #[name(root_window)]
        adw::Window {
            set_default_size: (1067, 600),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Modules disponibles"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_end = &gtk::Button {
                        set_label: "Fermer",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Close,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,

                    gtk::StackSidebar {
                        set_vexpand: true,
                        set_size_request: (200, -1),
                        set_stack: &modules_stack,
                    },
                    gtk::Separator {
                        set_orientation: gtk::Orientation::Vertical,
                    },
                    #[name(modules_stack)]
                    gtk::Stack {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_transition_type: gtk::StackTransitionType::SlideUpDown,
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            hidden: true,
            move_front: false,
        };
        let widgets = view_output!();

        // Populate the stack with modules
        for (name, content) in collomatique_binding_colloscopes::scripts::get_modules() {
            let scrolled_window = gtk::ScrolledWindow::builder()
                .hexpand(true)
                .vexpand(true)
                .margin_start(5)
                .margin_end(5)
                .margin_top(5)
                .margin_bottom(5)
                .build();

            let text_view = gtk::TextView::builder()
                .editable(false)
                .monospace(true)
                .build();

            let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
            buffer.set_text(content);
            text_view.set_buffer(Some(&buffer));

            scrolled_window.set_child(Some(&text_view));
            widgets
                .modules_stack
                .add_titled(&scrolled_window, Some(*name), *name);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        self.move_front = false;
        match msg {
            DialogInput::Show => {
                self.hidden = false;
                self.move_front = true;
            }
            DialogInput::Close => {
                self.hidden = true;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
