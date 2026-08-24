use std::ffi::OsStr;

use gtk::prelude::{
    BoxExt, ButtonExt, EntryBufferExt, EntryBufferExtManual, EntryExt, GtkWindowExt, OrientableExt,
    WidgetExt,
};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{SimpleComponent, adw, gtk};

use collomatique_subprocesses::{Process, ProcessEvent};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

use super::python_packages::InstallCommand;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    /// What the entry holds.
    ///
    /// Kept in the model because a `SimpleComponent`'s `update` cannot reach
    /// its widgets; same arrangement as
    /// `general_planning/annotation_dialog.rs`.
    package: String,
    /// Where the install speaks, the same widget the script runner uses for a
    /// subprocess's output.
    debug_view: Controller<DebugView>,
    /// The running pip, held for the whole run: dropping a `Process` kills the
    /// child. Cleared when it exits, which is also what re-enables "Installer".
    process: Option<Process>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Close,
    UpdatePackage(String),
    Install,
    /// A chunk of pip's output. Chunks, not lines: `Process` reports what it
    /// read, and the text view appends either way.
    Output(String),
    Finished(Option<u32>),
}

/// Nothing here touches the document, so the only thing to report back is the
/// window handover.
#[derive(Debug)]
pub enum DialogOutput {
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
                            // Nothing to install without a name, and nothing to
                            // start while pip is still running.
                            #[watch]
                            set_sensitive: !model.package.trim().is_empty()
                                && model.process.is_none(),
                            connect_clicked => DialogInput::Install,
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
            move_front: false,
            package: String::new(),
            debug_view,
            process: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.move_front = false;
        match msg {
            // The log is deliberately not cleared: each install announces
            // itself with its own line, so keeping it means a second install
            // does not erase what the first one said.
            DialogInput::Show => {
                self.hidden = false;
                self.move_front = true;
            }
            // Closing does not kill anything: the window hides while pip
            // finishes, and reopening shows the same log with "Installer" still
            // greyed until it is done. Cutting an install in half is the worse
            // of those two behaviours, and a window that refuses to close is
            // the worse of the other two.
            DialogInput::Close => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
            }
            DialogInput::UpdatePackage(package) => {
                self.package = package;
            }
            DialogInput::Install => {
                // Several modules on one line, like both shell scripts accept.
                let packages: Vec<String> = self
                    .package
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                if packages.is_empty() || self.process.is_some() {
                    return;
                }

                let command = match InstallCommand::build(&packages) {
                    Ok(command) => command,
                    Err(e) => {
                        self.debug_view
                            .emit(DebugViewInput::Append(format!("# {e}\n")));
                        return;
                    }
                };

                // What was run, before what it said. Also what separates one
                // install from the previous one in the log.
                self.debug_view.emit(DebugViewInput::Append(format!(
                    "# {}\n",
                    command.command_line()
                )));

                let input = sender.input_sender().clone();
                let callback = move |event: ProcessEvent| match event {
                    ProcessEvent::Stdout(data) | ProcessEvent::Stderr(data) => {
                        input.emit(DialogInput::Output(data.into_lossy_string()));
                    }
                    ProcessEvent::ProcessExited(code) => {
                        input.emit(DialogInput::Finished(code));
                    }
                };

                let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
                // Python block-buffers whenever its output is not a terminal,
                // which behind a pipe means pip says nothing until it is done.
                let envs = [("PYTHONUNBUFFERED", OsStr::new("1"))];

                // The same split, for the same reasons, as `Worker::spawn` in
                // colloscopes/subprocesses/src/worker.rs: a pty on unix, plain pipes on
                // windows, where ConPTY stalls at startup waiting for an answer
                // to the cursor position report it sends.
                #[cfg(unix)]
                let spawned =
                    Process::spawn_pty(command.program().as_os_str(), &args, &envs, callback);
                #[cfg(windows)]
                let spawned =
                    Process::spawn_pipes(command.program().as_os_str(), &args, &envs, callback);

                match spawned {
                    Ok(process) => self.process = Some(process),
                    Err(e) => self
                        .debug_view
                        .emit(DebugViewInput::Append(format!("# {e}\n"))),
                }
            }
            DialogInput::Output(text) => {
                self.debug_view.emit(DebugViewInput::Append(text));
            }
            // Clearing the process is what makes "Installer" clickable again.
            // The closing line is only written for a failure: a successful pip
            // ends by saying so itself, whereas a failed one is otherwise only
            // visible by reading to the end.
            DialogInput::Finished(code) => {
                self.process = None;
                if code != Some(0) {
                    self.debug_view.emit(DebugViewInput::Append(match code {
                        Some(code) => format!("# échec (code {code})\n"),
                        None => "# échec (code inconnu)\n".to_string(),
                    }));
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
