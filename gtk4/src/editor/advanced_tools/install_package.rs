use gtk::prelude::{
    BoxExt, ButtonExt, EntryBufferExt, EntryBufferExtManual, EntryExt, GtkWindowExt, OrientableExt,
    WidgetExt,
};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{SimpleComponent, adw, gtk};

use crate::widgets::debug_view::DebugView;

pub struct Dialog {
    hidden: bool,
    /// What the entry holds.
    ///
    /// Kept in the model because a `SimpleComponent`'s `update` cannot reach
    /// its widgets; same arrangement as `run_python_script/input_dialog.rs`.
    package: String,
    /// Where the install will speak, the same widget the script runner uses for
    /// a subprocess's output.
    debug_view: Controller<DebugView>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Close,
    UpdatePackage(String),
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    /// Nothing here touches the document, so there is nothing to report back.
    type Output = ();

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_default_size: (600, 400),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Installer un paquet Python"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_end = &gtk::Button {
                        set_label: "Fermer",
                        connect_clicked => DialogInput::Close,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
                    set_spacing: 10,
                    set_hexpand: true,
                    set_vexpand: true,

                    gtk::Box {
                        set_margin_all: 5,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 10,
                        #[name(package_entry)]
                        gtk::Entry {
                            set_hexpand: true,
                            set_placeholder_text: Some("Nom du module, par exemple : xlsxwriter"),
                            set_buffer = &gtk::EntryBuffer {
                                connect_text_notify[sender] => move |widget| {
                                    let text: String = widget.text().into();
                                    sender.input(DialogInput::UpdatePackage(text));
                                },
                            },
                        },
                        gtk::Button {
                            add_css_class: "suggested-action",
                            set_label: "Installer",
                            // Nothing behind it yet, so no `connect_clicked`:
                            // the message it will send does not exist.
                            #[watch]
                            set_sensitive: !model.package.trim().is_empty(),
                        },
                    },

                    append = model.debug_view.widget(),
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let debug_view = DebugView::builder().launch(()).detach();

        let model = Dialog {
            hidden: true,
            package: String::new(),
            debug_view,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            // The log is deliberately not cleared: each install announces
            // itself with its own line, so keeping it means a second install
            // does not erase what the first one said.
            DialogInput::Show => {
                self.hidden = false;
            }
            DialogInput::Close => {
                self.hidden = true;
            }
            DialogInput::UpdatePackage(package) => {
                self.package = package;
            }
        }
    }
}
