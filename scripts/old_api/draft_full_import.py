#!/usr/bin/env python3

import collomatique_old as collomatique
import csv

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

def show_error(s, msg):
    s.dialog_info_message(msg)
    raise ValueError(msg)

def update_general_settings(s):
    f = s.get_current_collomatique_file()
    collomatique.log("Configuration générale...")
    max_interrogations_per_day = int(s.dialog_input("Nb colles maximum par jour ?"))
    min_interrogations_per_week = int(s.dialog_input("Nb colles minimum par semaine ?"))
    max_interrogations_per_week = int(s.dialog_input("Nb colles maximum par semaine ?"))

    soft_interrogations_per_week_min = collomatique.SoftU32(min_interrogations_per_week)
    soft_interrogations_per_week_max = collomatique.SoftU32(max_interrogations_per_week)
    soft_max_interrogations_per_day = collomatique.SoftNonZeroU32(max_interrogations_per_day)
    limits = collomatique.Limits()
    limits.interrogations_per_week_min = soft_interrogations_per_week_min
    limits.interrogations_per_week_max = soft_interrogations_per_week_max
    limits.max_interrogations_per_day = soft_max_interrogations_per_day

    f.settings_update_global_limits(limits)

def update_general_planning(s):
    f = s.get_current_collomatique_file()
    collomatique.log("Configuration du planning...")
    week_count = int(s.dialog_input("Nb de semaines ?"))
    if week_count <= 0:
        raise ValueError("Le nombre de semaines doit être supérieur (ou égal) à 1")

    main_period_id = f.periods_add(week_count)
    
    return main_period_id, week_count

def find_subject_by_name(f, subject_name):
    subjects = f.get_main_params().subjects
    for subject in subjects:
        if subject.parameters.name == subject_name:
            return subject
    return None

def find_group_list_id_by_name(f, group_list_name):
    group_list_map = f.get_main_params().group_lists
    for (group_list_id, group_list) in group_list_map.items():
        if group_list.parameters.name == group_list_name:
            return group_list_id
    return None

def update_subjects_and_empty_group_lists(s, main_period_id):
    f = s.get_current_collomatique_file()
    collomatique.log("Configuration des matières et des listes de groupes associées...")
    
    file_path = s.dialog_open_file("subjects.csv ?", [("Fichiers CSV", "csv"), ("Tous les fichiers", "*")])
    if file_path is None:
        raise Exception("subjects.csv est nécessaire pour le remplissage automatique")
    
    csv_columns, csv_content = open_csv(file_path)
    collomatique.log("Parcours des matières...")
    for csv_line in csv_content:
        subject_name = csv_line["Matière"][0]
        collomatique.log("- \"{}\"".format(subject_name))

        if find_subject_by_name(f, subject_name) is not None:
            collomatique.log("Matière déjà existante")
            continue

        new_subject = collomatique.SubjectParameters(subject_name)
        duration = int(csv_line["Durée"][0])
        if duration == 0:
            interrogation_parameters = None
            group_list_id = None
        else:
            interrogation_parameters = collomatique.SubjectInterrogationParameters()

            period = int(csv_line["Période"][0])
            strict_period = int(csv_line["Période stricte"][0])
            tutorial = int(csv_line["TD"][0])
            students_per_group_min = int(csv_line["Min élèves par groupe"][0])
            students_per_group_max = int(csv_line["Max élèves par groupe"][0])
            groups_per_interrogation_max = int(csv_line["Max nb groupes"][0])
            group_list_name = csv_line["Liste"][0]

            interrogation_parameters.students_per_group_min = students_per_group_min
            interrogation_parameters.students_per_group_max = students_per_group_max
            interrogation_parameters.groups_per_interrogation_min = 1
            interrogation_parameters.groups_per_interrogation_max = groups_per_interrogation_max
            interrogation_parameters.duration = duration
            interrogation_parameters.take_duration_into_account = (tutorial == 0)

            if strict_period:
                periodicity = collomatique.SubjectPeriodicity.ExactlyPeriodic(period)
            else:
                periodicity = collomatique.SubjectPeriodicity.OnceForEveryBlockOfWeeks(period, 1)

            interrogation_parameters.periodicity = periodicity

            if group_list_name != "":
                group_list_id = find_group_list_id_by_name(f, group_list_name)
                if group_list_id is not None:
                    old_group_list = f.get_main_params().group_lists[group_list_id]
                    params = old_group_list.parameters
                    if params.students_per_group_min != students_per_group_min or params.students_per_group_max != students_per_group_max:
                        show_error(s, "La liste de groupes \"{}\" n'a pas le bon nombre d'élèves par groupe".format(group_list_name))
                else:
                    collomatique.log("Nouvelle liste de groupe \"{}\"".format(group_list_name))

                    new_group_list = collomatique.GroupListParameters(group_list_name)
                    new_group_list.students_per_group_min = students_per_group_min
                    new_group_list.students_per_group_max = students_per_group_max
                    new_group_list.group_names = ["" for i in range(255)]

                    group_list_id = f.group_lists_add(new_group_list)
            else:
                group_list_id = None

        new_subject.interrogation_parameters = interrogation_parameters

        subject_id = f.subjects_add(new_subject)
        f.group_lists_set_association(main_period_id, subject_id, group_list_id)

def find_week_pattern_id_by_name(s, week_pattern_name):
    f = s.get_current_collomatique_file()
    week_pattern_map = f.get_main_params().week_patterns
    for (week_pattern_id, week_pattern) in week_pattern_map.items():
        if week_pattern.name == week_pattern_name:
            return week_pattern_id
    return None

def find_teacher_id_by_name(s, firstname, surname):
    f = s.get_current_collomatique_file()
    teacher_map = f.get_main_params().teachers
    for (teacher_id, teacher) in teacher_map.items():
        if teacher.desc.firstname == firstname and teacher.desc.surname == surname:
            return teacher_id
    return None

def to_collomatique_day(s, day):
    match day:
        case 'Lundi':
            return collomatique.Weekday.Monday
        case 'Mardi':
            return collomatique.Weekday.Tuesday
        case 'Mercredi':
            return collomatique.Weekday.Wednesday
        case 'Jeudi':
            return collomatique.Weekday.Thursday
        case 'Vendredi':
            return collomatique.Weekday.Friday
        case 'Samedi':
            return collomatique.Weekday.Saturday
        case 'Dimanche':
            return collomatique.Weekday.Sunday
        case _:
            show_error(s, "Jour inconnu : \"{}\"".format(day))

def convert_week_pattern_name(s, week_pattern_name):
    f = s.get_current_collomatique_file()
    if week_pattern_name == "":
        return None
    else:
        week_pattern_id = find_week_pattern_id_by_name(s, week_pattern_name)
        if week_pattern_id is None:
            week_count = f.get_main_params().get_week_count()
            new_week_pattern = collomatique.WeekPattern(week_pattern_name, week_count)
            week_pattern_id = f.week_patterns_add(new_week_pattern)
        return week_pattern_id

def update_timeslots_and_teachers(s):
    f = s.get_current_collomatique_file()
    collomatique.log("Configuration des créneaux de colles et des colleurs associés...")
    
    file_path = s.dialog_open_file("timeslots.csv ?", [("Fichiers CSV", "csv"), ("Tous les fichiers", "*")])
    if file_path is None:
        raise Exception("timeslots.csv est nécessaire pour le remplissage automatique")
    
    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        teacher_firstname = csv_line["Prénom"][0]
        teacher_surname = csv_line["Nom"][0]

        teacher_id = find_teacher_id_by_name(s, teacher_firstname, teacher_surname)
        if teacher_id is None:
            collomatique.log("- Nouveau colleur : {} {}".format(teacher_firstname, teacher_surname))
            teacher_contact = csv_line["Contact"][0]

            new_teacher = collomatique.Teacher(teacher_firstname, teacher_surname)
            if teacher_contact != "":
                if "@" in teacher_contact:
                    new_teacher.desc.email = teacher_contact
                else:
                    new_teacher.desc.tel = teacher_contact
            teacher_id = f.teachers_add(new_teacher)
        
        subject_name = csv_line["Matière"][0]
        subject = find_subject_by_name(f, subject_name)
        if subject is None:
            show_error(s, "La matière \"{}\" trouvé dans timeslots.csv n'existe pas".format(subject_name))

        teacher = f.get_main_params().teachers[teacher_id]
        subjects = teacher.subjects
        subjects.add(subject.id)
        teacher.subjects = subjects
        f.teachers_update(teacher_id, teacher)

        week_pattern_name = csv_line["Semaines"][0]
        slot_day = to_collomatique_day(s, csv_line["Jour"][0])
        slot_time = int(csv_line["Heure"][0])
        slot_room = csv_line["Salle"][0]
        slot_cost = int(csv_line["Coût"][0])

        week_pattern_id = convert_week_pattern_name(s, week_pattern_name)
        
        start_time = collomatique.Time(slot_time, 0)
        slot_start = collomatique.SlotStart(slot_day, start_time)
        new_slot = collomatique.SlotParameters(teacher_id, slot_start)
        new_slot.extra_info = slot_room
        new_slot.week_pattern = week_pattern_id
        new_slot.cost = slot_cost

        f.slots_add(subject.id, new_slot)

def update_incompats(s):
    f = s.get_current_collomatique_file()
    collomatique.log("Configuration des incompatibilités horaires...")
    
    file_path = s.dialog_open_file("incompats.csv ?", [("Fichiers CSV", "csv"), ("Tous les fichiers", "*")])
    if file_path is None:
        raise Exception("incompats.csv est nécessaire pour le remplissage automatique")
    
    subject_map = {}

    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        subject_name = csv_line["Matière associée"][0]
        subject = find_subject_by_name(f, subject_name)

        if subject is None:
            collomatique.log("- Nouvelle matière sans colle \"{}\"".format(subject_name))
            new_subject = collomatique.SubjectParameters(subject_name)
            new_subject.interrogation_parameters = None
            subject_id = f.subjects_add(new_subject)
        else:
            subject_id = subject.id

        if subject_id not in subject_map:
            subject_map[subject_id] = {}

        incompat_name = csv_line["Incompatibilité"][0]

        incompat_day = to_collomatique_day(s, csv_line["Jour"][0])
        incompat_time = int(csv_line["Heure"][0])
        incompat_duration = int(csv_line["Durée"][0])*60

        start_time = collomatique.Time(incompat_time, 0)
        slot_start = collomatique.SlotStart(incompat_day, start_time)
        new_slot = collomatique.SlotWithDuration(slot_start, incompat_duration)

        if incompat_name in subject_map[subject_id]:
            slots = subject_map[subject_id][incompat_name].slots
            slots.append(new_slot)
            subject_map[subject_id][incompat_name].slots = slots
        else:
            incompat_min_free = int(csv_line["Min libre"][0])
            incompat_week_pattern = convert_week_pattern_name(s, csv_line["Semaines"][0])

            start_time = collomatique.Time(incompat_time, 0)
            slot_start = collomatique.SlotStart(incompat_day, start_time)
            new_slot = collomatique.SlotWithDuration(slot_start, incompat_duration)
            slots = [new_slot]

            incompat = collomatique.Incompat(subject_id, incompat_name, slots, incompat_min_free)
            incompat.week_pattern_id = incompat_week_pattern
            subject_map[subject_id][incompat_name] = incompat

    for (subject_id, incompats) in subject_map.items():
        for (incompat_name, incompat) in incompats.items():
            f.incompats_add(incompat)

def load_rules(s):
    f = s.get_current_collomatique_file()
    collomatique.log("Chargement d'un fichier de règle...")

    file_path = s.dialog_open_file("rules.csv ?", [("Fichiers CSV", "csv"), ("Tous les fichiers", "*")])
    if file_path is None:
        raise Exception("rules.csv est nécessaire pour le remplissage automatique")

    column_rules = {}
    auto_sub = []

    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        column = csv_line["Colonne"][0]
        content = csv_line["Contenu"][0]
        subject_name = csv_line["Matière"][0]

        subject = find_subject_by_name(f, subject_name)
        if subject is None:
            show_error(s, "Matière invalide dans les règles : \"{}\"".format(subject_name))
        
        if column == "":
            auto_sub.append(subject.id)
        else:
            if column not in column_rules:
                column_rules[column] = {}

            if content not in column_rules[column]:
                column_rules[column][content] = []
            
            column_rules[column][content].append(subject.id)
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

def subscribe_student_to_subjects(s, student_id, main_period_id, subjects):
    f = s.get_current_collomatique_file()
    for subject_id in subjects:
        f.assignments_set(main_period_id, student_id, subject_id, True)

def apply_rules(s, student_id, main_period_id, csv_line, rules):
    column_rules, auto_sub = rules

    subscribe_student_to_subjects(s, student_id, main_period_id, auto_sub)

    for (column, content_map) in column_rules.items():
        content = csv_line[column][0]
        if content in content_map:
            subscribe_student_to_subjects(s, student_id, main_period_id, content_map[content])

def import_students_file(s, main_period_id):
    f = s.get_current_collomatique_file()
    collomatique.log("Importation d'un fichier élève...")
    
    rules = load_rules(s)

    file_path = s.dialog_open_file("students.csv ?", [("Fichiers CSV", "csv"), ("Tous les fichiers", "*")])
    if file_path is None:
        raise Exception("students.csv est nécessaire pour le remplissage automatique")
    
    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        student_full_name = csv_line['\ufeff'][0] # Yes, the pronote CSV is that bad
        if not student_full_name:
            collomatique.log("Bad line: {}".format(csv_line))
            continue
        collomatique.log("- Ajout de {}".format(student_full_name))

        firstname, surname = split_student_name(student_full_name)

        student = collomatique.Student(firstname, surname)
        student_id = f.students_add(student)

        apply_rules(s, student_id, main_period_id, csv_line, rules)

def find_student_id_by_name(s, student_name):
    f = s.get_current_collomatique_file()
    firstname, surname = split_student_name(student_name)

    student_map = f.get_main_params().students
    for (student_id, student) in student_map.items():
        if student.desc.firstname == firstname and student.desc.surname == surname:
            return student_id
    return None

def update_group_lists(s):
    f = s.get_current_collomatique_file()
    collomatique.log("Remplissage des listes de groupes...")
    
    file_path = s.dialog_open_file("groups.csv ?", [("Fichiers CSV", "csv"), ("Tous les fichiers", "*")])
    if file_path is None:
        raise Exception("groups.csv est nécessaire pour le remplissage automatique")
    
    group_lists = {}

    csv_columns, csv_content = open_csv(file_path)
    for csv_line in csv_content:
        group_list_name = csv_line["Liste"][0]
        group_list_id = find_group_list_id_by_name(f, group_list_name)
        if group_list_id is None:
            new_group_list = collomatique.GroupListParameters(group_list_name)
            group_list_id = f.group_lists_add(new_group_list)
        
        if group_list_id not in group_lists:
            group_lists[group_list_id] = {}

        student_name = csv_line["Élève"][0]
        student_id = find_student_id_by_name(s, student_name)
        if student_id is None:
            show_error(s, "Élève inconnu : {}", student_name)
        
        group_name = csv_line["Groupe"][0]
        if group_name not in group_lists[group_list_id]:
            group_lists[group_list_id][group_name] = []

        group_lists[group_list_id][group_name].append(student_id)
    
    group_list_current_params = f.get_main_params().group_lists
    
    for (group_list_id, groups) in group_lists.items():
        prefilled_groups = []
        group_names = []
        for (group_name, students) in groups.items():
            new_group = collomatique.PrefilledGroup()
            new_group.students = set(students)
            prefilled_groups.append(new_group)
            group_names.append(group_name)
        params = group_list_current_params[group_list_id].parameters
        params.group_names = group_names
        new_group_list = collomatique.GroupList(
            params, collomatique.GroupListFilling.prefilled(prefilled_groups)
        )
        f.group_lists_update(group_list_id, new_group_list)

def build_subject_list_for_group_lists(s, main_period_id):
    f = s.get_current_collomatique_file()
    output = {}

    subjects = f.get_main_params().subjects
    for subject in subjects:
        group_lists_in_period = f.get_main_params().group_lists_associations[main_period_id]
        if subject.id in group_lists_in_period:
            group_list_id = group_lists_in_period[subject.id]
            if group_list_id not in output:
                output[group_list_id] = []
            output[group_list_id].append(subject.id)

    return output

def main():
    s = collomatique.current_session()
    update_general_settings(s)
    main_period_id, week_count = update_general_planning(s)
    update_subjects_and_empty_group_lists(s, main_period_id)
    update_timeslots_and_teachers(s)
    update_incompats(s)
    while s.dialog_confirm_action("Importer un fichier élève ?"):
        import_students_file(s, main_period_id)
    update_group_lists(s)

main()
