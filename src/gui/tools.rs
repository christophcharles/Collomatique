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

pub enum Icon {
    Undo,
    Redo,
    Open,
    New,
    SaveAs,
    Close,
}

pub fn icon<'a, Message>(ico: Icon) -> Element<'a, Message> {
    icon_with_codepoint(match ico {
        Icon::Undo => '\u{E801}',
        Icon::Redo => '\u{E800}',
        Icon::Open => '\u{F115}',
        Icon::New => '\u{E804}',
        Icon::SaveAs => '\u{E803}',
        Icon::Close => '\u{E802}',
    })
}
