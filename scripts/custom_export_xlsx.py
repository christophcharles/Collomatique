#!/usr/bin/env python3

import collomatique as clm
import sys
import xlsxwriter


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


def get_group_name(group_list, group_num):
    """Resolve a group number to its display name."""
    # `group_list.group_name(group_num)` would answer « Groupe 3 » here. This
    # sheet gives a group column five characters wide, so the bare number is
    # what fits — hence the script's own fallback rather than the api's.
    group_names = group_list.group_names
    if group_num < len(group_names) and group_names[group_num] is not None:
        return group_names[group_num]
    return str(group_num + 1)


def build_colloscope_sheet(workbook, doc):
    worksheet = workbook.add_worksheet("Colloscope")
    worksheet.set_landscape()

    # Compute total week columns and period layout
    period_layout = []  # list of (period, col_start, weeks)
    col_offset = 5  # first 5 columns: Matière, Colleur, Contact, Créneau, Salle
    for period in doc.periods:
        weeks = period.weeks
        period_layout.append((period, col_offset, weeks))
        col_offset += len(weeks)

    total_week_cols = col_offset - 5
    if total_week_cols == 0:
        return

    # -- Formats --
    header_fmt = workbook.add_format({
        "bold": True,
        "align": "center",
        "valign": "vcenter",
        "border": 2,
    })
    period_header_fmt = workbook.add_format({
        "bold": True,
        "align": "center",
        "valign": "vcenter",
        "top": 2,
        "bottom": 2,
        "left": 2,
        "right": 2,
    })
    week_header_fmt = workbook.add_format({
        "bold": True,
        "align": "center",
        "valign": "vcenter",
        "top": 2,
        "bottom": 2,
        "left": 1,
        "right": 1,
    })
    week_header_first_fmt = workbook.add_format({
        "bold": True,
        "align": "center",
        "valign": "vcenter",
        "top": 2,
        "bottom": 2,
        "left": 2,
        "right": 1,
    })
    week_header_last_fmt = workbook.add_format({
        "bold": True,
        "align": "center",
        "valign": "vcenter",
        "top": 2,
        "bottom": 2,
        "left": 1,
        "right": 2,
    })

    def make_data_fmt(top=1, bottom=1, left=2, right=2):
        return workbook.add_format({
            "align": "center",
            "valign": "vcenter",
            "top": top,
            "bottom": bottom,
            "left": left,
            "right": right,
        })

    def make_subject_fmt(top=2, bottom=2):
        return workbook.add_format({
            "align": "center",
            "valign": "vcenter",
            "top": top,
            "bottom": bottom,
            "left": 2,
            "right": 2,
        })

    # Pre-build week cell formats for each vertical position
    # (top_border, bottom_border) pairs
    def make_week_fmt(top=1, bottom=1, left=1, right=1):
        return workbook.add_format({
            "align": "center",
            "valign": "vcenter",
            "top": top,
            "bottom": bottom,
            "left": left,
            "right": right,
        })

    empty_row_fmt = workbook.add_format({
        "top": 2,
        "bottom": 2,
        "left": 2,
        "right": 2,
    })
    empty_row_week_fmt = workbook.add_format({
        "top": 2,
        "bottom": 2,
        "left": 1,
        "right": 1,
    })
    empty_row_week_first_fmt = workbook.add_format({
        "top": 2,
        "bottom": 2,
        "left": 2,
        "right": 1,
    })
    empty_row_week_last_fmt = workbook.add_format({
        "top": 2,
        "bottom": 2,
        "left": 1,
        "right": 2,
    })

    # -- Row 0: Period labels --
    # Leave first 5 cols empty in row 0
    for period, p_col_start, weeks in period_layout:
        num_weeks = len(weeks)
        if num_weeks == 1:
            worksheet.write(0, p_col_start, "Période", period_header_fmt)
        else:
            worksheet.merge_range(0, p_col_start, 0, p_col_start + num_weeks - 1,
                                  "Période", period_header_fmt)

    # -- Row 1: Fixed headers + week numbers --
    fixed_headers = ["Matière", "Colleur", "Contact", "Créneau", "Salle"]
    for col, name in enumerate(fixed_headers):
        worksheet.write(1, col, name, header_fmt)

    week_counter = 1
    for period, p_col_start, weeks in period_layout:
        num_weeks = len(weeks)
        for w in range(num_weeks):
            col = p_col_start + w
            if num_weeks == 1:
                fmt = week_header_fmt
            elif w == 0:
                fmt = week_header_first_fmt
            elif w == num_weeks - 1:
                fmt = week_header_last_fmt
            else:
                fmt = week_header_fmt
            worksheet.write(1, col, "S{}".format(week_counter), fmt)
            week_counter += 1

    # -- Data rows --
    row = 2
    first_subject = True

    for subject in doc.subjects:
        subject_name = subject.name

        # Get slots for this subject
        slots = list(subject.slots)
        if len(slots) == 0:
            continue

        # Separator row between subjects
        if not first_subject:
            for c in range(5):
                worksheet.write(row, c, "", empty_row_fmt)
            week_col = 5
            for _period, _p_col_start, weeks in period_layout:
                num_weeks = len(weeks)
                for w in range(num_weeks):
                    if num_weeks == 1:
                        fmt = empty_row_fmt
                    elif w == 0:
                        fmt = empty_row_week_first_fmt
                    elif w == num_weeks - 1:
                        fmt = empty_row_week_last_fmt
                    else:
                        fmt = empty_row_week_fmt
                    worksheet.write(row, week_col, "", fmt)
                    week_col += 1
            row += 1
        first_subject = False

        subject_start_row = row
        slot_count = len(slots)

        for slot_idx, slot in enumerate(slots):
            # Determine vertical border style for this slot row
            is_first = (slot_idx == 0)
            is_last = (slot_idx == slot_count - 1)
            if is_first and is_last:
                top_b, bot_b = 2, 2
            elif is_first:
                top_b, bot_b = 2, 1
            elif is_last:
                top_b, bot_b = 1, 2
            else:
                top_b, bot_b = 1, 1

            # Teacher info — a slot always names one, so there is no missing
            # case any more. A contact that is not filled in is `None` now,
            # rather than an empty string.
            teacher = slot.teacher
            surname = teacher.surname
            contact = teacher.email if teacher.email else (teacher.tel or "")

            # Slot time and room
            slot_time = format_slot_start(slot)
            room = slot.extra_info

            data_fmt = make_data_fmt(top=top_b, bottom=bot_b)
            worksheet.write(row, 1, surname, data_fmt)
            worksheet.write(row, 2, contact, data_fmt)
            worksheet.write(row, 3, slot_time, data_fmt)
            worksheet.write(row, 4, room, data_fmt)

            # Week columns
            for period, p_col_start, weeks in period_layout:
                num_weeks = len(weeks)

                # Find the group list this subject uses on this period
                group_list = doc.group_lists.association_for(period, subject)

                for w, week in enumerate(weeks):
                    col = p_col_start + w

                    # Determine left/right borders for week cells
                    if num_weeks == 1:
                        left_b, right_b = 2, 2
                    elif w == 0:
                        left_b, right_b = 2, 1
                    elif w == num_weeks - 1:
                        left_b, right_b = 1, 2
                    else:
                        left_b, right_b = 1, 1

                    cell_fmt = make_week_fmt(top=top_b, bottom=bot_b,
                                             left=left_b, right=right_b)

                    # An interrogation nobody scheduled and one that could not
                    # be scheduled are both `None` here, and both used to
                    # render as an empty cell anyway.
                    assigned_groups = doc.colloscope.interrogation(slot, week)
                    cell_text = ""
                    if assigned_groups:
                        group_names = []
                        for g in sorted(assigned_groups):
                            if group_list is not None:
                                group_names.append(
                                    get_group_name(group_list, g))
                            else:
                                group_names.append(str(g + 1))
                        cell_text = ", ".join(group_names)

                    worksheet.write(row, col, cell_text, cell_fmt)

            row += 1

        # Merge subject name vertically
        subject_end_row = row - 1
        subject_fmt = make_subject_fmt()
        if subject_start_row == subject_end_row:
            worksheet.write(subject_start_row, 0, subject_name, subject_fmt)
        else:
            worksheet.merge_range(subject_start_row, 0, subject_end_row, 0,
                                  subject_name, subject_fmt)

    # Auto-fit fixed columns
    worksheet.set_column(0, 0, 14)
    worksheet.set_column(1, 1, 14)
    worksheet.set_column(2, 2, 22)
    worksheet.set_column(3, 3, 14)
    worksheet.set_column(4, 4, 10)
    worksheet.set_column(5, 5 + total_week_cols - 1, 5)


def build_groups_sheet(workbook, doc):
    worksheet = workbook.add_worksheet("Groupes")

    # The lists the solver fills. A prefilled list carries its groups itself,
    # so it is not one of these — the old api hid the same ones.
    group_lists = [gl for gl in doc.group_lists if not gl.is_prefilled]

    if len(group_lists) == 0:
        return

    # Read once: a list that was never filled answers `None` rather than an
    # empty mapping.
    placements_per_list = [doc.colloscope.group_list(gl) for gl in group_lists]

    header_fmt = workbook.add_format({
        "bold": True,
        "align": "center",
        "valign": "vcenter",
        "border": 2,
    })

    def make_data_fmt(top=1, bottom=1, left=2, right=2):
        return workbook.add_format({
            "align": "center",
            "valign": "vcenter",
            "top": top,
            "bottom": bottom,
            "left": left,
            "right": right,
        })

    def make_gl_fmt(left=1, right=1, top=1, bottom=1):
        return workbook.add_format({
            "align": "center",
            "valign": "vcenter",
            "top": top,
            "bottom": bottom,
            "left": left,
            "right": right,
        })

    # -- Row 0: Headers --
    fixed_headers = ["Nom", "Prénom", "Courriel", "Téléphone"]
    for col, name in enumerate(fixed_headers):
        worksheet.write(0, col, name, header_fmt)

    gl_count = len(group_lists)
    for i, gl in enumerate(group_lists):
        worksheet.write(0, 4 + i, gl.name, header_fmt)

    # -- Collect and sort students --
    students_sorted = sorted(doc.students,
                             key=lambda student: (student.surname, student.firstname))

    student_count = len(students_sorted)
    for row_idx, student in enumerate(students_sorted):
        row = row_idx + 1

        is_first = (row_idx == 0)
        is_last = (row_idx == student_count - 1)
        if is_first and is_last:
            top_b, bot_b = 2, 2
        elif is_first:
            top_b, bot_b = 2, 1
        elif is_last:
            top_b, bot_b = 1, 2
        else:
            top_b, bot_b = 1, 1

        data_fmt = make_data_fmt(top=top_b, bottom=bot_b)
        worksheet.write(row, 0, student.surname, data_fmt)
        worksheet.write(row, 1, student.firstname, data_fmt)
        worksheet.write(row, 2, student.email or "", data_fmt)
        worksheet.write(row, 3, student.tel or "", data_fmt)

        for i, gl in enumerate(group_lists):
            col = 4 + i
            if gl_count == 1:
                left_b, right_b = 2, 2
            elif i == 0:
                left_b, right_b = 2, 1
            elif i == gl_count - 1:
                left_b, right_b = 1, 2
            else:
                left_b, right_b = 1, 1

            cell_fmt = make_gl_fmt(left=left_b, right=right_b,
                                   top=top_b, bottom=bot_b)

            placements = placements_per_list[i]
            cell_text = ""
            if placements is not None:
                group_num = placements.get(student)
                if group_num is not None:
                    cell_text = get_group_name(gl, group_num)

            worksheet.write(row, col, cell_text, cell_fmt)

    # Column widths
    worksheet.set_column(0, 0, 16)
    worksheet.set_column(1, 1, 14)
    worksheet.set_column(2, 2, 24)
    worksheet.set_column(3, 3, 14)
    worksheet.set_column(4, 4 + gl_count - 1, 12)


def main():
    doc = clm.default_document(sys.argv[1] if len(sys.argv) > 1 else None)

    path = clm.dialogs.save_file(
        title="Exporter en XLSX",
        filters=[("Fichiers XLSX", ["xlsx"])],
        file_name="colloscope.xlsx",
    )
    if path is None:
        return

    workbook = xlsxwriter.Workbook(str(path))

    # Nothing here writes to the document, so there is no transaction and no
    # save: this reads a colloscope and builds a workbook of its own.
    build_colloscope_sheet(workbook, doc)
    build_groups_sheet(workbook, doc)

    workbook.close()
    print("Export XLSX terminé : {}".format(path))


main()
