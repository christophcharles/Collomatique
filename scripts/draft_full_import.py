#!/usr/bin/env python3

import collomatique as clm
import csv
import datetime
import sys

CSV_FILTERS = [("Fichiers CSV", ["csv"]), ("Tous les fichiers", ["*"])]

def open_csv(file_path):
    csvfile = open(file_path, newline='')
    reader = csv.reader(csvfile, delimiter=';')

    output = []

    names = list(next(reader))
    for row in reader:
        new_line = {}
        for name,val in zip(names,row):
            if name in new_line:
                new_line[name].append(val)
            else:
                new_line[name] = [val]
        output.append(new_line)

    return names,output

def find_main_period(doc):
    # The old script asked for a number of weeks and always made a period. There
    # is no message box in the new api, so this follows
    # `import_pronote_web_2026_05_06.py` instead: reuse the document's first
    # period, and only make a default 10-week one when there is none at all.
    if len(doc.periods) == 0:
        return doc.periods.add(10).created
    return next(iter(doc.periods))

def find_subject_by_name(doc, subject_name):
    for subject in doc.subjects:
        if subject.name == subject_name:
            return subject
    return None

def find_group_list_by_name(doc, group_list_name):
    for group_list in doc.group_lists:
        if group_list.name == group_list_name:
            return group_list
    return None

def update_subjects_and_empty_group_lists(doc, main_period):
    print("Configuration des matières et des listes de groupes associées...")

    file_path = clm.dialogs.open_file(title="subjects.csv ?", filters=CSV_FILTERS)
    if file_path is None:
        raise Exception("subjects.csv est nécessaire pour le remplissage automatique")

    csv_columns, csv_content = open_csv(file_path)
    print("Parcours des matières...")
    for csv_line in csv_content:
        subject_name = csv_line["Matière"][0]
        print("- \"{}\"".format(subject_name))

        if find_subject_by_name(doc, subject_name) is not None:
            print("Matière déjà existante")
            continue

        duration = int(csv_line["Durée"][0])
        if duration == 0:
            interrogation = None
            group_list = None
        else:
            period = int(csv_line["Période"][0])
            strict_period = int(csv_line["Période stricte"][0])
            tutorial = int(csv_line["TD"][0])
            students_per_group_min = int(csv_line["Min élèves par groupe"][0])
            students_per_group_max = int(csv_line["Max élèves par groupe"][0])
            groups_per_interrogation_max = int(csv_line["Max nb groupes"][0])
            group_list_name = csv_line["Liste"][0]

            if strict_period:
                # `EveryNWeeks` is the old `ExactlyPeriodic` and `OncePerBlock`
                # the old `OnceForEveryBlockOfWeeks` — the same two model
                # variants, renamed for the public api.
                periodicity = clm.EveryNWeeks(period)
            else:
                periodicity = clm.OncePerBlock(period, 1)

            interrogation = clm.InterrogationData(
                students_per_group=(students_per_group_min, students_per_group_max),
                groups_per_interrogation=(1, groups_per_interrogation_max),
                duration=duration,
                take_duration_into_account=(tutorial == 0),
                periodicity=periodicity,
            )

            if group_list_name != "":
                group_list = find_group_list_by_name(doc, group_list_name)
                if group_list is not None:
                    # One `(min, max)` tuple now, rather than two fields.
                    if group_list.students_per_group != (students_per_group_min, students_per_group_max):
                        raise ValueError("La liste de groupes \"{}\" n'a pas le bon nombre d'élèves par groupe".format(group_list_name))
                else:
                    print("Nouvelle liste de groupe \"{}\"".format(group_list_name))

                    group_list = doc.group_lists.add(clm.GroupListData(
                        group_list_name,
                        students_per_group=(students_per_group_min, students_per_group_max),
                        # A placeholder `update_group_lists` replaces with the
                        # real names once groups.csv says how many groups there
                        # are. `None` and not "" — an empty string is refused as
                        # a stored group name.
                        group_names=[None for i in range(255)],
                    )).created
            else:
                group_list = None

        subject = doc.subjects.add(
            clm.SubjectData(subject_name, interrogation=interrogation)
        ).created

        # The old script cleared the association for every subject, including
        # the ones with no colles. `set_association` refuses a subject that
        # holds no interrogation — even to clear the row — so the call only
        # happens where there really is a list to attach.
        if group_list is not None:
            doc.group_lists.set_association(main_period, subject, group_list)

def find_week_pattern_by_name(doc, week_pattern_name):
    for week_pattern in doc.week_patterns:
        if week_pattern.name == week_pattern_name:
            return week_pattern
    return None

def convert_week_pattern_name(doc, week_pattern_name):
    if week_pattern_name == "":
        return None
    else:
        week_pattern = find_week_pattern_by_name(doc, week_pattern_name)
        if week_pattern is None:
            # A pattern is stored as the weeks it *excludes*, so one that leaves
            # every week alone needs no week count — which is why nothing here
            # has to ask the document how long the year is.
            week_pattern = doc.week_patterns.add(
                clm.WeekPatternData(week_pattern_name)
            ).created
        return week_pattern

def find_teacher_by_name(doc, firstname, surname):
    for teacher in doc.teachers:
        if teacher.firstname == firstname and teacher.surname == surname:
            return teacher
    return None

def to_collomatique_day(day):
    match day:
        case 'Lundi':
            return clm.Weekday.MONDAY
        case 'Mardi':
            return clm.Weekday.TUESDAY
        case 'Mercredi':
            return clm.Weekday.WEDNESDAY
        case 'Jeudi':
            return clm.Weekday.THURSDAY
        case 'Vendredi':
            return clm.Weekday.FRIDAY
        case 'Samedi':
            return clm.Weekday.SATURDAY
        case 'Dimanche':
            return clm.Weekday.SUNDAY
        case _:
            raise ValueError("Jour inconnu : \"{}\"".format(day))

def update_timeslots_and_teachers(doc):
    print("Configuration des créneaux de colles et des colleurs associés...")

    file_path = clm.dialogs.open_file(title="timeslots.csv ?", filters=CSV_FILTERS)
    if file_path is None:
        raise Exception("timeslots.csv est nécessaire pour le remplissage automatique")

    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        teacher_firstname = csv_line["Prénom"][0]
        teacher_surname = csv_line["Nom"][0]

        teacher = find_teacher_by_name(doc, teacher_firstname, teacher_surname)
        if teacher is None:
            print("- Nouveau colleur : {} {}".format(teacher_firstname, teacher_surname))
            teacher_contact = csv_line["Contact"][0]

            new_teacher = clm.TeacherData(teacher_firstname, teacher_surname)
            if teacher_contact != "":
                if "@" in teacher_contact:
                    new_teacher.email = teacher_contact
                else:
                    new_teacher.tel = teacher_contact
            teacher = doc.teachers.add(new_teacher).created

        subject_name = csv_line["Matière"][0]
        subject = find_subject_by_name(doc, subject_name)
        if subject is None:
            raise ValueError("La matière \"{}\" trouvé dans timeslots.csv n'existe pas".format(subject_name))

        # Declaring the teacher in a subject is a read-modify-write: `update`
        # replaces the whole value, so it has to start from what the teacher
        # already holds. `to_data()` fills the set with ids, so the new member
        # is an id too.
        teacher_data = teacher.to_data()
        teacher_data.subjects.add(subject.id)
        doc.teachers.update(teacher, teacher_data)

        week_pattern_name = csv_line["Semaines"][0]
        slot_day = to_collomatique_day(csv_line["Jour"][0])
        slot_time = int(csv_line["Heure"][0])
        slot_room = csv_line["Salle"][0]
        slot_cost = int(csv_line["Coût"][0])

        week_pattern = convert_week_pattern_name(doc, week_pattern_name)

        # The subject moved *into* the value, so `slots.add` no longer takes it
        # as a first argument.
        doc.slots.add(clm.SlotData(
            subject,
            teacher,
            slot_day,
            datetime.time(slot_time, 0),
            extra_info=slot_room,
            week_pattern=week_pattern,
            cost=slot_cost,
        ))

def update_incompats(doc):
    print("Configuration des incompatibilités horaires...")

    file_path = clm.dialogs.open_file(title="incompats.csv ?", filters=CSV_FILTERS)
    if file_path is None:
        raise Exception("incompats.csv est nécessaire pour le remplissage automatique")

    subject_map = {}

    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        subject_name = csv_line["Matière associée"][0]
        subject = find_subject_by_name(doc, subject_name)

        if subject is None:
            print("- Nouvelle matière sans colle \"{}\"".format(subject_name))
            subject = doc.subjects.add(
                clm.SubjectData(subject_name, interrogation=None)
            ).created

        if subject not in subject_map:
            subject_map[subject] = {}

        incompat_name = csv_line["Incompatibilité"][0]

        incompat_day = to_collomatique_day(csv_line["Jour"][0])
        incompat_time = int(csv_line["Heure"][0])
        incompat_duration = int(csv_line["Durée"][0])*60

        new_slot = clm.TimeSlot(
            incompat_day, datetime.time(incompat_time, 0), incompat_duration
        )

        if incompat_name in subject_map[subject]:
            # An `IncompatData` is a real dataclass, so its list belongs to the
            # value: appending to it is the whole update. The old getter handed
            # back a copy, which is why the original had to write the list back.
            subject_map[subject][incompat_name].slots.append(new_slot)
        else:
            incompat_min_free = int(csv_line["Min libre"][0])
            incompat_week_pattern = convert_week_pattern_name(doc, csv_line["Semaines"][0])

            # (nom, matière) — the subject comes second now, which is the other
            # way round from the old API.
            incompat = clm.IncompatData(
                incompat_name,
                subject,
                slots=[new_slot],
                minimum_free_slots=incompat_min_free,
                week_pattern=incompat_week_pattern,
            )
            subject_map[subject][incompat_name] = incompat

    for (subject, incompats) in subject_map.items():
        for (incompat_name, incompat) in incompats.items():
            doc.incompats.add(incompat)

def load_rules(doc):
    print("Chargement d'un fichier de règle...")

    file_path = clm.dialogs.open_file(title="rules.csv ?", filters=CSV_FILTERS)
    if file_path is None:
        raise Exception("rules.csv est nécessaire pour le remplissage automatique")

    column_rules = {}
    auto_sub = []

    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        column = csv_line["Colonne"][0]
        content = csv_line["Contenu"][0]
        subject_name = csv_line["Matière"][0]

        subject = find_subject_by_name(doc, subject_name)
        if subject is None:
            raise ValueError("Matière invalide dans les règles : \"{}\"".format(subject_name))

        if column == "":
            auto_sub.append(subject)
        else:
            if column not in column_rules:
                column_rules[column] = {}

            if content not in column_rules[column]:
                column_rules[column][content] = []

            column_rules[column][content].append(subject)
    return column_rules, auto_sub

def split_student_name(student_full_name):
    name_list = student_full_name.split(" ")
    surname = ""
    i = 0
    while i < len(name_list) and name_list[i].isupper():
        if i != 0:
            surname += " "
        surname += name_list[i]
        i += 1
    if i < len(name_list):
        firstname = name_list[i]
        i += 1
    else:
        firstname = ""

    while i < len(name_list):
        firstname += " "
        firstname += name_list[i]
        i += 1

    return firstname, surname

def subscribe_student_to_subjects(doc, student, main_period, subjects):
    for subject in subjects:
        # (période, matière, élève) — the subject comes before the student,
        # which is the other way round from the old API.
        doc.assignments.set(main_period, subject, student, True)

def apply_rules(doc, student, main_period, csv_line, rules):
    column_rules, auto_sub = rules

    subscribe_student_to_subjects(doc, student, main_period, auto_sub)

    for (column, content_map) in column_rules.items():
        content = csv_line[column][0]
        if content in content_map:
            subscribe_student_to_subjects(doc, student, main_period, content_map[content])

def import_students_file(doc, main_period, file_path):
    print("Importation d'un fichier élève...")

    rules = load_rules(doc)

    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        student_full_name = csv_line['\ufeff'][0] # Yes, the pronote CSV is that bad
        if not student_full_name:
            print("Bad line: {}".format(csv_line))
            continue
        print("- Ajout de {}".format(student_full_name))

        firstname, surname = split_student_name(student_full_name)

        student = doc.students.add(clm.StudentData(firstname, surname)).created

        apply_rules(doc, student, main_period, csv_line, rules)

def find_student_by_name(doc, student_name):
    firstname, surname = split_student_name(student_name)

    for student in doc.students:
        if student.firstname == firstname and student.surname == surname:
            return student
    return None

def update_group_lists(doc):
    print("Remplissage des listes de groupes...")

    file_path = clm.dialogs.open_file(title="groups.csv ?", filters=CSV_FILTERS)
    if file_path is None:
        raise Exception("groups.csv est nécessaire pour le remplissage automatique")

    group_lists = {}

    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        group_list_name = csv_line["Liste"][0]
        group_list = find_group_list_by_name(doc, group_list_name)
        if group_list is None:
            group_list = doc.group_lists.add(clm.GroupListData(group_list_name)).created

        if group_list not in group_lists:
            group_lists[group_list] = {}

        student_name = csv_line["Élève"][0]
        student = find_student_by_name(doc, student_name)
        if student is None:
            raise ValueError("Élève inconnu : {}".format(student_name))

        group_name = csv_line["Groupe"][0]
        if group_name not in group_lists[group_list]:
            group_lists[group_list][group_name] = []

        group_lists[group_list][group_name].append(student)

    for (group_list, groups) in group_lists.items():
        prefilled_groups = []
        group_names = []
        for (group_name, students) in groups.items():
            prefilled_groups.append(set(students))
            group_names.append(group_name)

        # A read-modify-write again: the names and the filling change, the
        # students per group the subjects laid down stay. The model wants
        # exactly one name per group, which is what replaces the 255 placeholder
        # names above.
        group_list_data = group_list.to_data()
        group_list_data.group_names = group_names
        group_list_data.filling = clm.PrefilledGroups(prefilled_groups)
        doc.group_lists.update(group_list, group_list_data)

def main():
    doc = clm.default_document(sys.argv[1] if len(sys.argv) > 1 else None)

    # One undo slot for the whole import: Ctrl-Z takes all of it back at once.
    with doc.transaction("Import complet"):
        main_period = find_main_period(doc)
        update_subjects_and_empty_group_lists(doc, main_period)
        update_timeslots_and_teachers(doc)
        update_incompats(doc)
        while True:
            # The old script asked « Importer un fichier élève ? » before each
            # round. There is no message box in the new api, so Cancel on the
            # chooser itself is what says "no more files" — one click less per
            # file, and each round now asks for students.csv before its
            # rules.csv rather than after.
            file_path = clm.dialogs.open_file(
                title="students.csv ? (Annuler pour terminer)",
                filters=CSV_FILTERS,
            )
            if file_path is None:
                break
            import_students_file(doc, main_period, file_path)
        update_group_lists(doc)

    doc.save()

main()
