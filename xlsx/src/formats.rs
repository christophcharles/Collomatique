use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder};

fn border(level: u8) -> FormatBorder {
    match level {
        0 => FormatBorder::None,
        1 => FormatBorder::Thin,
        _ => FormatBorder::Medium,
    }
}

pub fn header(bg: Color) -> Format {
    Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Medium)
        .set_background_color(bg)
}

pub fn period_header(bg: Color) -> Format {
    header(bg)
}

pub fn week_header(left: u8, right: u8, bg: Color) -> Format {
    Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border_top(FormatBorder::Medium)
        .set_border_bottom(FormatBorder::Medium)
        .set_border_left(border(left))
        .set_border_right(border(right))
        .set_background_color(bg)
}

pub fn data_cell(top: u8, bottom: u8, left: u8, right: u8, bg: Color) -> Format {
    Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border_top(border(top))
        .set_border_bottom(border(bottom))
        .set_border_left(border(left))
        .set_border_right(border(right))
        .set_background_color(bg)
}

pub fn subject_cell(top: u8, bottom: u8, bg: Color) -> Format {
    data_cell(top, bottom, 2, 2, bg)
}

pub fn week_cell(top: u8, bottom: u8, left: u8, right: u8, bg: Color) -> Format {
    data_cell(top, bottom, left, right, bg)
}

pub fn empty_row(left: u8, right: u8, bg: Color) -> Format {
    Format::new()
        .set_border_top(FormatBorder::Medium)
        .set_border_bottom(FormatBorder::Medium)
        .set_border_left(border(left))
        .set_border_right(border(right))
        .set_background_color(bg)
}
