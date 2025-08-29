#!/usr/bin/env python3

import collomatique
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

def build_subject_set(csv_content):
    S = set({})
    for csv_line in csv_content:
        for column in ["Option 1", "Option 2", "Option 3", "Autres options"]:
            opt = csv_line[column][0]
            if opt:
                S.add(opt)
    return S

def add_subjects(session, subject_set):
    subject_ids = {}
    for subject in subject_set:
        new_id = session.subjects_add(collomatique.SubjectParameters(subject))
        subject_ids[subject] = new_id
    return subject_ids

def add_student_from_csv_line(session, csv_line, subject_ids):
    student_full_name = csv_line['\ufeff'][0] # Yes, the pronote CSV is that bad
    if not student_full_name:
        collomatique.log("Bad line: {}".format(csv_line))
        return
    collomatique.log("Ajout de {}".format(student_full_name))

    name_list = student_full_name.split(" ", 1)
    surname = name_list[0]
    firstname = name_list[1]

    student = collomatique.Student(firstname, surname)
    student_id = session.students_add(student)

    periods = session.periods_get_list()

    for column in ["Option 1", "Option 2", "Option 3", "Autres options"]:
        opt = csv_line[column][0]
        if not opt:
            continue
        opt_id = subject_ids[opt]

        for period in periods:
            session.assignments_set(period.id, student_id, opt_id, True)

def main():
    session = collomatique.current_session()

    file_path = collomatique.open_dialog([("Fichiers CSV", "csv"), ("Tous les fichiers", "*")])
    if file_path is None:
        return
    
    csv_columns, csv_content = open_csv(file_path)

    subject_set = build_subject_set(csv_content)
    subject_ids = add_subjects(session, subject_set)

    for csv_line in csv_content:
        add_student_from_csv_line(session, csv_line, subject_ids)

main()