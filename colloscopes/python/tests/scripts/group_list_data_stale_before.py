import collomatique

doc = collomatique.load(source)

# Rust removes one group list between this stage and the next — the read
# surface ships no writes of its own.
group_list_list = list(doc.group_lists)
doomed = group_list_list[-1]
living = group_list_list[0]
assert doomed != living

# Written down while everything is alive, and read again afterwards.
living_name = living.name

# A value is a detached object: the removal of the list it describes cannot
# reach it, since nothing in it names the list.
doomed_value = doomed.to_data()
doomed_name = doomed_value.name
