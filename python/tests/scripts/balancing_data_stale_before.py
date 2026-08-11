import collomatique

# The first of two stages. Métamorphose has an override in the file; rust
# removes it between this stage and the next.
doc = collomatique.load(source)

metamorphose = [subject for subject in doc.subjects if subject.name == "Métamorphose"][0]

# Held so that `to_data()` through each of them is tried in the second half:
# the raw view dies with its entry, the resolved view re-resolves.
raw_view = doc.balancing.override_for(metamorphose)
resolved_view = doc.balancing.options_for(metamorphose)
assert raw_view is not None

# Written down while everything is alive. The value is a detached object: the
# removal of the entry it describes cannot reach it, since nothing in it names
# the document.
doomed_value = raw_view.to_data()
resolved_value = resolved_view.to_data()
global_value = doc.balancing.global_options.to_data()
assert resolved_value != global_value
