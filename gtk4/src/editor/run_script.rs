use gtk::prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use vte4::TerminalExt;

use std::path::PathBuf;

pub struct Dialog {
    hidden: bool,
    path: PathBuf,
    pipe: Option<std::fs::File>,
}

#[derive(Debug)]
pub enum DialogInput {
    Run(PathBuf, String),
    Close,
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = ();

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_size_request: (700, 400),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Exécution du script Python"),
            add_css_class: "devel",

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_end = &gtk::Button {
                        set_label: "Fermer",
                        set_sensitive: true,
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Close,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        set_margin_all: 5,
                        #[name(term)]
                        vte4::Terminal {
                            set_hexpand: true,
                            set_vexpand: true,
                        },
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        add_css_class: "dimmed",
                        #[watch]
                        set_label: &model.path.to_string_lossy(),
                    },
                }
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Dialog {
            hidden: true,
            path: PathBuf::new(),
            pipe: None,
        };

        let widgets = view_output!();

        let pty = widgets
            .term
            .pty_new_sync(vte4::PtyFlags::empty(), gtk::gio::Cancellable::NONE)
            .unwrap();
        widgets.term.set_pty(Some(&pty));
        let owned_fd = pty.fd().try_clone_to_owned().unwrap();
        let peer_fd =
            rustix::pty::ioctl_tiocgptpeer(owned_fd, rustix::pty::OpenptFlags::RDWR).unwrap();
        let pipe = std::fs::File::from(peer_fd);
        model.pipe = Some(pipe);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Run(path, _script) => {
                self.hidden = false;
                self.path = path;
            }
            DialogInput::Close => {
                self.hidden = true;
            }
        }
    }
}
