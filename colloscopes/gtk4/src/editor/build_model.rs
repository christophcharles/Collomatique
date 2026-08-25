//! Shared building blocks for turning the current document into an ILP model: the dialog that
//! configures what gets rebuilt ([`config_dialog`]) and the dialog that runs the build off-thread
//! ([`loading_dialog`]).
//!
//! The configuration dialog is generic over a [`ConfigExtension`]: a component filling the slot
//! next to the "Paramètres avancés" button with whatever the consumer needs on top of the model
//! configuration proper (the solver strategy for the colloscope resolution, the choice of the
//! exported problem for the MPS export).

pub mod config_dialog;
pub mod loading_dialog;

use relm4::gtk;

/// What a [`ConfigExtension`] tells the configuration dialog.
#[derive(Debug)]
pub enum ExtensionOutput<V> {
    /// The extension's value changed. Emitted on every edit, so the dialog always holds the
    /// current value and never has to ask for it when the user validates.
    ValueChanged(V),
    /// One of the extension's own dialogs just closed: the configuration window should be
    /// brought back to the front, because Windows will not do it on its own.
    Present,
}

/// A component plugged into [`config_dialog::Dialog`] to configure whatever is specific to one
/// consumer of the built model. Its widget sits at the bottom of the window, left of the
/// "Paramètres avancés" button, and it is handed the configuration window at launch so it can
/// make its own dialogs transient for it.
pub trait ConfigExtension:
    relm4::Component<
        Root: AsRef<gtk::Widget>,
        Init = gtk::Window,
        Output = ExtensionOutput<Self::Value>,
    >
{
    /// The configuration the extension assembles, handed back to the consumer on validation.
    type Value: Clone + std::fmt::Debug + Send + 'static;

    /// Title of the configuration window hosting this extension.
    const WINDOW_TITLE: &'static str;

    /// Message seeding the extension's widgets from a value, sent every time the dialog is shown.
    fn set_value_msg(value: Self::Value) -> Self::Input;
}
