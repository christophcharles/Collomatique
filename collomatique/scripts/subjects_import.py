import collomatique

def import_subjects(db, csv):
    def get_subject_group_handle(db, subject_group):
        subject_groups = db.subject_groups_get_all()
        for (key, sg) in subject_groups.items():
            if sg.name == subject_group:
                return key
        return None

    import collomatique
    subject_group_handles = {}
    for line in csv.map:
        subject_group = line["Groupement"][0]
        subject_group_handle = get_subject_group_handle(db, subject_group)
        if subject_group_handle is None:
            collo_object = collomatique.SubjectGroup(subject_group)
            subject_group_handle = db.subject_groups_create(collo_object)
        subject_group_handles[subject_group] = subject_group_handle

    def get_subject_handle(db, subject):
        subjects = db.subjects_get_all()
        for (key, s) in subjects.items():
            if s.name == subject:
                return key
        return None

    for line in csv.map:
        subject = line["Matière"][0]
        if get_subject_handle(db, subject) is None:
            subject_group = line["Groupement"][0]
            subject_group_handle = subject_group_handles[subject_group]
            
            collo_object = collomatique.Subject(subject, subject_group_handle)
            collo_object.duration = int(line["Durée"][0])
            collo_object.students_per_group_range = (int(line["Min élèves par groupe"][0]), int(line["Max élèves par groupe"][0]))
            collo_object.period = int(line["Période"][0])
            collo_object.period_is_strict = int(line["Période stricte"][0]) != 0
            collo_object.is_tutorial = int(line["TD"][0]) != 0
            collo_object.max_groups_per_slot = 1
            collo_object.balance_teachers = int(line["Équilibrer les prof"][0]) != 0
            collo_object.balance_timeslots = True

            db.subjects_create(collo_object)

import_subjects(db, csv)

