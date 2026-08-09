import collomatique

# The first of three stages. Rust switches one subject's interrogations off
# between this stage and the next, then removes the subject between that one and
# the last — the read surface ships no writes of its own yet. Everything this
# stage leaves in the globals is what the other two ask questions about.
doc = collomatique.load(source)

subject_count_before = len(doc.subjects)

with_colles = [subject for subject in doc.subjects if subject.interrogation is not None]

# The *last* of them, so that removing it leaves the survivor's position alone:
# this way a changed `.index` further down would be a real failure and not the
# renumbering of a list that lost an earlier entry.
doomed = with_colles[-1]
doomed_id = doomed.id
doomed_name = doomed.name
doomed_index = doomed.index
doomed_view = doomed.interrogation
doomed_duration = doomed_view.duration

survivor = with_colles[0]
survivor_view = survivor.interrogation
survivor_name = survivor.name
survivor_duration = survivor_view.duration

# Handles and sub-views as dict keys: this is what must not blow up when the
# thing they name dies.
by_handle = {doomed: "subject", doomed_view: "interrogation"}
