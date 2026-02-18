use rust_xlsxwriter::{Format, FormatAlign, FormatBorder};

fn border(level: u8) -> FormatBorder {
    match level {
        0 => FormatBorder::None,
        1 => FormatBorder::Thin,
        _ => FormatBorder::Medium,
    }
}

pub fn header() -> Format {
    Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Medium)
}

pub fn period_header() -> Format {
    header()
}

pub fn week_header(left: u8, right: u8) -> Format {
    Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border_top(FormatBorder::Medium)
        .set_border_bottom(FormatBorder::Medium)
        .set_border_left(border(left))
        .set_border_right(border(right))
}

pub fn data_cell(top: u8, bottom: u8, left: u8, right: u8) -> Format {
    Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_border_top(border(top))
        .set_border_bottom(border(bottom))
        .set_border_left(border(left))
        .set_border_right(border(right))
}

pub fn subject_cell(top: u8, bottom: u8) -> Format {
    data_cell(top, bottom, 2, 2)
}

pub fn week_cell(top: u8, bottom: u8, left: u8, right: u8) -> Format {
    data_cell(top, bottom, left, right)
}

pub fn empty_row(left: u8, right: u8) -> Format {
    Format::new()
        .set_border_top(FormatBorder::Medium)
        .set_border_bottom(FormatBorder::Medium)
        .set_border_left(border(left))
        .set_border_right(border(right))
}
