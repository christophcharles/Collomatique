use iced::widget::{center, container, opaque, stack, text};
use iced::{Color, Element, Font};

pub fn modal<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    stack![
        base.into(),
        opaque(center(content).style(|_theme| {
            container::Style {
                background: Some(
                    Color {
                        a: 0.8,
                        ..Color::BLACK
                    }
                    .into(),
                ),
                ..container::Style::default()
            }
        }))
    ]
    .into()
}

pub fn icon_with_codepoint<'a, Message>(codepoint: char) -> Element<'a, Message> {
    const ICON_FONT: Font = Font::with_name("collomatique-icons");

    text(codepoint).font(ICON_FONT).into()
}

#[allow(dead_code)]
pub enum Icon {
    Undo,
    Redo,
    Open,
    New,
    Save,
    Close,
    Check,
    Error,
    Cogs,
    Edit,
    Download,
    Delete,
    Add,
    SortDown,
    SortUp,
    SortLettersDown,
    SortLettersUp,
    SortNumbersDown,
    SortNumbersUp,
    Spinner,
    Magic,
    Wrench,
    Dials,
    Docs,
}

pub fn icon<'a, Message>(ico: Icon) -> Element<'a, Message> {
    icon_with_codepoint(match ico {
        Icon::Undo => '\u{E801}',
        Icon::Redo => '\u{E800}',
        Icon::Open => '\u{F115}',
        Icon::New => '\u{E804}',
        Icon::Save => '\u{E803}',
        Icon::Close => '\u{E802}',
        Icon::Check => '\u{E805}',
        Icon::Error => '\u{E806}',
        Icon::Cogs => '\u{E807}',
        Icon::Edit => '\u{E808}',
        Icon::Download => '\u{E809}',
        Icon::Delete => '\u{E80A}',
        Icon::Add => '\u{E80B}',
        Icon::SortDown => '\u{F161}',
        Icon::SortUp => '\u{F160}',
        Icon::SortLettersDown => '\u{F15E}',
        Icon::SortLettersUp => '\u{F15D}',
        Icon::SortNumbersDown => '\u{F163}',
        Icon::SortNumbersUp => '\u{F163}',
        Icon::Spinner => '\u{F110}',
        Icon::Magic => '\u{F0D0}',
        Icon::Wrench => '\u{E80C}',
        Icon::Dials => '\u{F1DE}',
        Icon::Docs => '\u{F0C5}',
    })
}
