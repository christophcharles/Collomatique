use gtk::prelude::{OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{FactorySender, RelmWidgetExt};

/// How loud a message is: an error blocks validation, a warning flags a choice
/// that works but costs, an info is a plain nudge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSeverity {
    Error,
    Warning,
    Info,
}

impl MessageSeverity {
    fn icon_name(self) -> &'static str {
        match self {
            MessageSeverity::Error => "dialog-error-symbolic",
            MessageSeverity::Warning => "dialog-warning-symbolic",
            MessageSeverity::Info => "dialog-information-symbolic",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            MessageSeverity::Error => "error",
            MessageSeverity::Warning => "warning",
            MessageSeverity::Info => "accent",
        }
    }
}

/// One line of feedback in a dialog's message area — meant to be pushed into a
/// [gtk::ListBox] carrying the `boxed-list` class.
#[derive(Debug)]
pub struct MessageRow {
    severity: MessageSeverity,
    message: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for MessageRow {
    type Init = (MessageSeverity, String);
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::Box {
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            add_css_class: self.severity.css_class(),
            gtk::Image {
                set_margin_end: 5,
                set_icon_name: Some(self.severity.icon_name()),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_wrap: true,
                set_label: &self.message,
            },
        },
    }

    fn init_model(
        (severity, message): Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self { severity, message }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

/// The same message reduced to its icon, for a row that has no space for the
/// text — the text becomes the icon's tooltip. Meant to be pushed into a
/// horizontal [gtk::Box].
#[derive(Debug)]
pub struct MessageIcon {
    severity: MessageSeverity,
    message: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for MessageIcon {
    type Init = (MessageSeverity, String);
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Image {
            set_icon_name: Some(self.severity.icon_name()),
            add_css_class: self.severity.css_class(),
            set_tooltip_text: Some(&self.message),
        },
    }

    fn init_model(
        (severity, message): Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self { severity, message }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}
