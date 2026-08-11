#!/usr/bin/env python3

import collomatique_old as collomatique
import xlsxwriter


def get_group_name(group_list_params, group_num):
    """Resolve a group number to its display name."""
    group_names = group_list_params.group_names
    if group_num < len(group_names) and group_names[group_num] is not None:
        return group_names[group_num]
    return str(group_num + 1)


def build_colloscope_sheet(workbook, params, colloscope):
    worksheet = workbook.add_worksheet("Colloscope")
    worksheet.set_landscape()

    periods = params.periods

    # Compute total week columns and period layout
    period_layout = []  # list of (period, col_start, num_weeks)
    col_offset = 5  # first 5 columns: Matière, Colleur, Contact, Créneau, Salle
    for period in periods:
        num_weeks = len(period.weeks_status)
        period_layout.append((period, col_offset, num_weeks))
        col_offset += num_weeks

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
    for period, p_col_start, num_weeks in period_layout:
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
    for period, p_col_start, num_weeks in period_layout:
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

    for subject in params.subjects:
        subject_id = subject.id
        subject_name = subject.parameters.name

        # Get slots for this subject
        slots_for_subject = params.slots.get(subject_id)
        if slots_for_subject is None:
            continue
        slots = list(slots_for_subject)
        if len(slots) == 0:
            continue

        # Separator row between subjects
        if not first_subject:
            for c in range(5):
                worksheet.write(row, c, "", empty_row_fmt)
            week_col = 5
            for _period, _p_col_start, num_weeks in period_layout:
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
            sp = slot.parameters

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

            # Teacher info
            teacher = params.teachers.get(sp.teacher_id)
            if teacher is not None:
                surname = teacher.desc.surname
                contact = teacher.desc.email if teacher.desc.email else teacher.desc.tel
            else:
                surname = ""
                contact = ""

            # Slot time and room
            slot_time = str(sp.start_time)
            room = sp.extra_info

            data_fmt = make_data_fmt(top=top_b, bottom=bot_b)
            worksheet.write(row, 1, surname, data_fmt)
            worksheet.write(row, 2, contact, data_fmt)
            worksheet.write(row, 3, slot_time, data_fmt)
            worksheet.write(row, 4, room, data_fmt)

            # Week columns
            for period, p_col_start, num_weeks in period_layout:
                period_id = period.id

                # Find group_list_id for this subject in this period
                period_assocs = params.group_lists_associations.get(period_id)
                group_list_id = None
                if period_assocs is not None:
                    group_list_id = period_assocs.get(subject_id)

                group_list_params = None
                if group_list_id is not None:
                    gl = params.group_lists.get(group_list_id)
                    if gl is not None:
                        group_list_params = gl.parameters

                # Find colloscope slot data
                colloscope_period = colloscope.period_map.get(period_id)
                colloscope_slot = None
                if colloscope_period is not None:
                    colloscope_slot = colloscope_period.slot_map.get(slot.id)

                for w in range(num_weeks):
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

                    cell_text = ""
                    if colloscope_slot is not None:
                        interrogations = colloscope_slot.interrogations
                        if w < len(interrogations) and interrogations[w] is not None:
                            interrog = interrogations[w]
                            group_names = []
                            for g in sorted(interrog.assigned_groups):
                                if group_list_params is not None:
                                    group_names.append(
                                        get_group_name(group_list_params, g))
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


def build_groups_sheet(workbook, params, colloscope):
    worksheet = workbook.add_worksheet("Groupes")

    # Collect group list IDs that have student assignments
    group_list_ids = []
    for gl_id in colloscope.group_lists:
        group_list_ids.append(gl_id)

    if len(group_list_ids) == 0:
        return

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

    gl_count = len(group_list_ids)
    for i, gl_id in enumerate(group_list_ids):
        gl = params.group_lists.get(gl_id)
        name = gl.parameters.name if gl is not None else ""
        worksheet.write(0, 4 + i, name, header_fmt)

    # -- Collect and sort students --
    students_sorted = sorted(params.students.items(),
                             key=lambda item: (item[1].desc.surname, item[1].desc.firstname))

    student_count = len(students_sorted)
    for row_idx, (student_id, student) in enumerate(students_sorted):
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
        worksheet.write(row, 0, student.desc.surname, data_fmt)
        worksheet.write(row, 1, student.desc.firstname, data_fmt)
        worksheet.write(row, 2, student.desc.email, data_fmt)
        worksheet.write(row, 3, student.desc.tel, data_fmt)

        for i, gl_id in enumerate(group_list_ids):
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

            colloscope_gl = colloscope.group_lists.get(gl_id)
            cell_text = ""
            if colloscope_gl is not None:
                group_num = colloscope_gl.groups_for_students.get(student_id)
                if group_num is not None:
                    gl = params.group_lists.get(gl_id)
                    if gl is not None:
                        cell_text = get_group_name(gl.parameters, group_num)
                    else:
                        cell_text = str(group_num + 1)

            worksheet.write(row, col, cell_text, cell_fmt)

    # Column widths
    worksheet.set_column(0, 0, 16)
    worksheet.set_column(1, 1, 14)
    worksheet.set_column(2, 2, 24)
    worksheet.set_column(3, 3, 14)
    worksheet.set_column(4, 4 + gl_count - 1, 12)


def main():
    session = collomatique.current_session()
    file = session.get_current_collomatique_file()

    path = session.dialog_save_file(
        "Exporter en XLSX",
        [("Fichiers XLSX", "xlsx")],
        "colloscope.xlsx",
    )
    if path is None:
        return

    params = file.get_main_params()
    colloscope = file.get_colloscope()

    workbook = xlsxwriter.Workbook(str(path))

    build_colloscope_sheet(workbook, params, colloscope)
    build_groups_sheet(workbook, params, colloscope)

    workbook.close()
    collomatique.log("Export XLSX terminé : {}".format(path))


main()
