import collomatique

# `source` is a document written by the test holding at least one edge of every
# site class; `other` is the example, whose ids are disjoint from the fixture's
# — the foreign-handle questions below.
doc = collomatique.load(source)
other = collomatique.load(other_source)

first_period = next(iter(doc.periods))

# `referenced_by` hands a tuple out, in the registry's walk order.
assert isinstance(first_period.referenced_by(), tuple)

# The coordinate attribute names, pinned class by class: the table is what
# makes the generic recording below work, and what the rust side mirrors as
# the site conversion.
COORDS = {
    collomatique.WeekPeriod: ("week",),
    collomatique.SubjectExcludedPeriod: ("subject",),
    collomatique.StudentExcludedPeriod: ("student",),
    collomatique.PairingRuleExcludedPeriod: ("rule",),
    collomatique.SlotPairingRuleExcludedPeriod: ("rule",),
    collomatique.AssignmentRow: ("period", "subject"),
    collomatique.GroupListAssociation: ("period", "subject"),
    collomatique.TeacherSubject: ("teacher",),
    collomatique.SlotSubject: ("slot",),
    collomatique.SlotTeacher: ("slot",),
    collomatique.SlotWeekPattern: ("slot",),
    collomatique.IncompatSubject: ("incompat",),
    collomatique.IncompatWeekPattern: ("incompat",),
    collomatique.PairingRuleAntecedent: ("rule",),
    collomatique.PairingRuleConsequent: ("rule",),
    collomatique.SlotPairingRuleAntecedent: ("rule",),
    collomatique.SlotPairingRuleConsequent: ("rule",),
    collomatique.SettingsOverride: ("student",),
    collomatique.BalancingOverride: ("subject",),
    collomatique.WeekPatternExcludedWeek: ("week_pattern",),
    collomatique.GroupListPrefilledStudent: ("group_list",),
    collomatique.GroupListExcludedStudent: ("group_list",),
    collomatique.ColloscopeInterrogation: ("slot", "week"),
    collomatique.ColloscopeGroupListRow: ("group_list",),
}

# Each entity kind's position in its own collection: a recorded site's
# coordinates are named by their index, the shape the rust side compares. The
# dicts keyed by handle also pin that a site's coordinates are live handles.
index = {}
for kind, collection in (
    (collomatique.Period, doc.periods),
    (collomatique.Week, doc.weeks),
    (collomatique.Subject, doc.subjects),
    (collomatique.Teacher, doc.teachers),
    (collomatique.Student, doc.students),
    (collomatique.WeekPattern, doc.week_patterns),
    (collomatique.Slot, doc.slots),
    (collomatique.Incompat, doc.incompats),
    (collomatique.GroupList, doc.group_lists),
    (collomatique.PairingRule, doc.pairings),
    (collomatique.SlotPairingRule, doc.slot_pairings),
):
    index[kind] = {handle: i for i, handle in enumerate(collection)}


def record(handle):
    out = []
    for site in handle.referenced_by():
        assert isinstance(site, collomatique.RefSite)
        attrs = COORDS[type(site)]
        out.append(
            (
                type(site).__name__,
                tuple(index[type(getattr(site, attr))][getattr(site, attr)] for attr in attrs),
            )
        )
    return out


# Every entity of every referencable kind, in its collection's order: the
# recorded lists are the same shape the rust side computes from the model.
period_refs = [record(p) for p in doc.periods]
week_refs = [record(w) for w in doc.weeks]
subject_refs = [record(s) for s in doc.subjects]
teacher_refs = [record(t) for t in doc.teachers]
student_refs = [record(s) for s in doc.students]
week_pattern_refs = [record(p) for p in doc.week_patterns]
slot_refs = [record(s) for s in doc.slots]
group_list_refs = [record(g) for g in doc.group_lists]

# Three kinds are never the target of a reference: the registry has no site
# vocabulary for them, so while the handle is alive the answer is `()`.
for handle in list(doc.incompats) + list(doc.pairings) + list(doc.slot_pairings):
    assert handle.referenced_by() == ()
never_referenced = True

# A script builds the site it expects from handles and asks membership — the
# reason the classes are constructible at all.
teacher = next(t for t in doc.teachers if t.surname == "McGonagall")
slot_with_pattern = next(s for s in doc.slots if s.week_pattern is not None)
assert collomatique.SlotTeacher(slot_with_pattern) in teacher.referenced_by()

# Equality, hashing and inequality, all by the coordinates.
site = next(s for s in teacher.referenced_by() if isinstance(s, collomatique.SlotTeacher))
assert site == collomatique.SlotTeacher(site.slot)
assert hash(site) == hash(collomatique.SlotTeacher(site.slot))
other_slot = next(s for s in doc.slots if s.id != site.slot.id)
assert site != collomatique.SlotTeacher(other_slot)
assert site != 3

# A site from another document's handles is simply not this document's site:
# it answers `False` to the membership question, without raising.
other_slot = list(other.slots)[0]
other_period = list(other.periods)[0]
assert collomatique.SlotTeacher(other_slot) not in teacher.referenced_by()
first_subject = next(iter(doc.subjects))
assert collomatique.AssignmentRow(other_period, first_subject) not in first_subject.referenced_by()

# The coordinates are handles, and nothing else: a bare id or a number is a
# TypeError, not a site that silently compares false.
for bad in (slot_with_pattern.id, 3, "Maths"):
    try:
        collomatique.SlotTeacher(bad)
    except TypeError:
        pass
    else:
        raise AssertionError("a site's coordinates are handles, and nothing else")

# The repr names the place through its coordinate handle's own repr.
assert repr(site).startswith("SlotTeacher(slot=<Slot ")

# Matching names the coordinates positionally, through `__match_args__`.
matched = []
for s in first_subject.referenced_by():
    match s:
        case collomatique.AssignmentRow(period=period, subject=subject):
            assert period in doc.periods and subject in doc.subjects
            matched.append(("AssignmentRow", period.index, subject.index))
        case collomatique.SlotSubject(slot):
            assert slot.subject == first_subject
            matched.append(("SlotSubject", slot.index))
        case _:
            pass
assert len(matched) > 0
