import collomatique

def import_students(db, pronote_csv, rules_csv):
    import collomatique

    student_handles = []
    for line in pronote_csv.content:
        name = line[0]
        firstname, lastname = collomatique.extract_name_parts(name, firstname_first = False)

        collo_object = collomatique.Student(firstname, lastname)
        student_handle = db.students_create(collo_object)

        student_handles.append(student_handle)

    def find_subject_group(db, subject_group):
        subject_groups = db.subject_groups_get_all()
        for (key,sg) in subject_groups.items():
            if sg.name == subject_group:
                return key
        return None

    def find_subject(db, subject):
        subjects = db.subjects_get_all()
        for (key,s) in subjects.items():
            if s.name == subject:
                return key
        return None

    def apply_rule(db, student_handle, student_line_map, rule_map):
        column = rule_map["Colonne"][0]
        content = student_line_map[column][0]
        if content == rule_map["Contenu"][0]:
            subject_group_handle = find_subject_group(db, rule_map["Groupement"][0])
            subject_handle = find_subject(db, rule_map["Matière"][0])
            
            db.subject_group_for_student_set(student_handle, subject_group_handle, subject_handle)

    for i,student_handle in enumerate(student_handles):
        for rule_map in rules_csv.map:
            apply_rule(db, student_handle, pronote_csv.map[i], rule_map)

rules_csv = collomatique.load_csv("scripts/rules.csv", has_headers = True)
import_students(db, csv, rules_csv)

