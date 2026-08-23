#!/usr/bin/env python3

"""Exports a colloscope to xlsx, the way the application does.

This is the application's own export, written out in python: it reads the
document's export settings — the ones the export panel writes — and produces
the same workbook, sheet for sheet. `doc.export_xlsx(path)` does exactly this
in one call; this script exists so that you can change it.

The places worth changing:

  * `main()` reads the document's settings into `config` and hands them on.
    Override a field there — `config.colloscope_config.sheet_name`,
    `config.per_group_list_enabled` — and this script writes a different
    workbook without touching the document.
  * `write_xlsx` decides which sheets are written, and in what order.
  * the three `build_*_sheet` functions are one sheet each; the `fmt_*`
    helpers above them are every colour and border they use.

The script is one file on purpose: a script runs with nothing of its own
directory on `sys.path`, so there is no second module to import — which is
also why the french headings are written here rather than fetched from
somewhere.
"""

from __future__ import annotations

import dataclasses
import datetime
import sys

import collomatique as clm
import xlsxwriter


# --------------------------------------------------------------- constants


# The xlsx paper-size index for A4 (210 × 297 mm).
PAPER_A4 = 9

# A group sheet with at least this many group lists is printed landscape, when
# the settings leave the orientation to be decided.
AUTO_LANDSCAPE_GROUP_LIST_THRESHOLD = 4

# The six characters excel refuses in a sheet name.
FORBIDDEN_SHEET_NAME_CHARS = "[]:*?/\\"

# The application's own name for a day, as `str(SlotStart)` used to build it:
# the capitalized french weekday, then the time as « 14h00 ».
WEEKDAY_NAMES = {
    clm.Weekday.MONDAY: "Lundi",
    clm.Weekday.TUESDAY: "Mardi",
    clm.Weekday.WEDNESDAY: "Mercredi",
    clm.Weekday.THURSDAY: "Jeudi",
    clm.Weekday.FRIDAY: "Vendredi",
    clm.Weekday.SATURDAY: "Samedi",
    clm.Weekday.SUNDAY: "Dimanche",
}


def format_slot_start(slot):
    """Render a slot's day and time the way the application does."""
    return "{} {}".format(WEEKDAY_NAMES[slot.weekday],
                          slot.start_time.strftime("%Hh%M"))


# ----------------------------------------------------- colours and formats


def color_hex(color):
    """A `clm.Color` as xlsxwriter spells one."""
    return "#{:02X}{:02X}{:02X}".format(color.red, color.green, color.blue)


class Formats:
    """The workbook's cell formats, built once each.

    The application builds a fresh format for every cell it writes and lets
    its writer fold the duplicates away when it saves. `xlsxwriter` has no
    such folding: a workbook of a few thousand cells would carry a few
    thousand format records. So the properties are the key, and a format is
    built the first time it is asked for.
    """

    def __init__(self, workbook):
        self._workbook = workbook
        self._built = {}

    def get(self, **properties):
        key = tuple(sorted(properties.items()))
        fmt = self._built.get(key)
        if fmt is None:
            fmt = self._workbook.add_format(properties)
            self._built[key] = fmt
        return fmt


def _fill(background):
    """A solid fill of one colour.

    In xlsxwriter `bg_color` is the *background* of a pattern, and on its own
    it paints nothing. A solid fill is pattern 1 with the colour in the
    foreground.
    """
    return {"pattern": 1, "fg_color": background}


# The border levels below are the numbers xlsxwriter itself uses: 0 none,
# 1 thin, 2 medium. They are the same numbers the application's own export
# encodes, so a border level is passed straight through as a property.


def fmt_header(formats, background):
    """A heading, ruled medium all round. Also the period titles."""
    return formats.get(bold=True, align="center", valign="vcenter", border=2,
                       **_fill(background))


def fmt_header_cell(formats, left, right, background):
    """A heading inside a band: medium above and below, its own sideways."""
    return formats.get(bold=True, align="center", valign="vcenter",
                       top=2, bottom=2, left=left, right=right,
                       **_fill(background))


def fmt_week_dates(formats, left, right, background):
    """A week's date range, written sideways in small type."""
    return formats.get(align="center", valign="vcenter", rotation=90,
                       top=2, bottom=2, left=left, right=right,
                       font_size=8, **_fill(background))


def fmt_data_cell(formats, top, bottom, left, right, background):
    """A cell of the body, with a border level per side."""
    return formats.get(align="center", valign="vcenter",
                       top=top, bottom=bottom, left=left, right=right,
                       **_fill(background))


def fmt_annotation(formats, background):
    """A week's annotation, written sideways under the grid, unruled."""
    return formats.get(rotation=90, align="center", valign="top",
                       **_fill(background))


def fmt_empty_row(formats, left, right, background):
    """The blank ruled row that separates two subjects."""
    return formats.get(top=2, bottom=2, left=left, right=right,
                       **_fill(background))


# --------------------------------------------------------------- settings


# The document stores each `*_enabled` flag beside the value it gates, so that
# the interface remembers what was chosen before a section was switched off.
# What a sheet builder wants is the other shape: `None` where a section is
# off, so that a disabled value cannot be read by accident. `resolve` below
# turns the first into the second, and turns every colour into the string
# xlsxwriter wants, so no builder ever sees a `clm.Color`.


@dataclasses.dataclass
class GlobalSettings:
    background_color: str
    stripes_color: str


@dataclasses.dataclass
class ColloscopeSettings:
    sheet_name: str
    extra_info_column_name: str | None
    teacher_email: str | None
    teacher_tel: str | None
    orientation: object
    display_week_dates: bool
    display_annotations: bool
    no_interrogation_color: str
    annotation_color: str | None
    extra_colors: dict


@dataclasses.dataclass
class StudentGroupsSettings:
    sheet_name: str
    orientation: object | None      # None means « decide from the group count »
    show_emails: bool
    show_tel: bool


@dataclasses.dataclass
class GroupListSettings:
    orientation: object
    show_emails: bool
    show_tel: bool
    center_vertically: bool


@dataclasses.dataclass
class Settings:
    globals: GlobalSettings
    colloscope: ColloscopeSettings | None
    all_groups: StudentGroupsSettings | None
    automatic_groups: StudentGroupsSettings | None
    prefilled_groups: StudentGroupsSettings | None
    per_group_list: GroupListSettings | None


def optional(enabled, value):
    """The value when its toggle is on, `None` when it is off."""
    return value if enabled else None


def colloscope_settings(config):
    return ColloscopeSettings(
        sheet_name=config.sheet_name,
        extra_info_column_name=optional(config.extra_info_column_enabled,
                                        config.extra_info_column_name),
        teacher_email=optional(config.teacher_email_enabled,
                               config.teacher_email),
        teacher_tel=optional(config.teacher_tel_enabled, config.teacher_tel),
        orientation=config.orientation,
        display_week_dates=config.display_week_dates,
        display_annotations=config.display_annotations,
        no_interrogation_color=color_hex(config.no_interrogation_color),
        annotation_color=optional(
            config.annotation_color_enabled,
            color_hex(config.annotation_color)),
        extra_colors={text: color_hex(color)
                      for text, color in config.extra_colors.items()},
    )


def student_groups_settings(config):
    return StudentGroupsSettings(
        sheet_name=config.sheet_name,
        orientation=config.orientation,
        show_emails=config.show_emails,
        show_tel=config.show_tel,
    )


def group_list_settings(config):
    return GroupListSettings(
        orientation=config.orientation,
        show_emails=config.show_emails,
        show_tel=config.show_tel,
        center_vertically=config.center_vertically,
    )


def resolve(config):
    """The document's export settings, with the disabled halves cut away."""
    global_config = config.global_config
    background = color_hex(global_config.background_color)

    return Settings(
        globals=GlobalSettings(
            background_color=background,
            # Stripes that are switched off are stripes painted in the colour
            # the sheet already is: they simply disappear.
            stripes_color=(color_hex(global_config.stripes_color)
                           if global_config.stripes_color_enabled
                           else background),
        ),
        colloscope=(colloscope_settings(config.colloscope_config)
                    if config.colloscope_enabled else None),
        all_groups=(student_groups_settings(config.all_groups_config)
                    if config.all_groups_enabled else None),
        automatic_groups=(student_groups_settings(config.automatic_groups_config)
                          if config.automatic_groups_enabled else None),
        prefilled_groups=(student_groups_settings(config.prefilled_groups_config)
                          if config.prefilled_groups_enabled else None),
        per_group_list=(group_list_settings(config.per_group_list_config)
                        if config.per_group_list_enabled else None),
    )


# ---------------------------------------------------------------- helpers


def sanitize_sheet_name(name):
    """The name a sheet can carry, or `None` for « keep the default ».

    Excel refuses six characters outright and stops at 31, and a name that is
    only spaces or quotes is no name at all.
    """
    sanitized = "".join("-" if c in FORBIDDEN_SHEET_NAME_CHARS else c
                        for c in name)
    trimmed = sanitized.strip().strip("'")
    if not trimmed:
        return None
    return trimmed[:31]


def add_sheet(workbook, name):
    """Adds a sheet under the name it can actually carry.

    A name excel would refuse outright leaves the workbook's own « Sheet1 ».
    Two sheets that end up with the same name still raise.
    """
    safe_name = sanitize_sheet_name(name)
    if safe_name is None:
        return workbook.add_worksheet()
    return workbook.add_worksheet(safe_name)


def generate_period_title(first_week, period_index, first_week_num,
                          week_count):
    """« Période 2 du 05/01/2026 au 09/02/2026 », or « Période 2 » undated."""
    if first_week is None:
        return "Période {}".format(period_index + 1)
    start = first_week + datetime.timedelta(days=7 * first_week_num)
    end = start + datetime.timedelta(days=7 * week_count - 1)
    return "Période {} du {} au {}".format(
        period_index + 1, start.strftime("%d/%m/%Y"), end.strftime("%d/%m/%Y"))


def generate_week_dates_title(monday):
    """«   Du 05/01/2026 au 11/01/2026  » — the padding spaces are wanted.

    `week.monday` is already the start date plus seven days per week of global
    order, which is exactly how the application dates its own export.
    """
    end = monday + datetime.timedelta(days=6)
    return "  Du {} au {}  ".format(monday.strftime("%d/%m/%Y"),
                                    end.strftime("%d/%m/%Y"))


def get_group_name(group_names, group_num):
    """Resolve a group number to its display name."""
    # `group_list.group_name(group_num)` would answer « Groupe 3 » here. The
    # colloscope sheet gives a group column five characters wide, so the bare
    # number is what fits — hence this fallback rather than the api's.
    if group_num < len(group_names) and group_names[group_num]:
        return group_names[group_num]
    return str(group_num + 1)


def vertical_borders(index, count):
    """(top, bottom) of one row in a block: medium outside, thin inside."""
    return (2 if index == 0 else 1, 2 if index == count - 1 else 1)


def side_borders(index, count):
    """(left, right) of one column in a band — the same rule, sideways."""
    return (2 if index == 0 else 1, 2 if index == count - 1 else 1)


def non_empty_group_lists_by_name(doc):
    """The group lists that hold at least one student, sorted by name.

    Either the solver placed students in it — `doc.colloscope.group_list`
    answers a mapping rather than `None` — or it is prefilled and one of its
    groups is not empty.
    """
    found = [
        group_list for group_list in doc.group_lists
        if doc.colloscope.group_list(group_list) is not None
        or any(group for group in (group_list.groups or ()))
    ]
    found.sort(key=lambda group_list: group_list.name)
    return found


def write_mailto(worksheet, row, col, address, fmt):
    """One address as a link, or a blank cell when there is none."""
    if not address:
        worksheet.write(row, col, "", fmt)
    else:
        worksheet.write_url(row, col, "mailto:{}".format(address), fmt,
                            address)


def write_or_merge(worksheet, first_row, first_col, last_row, last_col, text,
                   fmt):
    """One value over a range — written plain when the range is one cell.

    `merge_range` refuses a range of a single cell, so that case is written
    out.
    """
    if (first_row, first_col) == (last_row, last_col):
        worksheet.write(first_row, first_col, text, fmt)
    else:
        worksheet.merge_range(first_row, first_col, last_row, last_col, text,
                              fmt)


# -------------------------------------------------------------- page setup


def apply_page_setup(worksheet, orientation, center_vertically=True):
    """A4, fitted to one page, centered — as every sheet of the export is."""
    worksheet.set_paper(PAPER_A4)
    worksheet.center_horizontally()
    # `center_vertically()` takes no argument and off is the default, hence
    # the call rather than a flag.
    if center_vertically:
        worksheet.center_vertically()
    worksheet.fit_to_pages(1, 1)
    if orientation == clm.Orientation.LANDSCAPE:
        worksheet.set_landscape()
    else:
        worksheet.set_portrait()


def auto_orientation(orientation, group_list_count):
    """The orientation of a per-student-groups sheet.

    `None` in the settings means « decide from the group lists »: a sheet with
    enough columns to need it is printed landscape.
    """
    if orientation is not None:
        return orientation
    if group_list_count >= AUTO_LANDSCAPE_GROUP_LIST_THRESHOLD:
        return clm.Orientation.LANDSCAPE
    return clm.Orientation.PORTRAIT


# --------------------------------------------------------- who is in what


def automatic_memberships(doc, memberships):
    """What the solver placed, list by list."""
    for group_list, placements in doc.colloscope.group_lists():
        for student, group_number in placements.items():
            memberships[(group_list, student)] = group_number


def prefilled_memberships(doc, memberships):
    """What the parameters fix by hand — a group's number is its position."""
    for group_list in doc.group_lists:
        for group_number, group in enumerate(group_list.groups or ()):
            for student in group:
                memberships[(group_list, student)] = group_number


def query_all(doc):
    """Every non-empty list, however it was filled."""
    memberships = {}
    automatic_memberships(doc, memberships)
    prefilled_memberships(doc, memberships)
    return non_empty_group_lists_by_name(doc), memberships


def query_automatic(doc):
    """The lists the solver filled."""
    memberships = {}
    automatic_memberships(doc, memberships)
    group_lists = sorted((group_list
                          for group_list, _ in doc.colloscope.group_lists()),
                         key=lambda group_list: group_list.name)
    return group_lists, memberships


def query_prefilled(doc):
    """The lists filled by hand."""
    memberships = {}
    prefilled_memberships(doc, memberships)
    group_lists = sorted(
        (group_list for group_list in doc.group_lists
         if any(group for group in (group_list.groups or ()))),
        key=lambda group_list: group_list.name)
    return group_lists, memberships


# ------------------------------------------------------------ the sheets


class FixedColumns:
    """Where the non-week columns sit, once the optional ones are known.

    Each optional column shifts the ones after it, so the positions are worked
    out once and read from here rather than counted at every write.
    """

    def __init__(self, settings):
        self.subject = 0
        self.teacher = 1
        next_col = 2

        self.email = None
        if settings.teacher_email is not None:
            self.email = next_col
            next_col += 1

        self.tel = None
        if settings.teacher_tel is not None:
            self.tel = next_col
            next_col += 1

        self.slot = next_col
        next_col += 1

        self.extra_info = None
        if settings.extra_info_column_name is not None:
            self.extra_info = next_col
            next_col += 1

        self.count = next_col


@dataclasses.dataclass
class PeriodLayout:
    period: object
    weeks: tuple
    col_start: int
    period_index: int
    first_week_num: int


def period_layout(doc, first_col):
    """The periods that have weeks, left to right.

    A period with no week is skipped entirely, numbering included: « Période 1 »
    is the first period that has weeks, not the first period.
    """
    layout = []
    col = first_col
    weeks_before = 0
    for period in doc.periods:
        weeks = period.weeks
        if not weeks:
            continue
        layout.append(PeriodLayout(period, weeks, col, len(layout),
                                   weeks_before))
        col += len(weeks)
        weeks_before += len(weeks)
    return layout


def week_background(settings, week, default_background):
    """The colour a week's column is painted, in the export's own order.

    1. an annotation that names one of the extra colours wins outright;
    2. then a week that holds no interrogation at all;
    3. then any annotation, when the annotation colour is on;
    4. otherwise whatever the row was already painted.
    """
    annotation = week.annotation
    if annotation is not None:
        extra = settings.extra_colors.get(annotation)
        if extra is not None:
            return extra
    if not week.interrogations:
        return settings.no_interrogation_color
    if annotation is not None and settings.annotation_color is not None:
        return settings.annotation_color
    return default_background


def build_colloscope_sheet(worksheet, formats, doc, global_settings, settings):
    """The colloscope itself: one row per slot, one column per week."""
    cols = FixedColumns(settings)
    layout = period_layout(doc, cols.count)
    week_col_count = sum(len(entry.weeks) for entry in layout)

    background = global_settings.background_color
    stripe = global_settings.stripes_color

    first_week = doc.periods.first_week
    show_week_dates = settings.display_week_dates and first_week is not None
    header_row_offset = 1 if show_week_dates else 0

    # -- Row 0: one title per period, over its weeks --
    for entry in layout:
        label = generate_period_title(first_week, entry.period_index,
                                      entry.first_week_num, len(entry.weeks))
        write_or_merge(worksheet, 0, entry.col_start,
                       0, entry.col_start + len(entry.weeks) - 1,
                       label, fmt_header(formats, background))

    # -- Row 1, when asked for: each week's date range, written sideways --
    if show_week_dates:
        for entry in layout:
            for index, week in enumerate(entry.weeks):
                left, right = side_borders(index, len(entry.weeks))
                fmt = fmt_week_dates(formats, left, right,
                                     week_background(settings, week,
                                                     background))
                worksheet.write(1, entry.col_start + index,
                                generate_week_dates_title(week.monday), fmt)

    # -- The heading row: the fixed columns, then S1, S2, … --
    header_row = 1 + header_row_offset
    header_fmt = fmt_header(formats, background)
    worksheet.write(header_row, cols.subject, "Matière", header_fmt)
    worksheet.write(header_row, cols.teacher, "Colleur", header_fmt)
    # A column exists exactly when its heading does, and an empty heading is a
    # heading someone chose — the model allows it — so it is written as it is.
    if cols.email is not None:
        worksheet.write(header_row, cols.email, settings.teacher_email,
                        header_fmt)
    if cols.tel is not None:
        worksheet.write(header_row, cols.tel, settings.teacher_tel, header_fmt)
    worksheet.write(header_row, cols.slot, "Créneau", header_fmt)
    if cols.extra_info is not None:
        worksheet.write(header_row, cols.extra_info,
                        settings.extra_info_column_name, header_fmt)

    week_number = 1
    for entry in layout:
        for index, week in enumerate(entry.weeks):
            left, right = side_borders(index, len(entry.weeks))
            fmt = fmt_header_cell(formats, left, right,
                                  week_background(settings, week, background))
            worksheet.write(header_row, entry.col_start + index,
                            "S{}".format(week_number), fmt)
            week_number += 1

    # -- The body --
    row = 2 + header_row_offset
    first_subject = True
    stripe_index = 0

    for subject in doc.subjects:
        slots = subject.slots
        if not slots:
            continue

        if not first_subject:
            # A blank ruled row between two subjects. It does not advance
            # `stripe_index`: the stripes count interrogation rows, not sheet
            # rows, so a subject boundary never flips them.
            for col in range(cols.count):
                worksheet.write(row, col, "",
                                fmt_empty_row(formats, 2, 2, background))
            for entry in layout:
                for index, week in enumerate(entry.weeks):
                    left, right = side_borders(index, len(entry.weeks))
                    fmt = fmt_empty_row(formats, left, right,
                                        week_background(settings, week,
                                                        background))
                    worksheet.write(row, entry.col_start + index, "", fmt)
            row += 1
        first_subject = False

        subject_start_row = row

        for slot_index, slot in enumerate(slots):
            teacher = slot.teacher
            top, bottom = vertical_borders(slot_index, len(slots))
            row_background = stripe if stripe_index % 2 == 0 else background
            data_fmt = fmt_data_cell(formats, top, bottom, 2, 2,
                                     row_background)

            worksheet.write(row, cols.teacher, teacher.surname, data_fmt)
            if cols.email is not None:
                write_mailto(worksheet, row, cols.email, teacher.email,
                             data_fmt)
            if cols.tel is not None:
                worksheet.write(row, cols.tel, teacher.tel or "", data_fmt)
            worksheet.write(row, cols.slot, format_slot_start(slot), data_fmt)
            if cols.extra_info is not None:
                worksheet.write(row, cols.extra_info, slot.extra_info,
                                data_fmt)

            for entry in layout:
                # Which group list this subject uses depends on the period.
                group_list = doc.group_lists.association_for(entry.period,
                                                             subject)
                names = (group_list.group_names
                         if group_list is not None else None)
                for index, week in enumerate(entry.weeks):
                    left, right = side_borders(index, len(entry.weeks))
                    fmt = fmt_data_cell(formats, top, bottom, left, right,
                                        week_background(settings, week,
                                                        row_background))
                    worksheet.write(row, entry.col_start + index,
                                    interrogation_text(doc, slot, week, names),
                                    fmt)

            stripe_index += 1
            row += 1

        write_or_merge(worksheet, subject_start_row, cols.subject,
                       row - 1, cols.subject, subject.name,
                       fmt_data_cell(formats, 2, 2, 2, 2, background))

    # -- Under the grid: each annotated week's own text, written sideways --
    if settings.display_annotations:
        for entry in layout:
            for index, week in enumerate(entry.weeks):
                if week.annotation is not None:
                    worksheet.write(row, entry.col_start + index,
                                    "{} ".format(week.annotation),
                                    fmt_annotation(formats, background))

    worksheet.set_column(cols.subject, cols.subject, 14)
    worksheet.set_column(cols.teacher, cols.teacher, 14)
    if cols.email is not None:
        worksheet.set_column(cols.email, cols.email, 22)
    if cols.tel is not None:
        worksheet.set_column(cols.tel, cols.tel, 14)
    worksheet.set_column(cols.slot, cols.slot, 14)
    if cols.extra_info is not None:
        worksheet.set_column(cols.extra_info, cols.extra_info, 10)
    if week_col_count:
        worksheet.set_column(cols.count, cols.count + week_col_count - 1, 5)


def interrogation_text(doc, slot, week, group_names):
    """« 3, 5 » — the groups interrogated on that slot that week, or « »."""
    # An interrogation nobody scheduled and one that could not be scheduled
    # are both `None` here, and both render as an empty cell.
    groups = doc.colloscope.interrogation(slot, week)
    if not groups:
        return ""
    if group_names is None:
        return ", ".join(str(group + 1) for group in sorted(groups))
    return ", ".join(get_group_name(group_names, group)
                     for group in sorted(groups))


def build_per_student_groups_sheet(worksheet, formats, doc, global_settings,
                                   show_emails, show_tel,
                                   group_lists, memberships):
    """One row per student, one column per group list."""
    background = global_settings.background_color
    stripe = global_settings.stripes_color

    # The two name columns read as one band: medium outside, thin between.
    header_fmt = fmt_header(formats, background)
    worksheet.write(0, 0, "Nom", fmt_header_cell(formats, 2, 1, background))
    worksheet.write(0, 1, "Prénom", fmt_header_cell(formats, 1, 2, background))
    col = 2
    if show_emails:
        worksheet.write(0, col, "Courriel", header_fmt)
        col += 1
    if show_tel:
        worksheet.write(0, col, "Téléphone", header_fmt)
        col += 1
    first_list_col = col

    for index, group_list in enumerate(group_lists):
        left, right = side_borders(index, len(group_lists))
        worksheet.write(0, first_list_col + index, group_list.name,
                        fmt_header_cell(formats, left, right, background))

    students = sorted(doc.students,
                      key=lambda student: (student.surname, student.firstname))

    for row_index, student in enumerate(students):
        row = row_index + 1
        top, bottom = vertical_borders(row_index, len(students))
        row_background = stripe if row_index % 2 == 0 else background

        worksheet.write(row, 0, student.surname,
                        fmt_data_cell(formats, top, bottom, 2, 1,
                                      row_background))
        worksheet.write(row, 1, student.firstname,
                        fmt_data_cell(formats, top, bottom, 1, 2,
                                      row_background))
        contact_fmt = fmt_data_cell(formats, top, bottom, 2, 2, row_background)
        col = 2
        if show_emails:
            write_mailto(worksheet, row, col, student.email, contact_fmt)
            col += 1
        if show_tel:
            worksheet.write(row, col, student.tel or "", contact_fmt)
            col += 1

        for index, group_list in enumerate(group_lists):
            left, right = side_borders(index, len(group_lists))
            fmt = fmt_data_cell(formats, top, bottom, left, right,
                                row_background)
            group_number = memberships.get((group_list, student))
            text = ("" if group_number is None
                    else get_group_name(group_list.group_names, group_number))
            worksheet.write(row, first_list_col + index, text, fmt)

    worksheet.set_column(0, 0, 16)
    worksheet.set_column(1, 1, 14)
    col = 2
    if show_emails:
        worksheet.set_column(col, col, 24)
        col += 1
    if show_tel:
        worksheet.set_column(col, col, 14)
        col += 1
    if group_lists:
        worksheet.set_column(col, col + len(group_lists) - 1, 12)


def build_per_group_list_sheet(worksheet, formats, doc, global_settings,
                               group_list, settings):
    """One group list: its groups, and who is in each."""
    background = global_settings.background_color
    stripe = global_settings.stripes_color
    show_emails = settings.show_emails
    show_tel = settings.show_tel

    # Members come from both sources — what the solver placed, and what the
    # parameters fix by hand. A list is one kind or the other, so the two
    # never disagree; reading both is how one function serves both kinds.
    groups = {}
    placements = doc.colloscope.group_list(group_list)
    if placements is not None:
        for student, group_number in placements.items():
            groups.setdefault(group_number, []).append(student)
    for group_number, group in enumerate(group_list.groups or ()):
        for student in group:
            groups.setdefault(group_number, []).append(student)
    for members in groups.values():
        members.sort(key=lambda student: (student.surname, student.firstname))

    headings = ["Groupe", "Nom", "Prénom"]
    if show_emails:
        headings.append("Courriel")
    if show_tel:
        headings.append("Téléphone")

    header_fmt = fmt_header(formats, background)
    write_or_merge(worksheet, 0, 0, 0, len(headings) - 1, group_list.name,
                   header_fmt)
    for col, name in enumerate(headings):
        worksheet.write(1, col, name, header_fmt)

    row = 2
    for display_index, group_number in enumerate(sorted(groups)):
        members = groups[group_number]
        # The stripe alternates per *group* here, not per row: a group reads
        # as one block.
        row_background = stripe if display_index % 2 == 0 else background
        name = get_group_name(group_list.group_names, group_number)
        group_fmt = fmt_data_cell(formats, 2, 2, 2, 2, row_background)
        write_or_merge(worksheet, row, 0, row + len(members) - 1, 0, name,
                       group_fmt)

        for member_index, student in enumerate(members):
            # Medium at the edges of a group, thin between its students.
            top, bottom = vertical_borders(member_index, len(members))
            data_fmt = fmt_data_cell(formats, top, bottom, 2, 2,
                                     row_background)
            worksheet.write(row, 1, student.surname, data_fmt)
            worksheet.write(row, 2, student.firstname, data_fmt)
            col = 3
            if show_emails:
                write_mailto(worksheet, row, col, student.email, data_fmt)
                col += 1
            if show_tel:
                worksheet.write(row, col, student.tel or "", data_fmt)
                col += 1
            row += 1

    worksheet.set_column(0, 0, 14)
    worksheet.set_column(1, 1, 16)
    worksheet.set_column(2, 2, 14)
    col = 3
    if show_emails:
        worksheet.set_column(col, col, 24)
        col += 1
    if show_tel:
        worksheet.set_column(col, col, 14)


# ------------------------------------------------------------ the export


def add_student_groups_sheet(workbook, formats, doc, global_settings, settings,
                             query):
    """One of the three per-student-groups sheets, page setup and all."""
    group_lists, memberships = query
    worksheet = add_sheet(workbook, settings.sheet_name)
    build_per_student_groups_sheet(worksheet, formats, doc, global_settings,
                                   settings.show_emails, settings.show_tel,
                                   group_lists, memberships)
    apply_page_setup(worksheet,
                     auto_orientation(settings.orientation, len(group_lists)))


def write_xlsx(doc, path, config):
    """Writes the whole workbook the settings describe."""
    settings = resolve(config)
    workbook = xlsxwriter.Workbook(str(path))
    formats = Formats(workbook)

    if settings.colloscope is not None:
        worksheet = add_sheet(workbook, settings.colloscope.sheet_name)
        build_colloscope_sheet(worksheet, formats, doc, settings.globals,
                               settings.colloscope)
        apply_page_setup(worksheet, settings.colloscope.orientation)

    if settings.all_groups is not None:
        add_student_groups_sheet(workbook, formats, doc, settings.globals,
                                 settings.all_groups, query_all(doc))

    # The automatic and prefilled sheets ask about the *lists*, not about the
    # colloscope: a document with no automatic list at all gets no automatic
    # sheet, but one whose lists were never filled still gets an empty one.
    if settings.automatic_groups is not None:
        if any(not gl.is_prefilled for gl in doc.group_lists):
            add_student_groups_sheet(workbook, formats, doc, settings.globals,
                                     settings.automatic_groups,
                                     query_automatic(doc))

    if settings.prefilled_groups is not None:
        if any(gl.is_prefilled for gl in doc.group_lists):
            add_student_groups_sheet(workbook, formats, doc, settings.globals,
                                     settings.prefilled_groups,
                                     query_prefilled(doc))

    if settings.per_group_list is not None:
        for group_list in non_empty_group_lists_by_name(doc):
            worksheet = add_sheet(workbook, group_list.name)
            build_per_group_list_sheet(worksheet, formats, doc,
                                       settings.globals, group_list,
                                       settings.per_group_list)
            apply_page_setup(worksheet, settings.per_group_list.orientation,
                             settings.per_group_list.center_vertically)

    workbook.close()


def main():
    doc = clm.default_document(sys.argv[1] if len(sys.argv) > 1 else None)

    # The document's own export settings — the ones the export panel writes,
    # and the ones `doc.export_xlsx(path)` would use. Nothing here writes them
    # back: change a field and this run writes a different workbook, the
    # document keeps what it had.
    config = doc.export_config.to_data()

    path = clm.dialogs.save_file(
        title="Exporter en XLSX",
        filters=[("Fichiers XLSX", ["xlsx"])],
        file_name="colloscope.xlsx",
    )
    if path is None:
        return

    # Nothing here writes to the document, so there is no transaction and no
    # save: this reads a colloscope and builds a workbook of its own.
    write_xlsx(doc, path, config)
    print("Export XLSX terminé : {}".format(path))


main()
