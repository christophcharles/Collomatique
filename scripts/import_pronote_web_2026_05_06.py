#!/usr/bin/env python3

import collomatique as clm
import csv
import sys

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
            if column not in csv_line:
                continue
            opt = csv_line[column][0]
            if opt:
                S.add(opt)
    return S

def find_or_add_subject(doc, subject):
    for sub in doc.subjects:
        if sub.name == subject:
            return sub
    # `SubjectData(name)` gives the subject a default interrogation, which is
    # what the old `SubjectParameters(name)` did too.
    return doc.subjects.add(clm.SubjectData(subject)).created

def add_subjects(doc, subject_set):
    subjects = {}
    for subject in subject_set:
        subjects[subject] = find_or_add_subject(doc, subject)
    return subjects

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

def add_student_from_csv_line(doc, csv_line, subjects):
    student_full_name = csv_line['\ufeff'][0]
    if not student_full_name:
        print("Bad line: {}".format(csv_line))
        return
    print("Ajout de {}".format(student_full_name))

    firstname, surname = split_student_name(student_full_name)

    student = doc.students.add(clm.StudentData(firstname, surname)).created

    for column in ["Option 1", "Option 2", "Option 3", "Autres options"]:
        if column not in csv_line:
            continue
        opt = csv_line[column][0]
        if not opt:
            continue
        subject = subjects[opt]

        for period in doc.periods:
            # (période, matière, élève) — the subject comes before the student,
            # which is the other way round from the old API.
            doc.assignments.set(period, subject, student, True)

def main():
    doc = clm.default_document(sys.argv[1] if len(sys.argv) > 1 else None)

    file_path = clm.dialogs.open_file(
        title="Ouvrir un CSV",
        filters=[("Fichiers CSV", ["csv"]), ("Tous les fichiers", ["*"])],
    )
    if file_path is None:
        return

    csv_columns, csv_content = open_csv(file_path)

    # One undo slot for the whole import, rather than one per student.
    with doc.transaction("Import Pronote"):
        if len(doc.periods) == 0:
            doc.periods.add(10)

        subject_set = build_subject_set(csv_content)
        subjects = add_subjects(doc, subject_set)

        for csv_line in csv_content:
            add_student_from_csv_line(doc, csv_line, subjects)

    doc.save()

main()
