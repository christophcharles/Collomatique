use gtk::prelude::{TextBufferExt, TextViewExt, WidgetExt};
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt, gtk};

#[derive(Debug)]
pub enum DebugViewInput {
    Append(String),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BufferOp {
    Clear,
    Append(String),
}

pub struct DebugView {
    buffer_op: Option<BufferOp>,
    /// Maximum number of lines kept in the buffer; `None` means unbounded.
    /// Oldest lines are dropped from the start when the cap is exceeded.
    max_lines: Option<usize>,
}

#[relm4::component(pub)]
impl Component for DebugView {
    type Input = DebugViewInput;
    type Output = ();
    /// Maximum number of lines to keep, or `None` for an unbounded buffer.
    type Init = Option<usize>;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_vexpand: true,
            set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
            set_margin_all: 5,
            #[name(text_view)]
            gtk::TextView {
                add_css_class: "frame",
                add_css_class: "osd",
                set_wrap_mode: gtk::WrapMode::Char,
                set_editable: false,
                set_monospace: true,
                #[name(text_buffer)]
                #[wrap(Some)]
                set_buffer = &gtk::TextBuffer {
                    #[track(model.buffer_op == Some(BufferOp::Clear))]
                    set_text: "",
                },
            }
        }
    }

    fn init(
        max_lines: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = DebugView {
            buffer_op: None,
            max_lines,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            DebugViewInput::Append(text) => self.buffer_op = Some(BufferOp::Append(text)),
            DebugViewInput::Clear => self.buffer_op = Some(BufferOp::Clear),
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update(message, sender.clone(), root);
        self.update_view(widgets, sender);
        self.update_buffer_if_needed(widgets);
    }
}

impl DebugView {
    fn update_buffer_if_needed(&mut self, widgets: &mut <Self as Component>::Widgets) {
        if let Some(BufferOp::Append(content)) = self.buffer_op.take() {
            let mut end_iter = widgets.text_buffer.end_iter();
            widgets.text_buffer.insert(&mut end_iter, &content);
            if let Some(max_lines) = self.max_lines {
                // `line_count` counts the trailing (possibly empty) line too; the
                // cap is a memory bound, not an exact count, so that is fine.
                let count = widgets.text_buffer.line_count();
                let excess = count - i32::try_from(max_lines).unwrap_or(i32::MAX);
                if excess > 0 {
                    let mut start = widgets.text_buffer.start_iter();
                    let mut cut = widgets
                        .text_buffer
                        .iter_at_line(excess)
                        .expect("excess < line_count, so the line exists");
                    widgets.text_buffer.delete(&mut start, &mut cut);
                }
            }
            let cursor_mark = widgets.text_buffer.get_insert();
            widgets.text_view.scroll_mark_onscreen(&cursor_mark);
        }
    }
}
