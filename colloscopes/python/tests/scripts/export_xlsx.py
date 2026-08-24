import pathlib

import collomatique

# `source` is a throwaway copy of a real colloscope; `own_target`,
# `full_target` and `bad_target` are where the script writes, handed in by the
# rust side so rust can read the files back.
doc = collomatique.load(source)

assert issubclass(collomatique.ExportError, collomatique.Error)

# ------------------------------------------------ what the document itself says

# The document's own configuration, cut down to a single sheet: this is what a
# bare `export_xlsx` has to use, and the smaller of the two files is how rust
# tells that it did.
with doc.transaction("Cut the export down"):
    doc.export_config.set_colloscope_enabled(True)
    doc.export_config.set_all_groups_enabled(False)
    doc.export_config.set_automatic_groups_enabled(False)
    doc.export_config.set_prefilled_groups_enabled(False)
    doc.export_config.set_per_group_list_enabled(False)

doc.export_xlsx(own_target)

assert pathlib.Path(own_target).exists()

# An export writes a spreadsheet, not the document: nothing was applied, so the
# undo stack is where the block left it.
assert doc.undo_name == "Cut the export down"
assert doc.can_redo is False

# ---------------------------------------------------- what the caller asks for

# The same document, exported the caller's way. The tree comes from the
# document and goes back changed, which is how a script says « like mine, but
# with the group sheets too ».
config = doc.export_config.to_data()
assert isinstance(config, collomatique.ExportConfigData)
config.all_groups_enabled = True
config.per_group_list_enabled = True

doc.export_xlsx(pathlib.Path(full_target), config)

# For this call only: a configuration handed in is not remembered, and the
# document still holds the one the block above wrote.
assert doc.export_config.all_groups_enabled is False
assert doc.export_config.per_group_list_enabled is False
assert doc.undo_name == "Cut the export down"

# More sheets is more file. This is the whole of « `None` means the document's
# own »: the two exports differ exactly because the configurations do.
own_size = pathlib.Path(own_target).stat().st_size
full_size = pathlib.Path(full_target).stat().st_size
assert own_size < full_size

# ------------------------------------------------------------- what is refused

# A configuration that is not one, refused where every other value is — at the
# boundary, before anything is written.
try:
    doc.export_xlsx(bad_target, 3)
except TypeError:
    pass
else:
    raise AssertionError("export_xlsx takes an ExportConfigData")

assert not pathlib.Path(bad_target).exists()

# A path with no directory to hold it. The writer's failures and the file
# system's arrive the same way, and the message names the path that failed.
missing = pathlib.Path(bad_target).parent / "no-such-directory" / "export.xlsx"
try:
    doc.export_xlsx(missing)
except collomatique.ExportError as e:
    failure = str(e)
else:
    raise AssertionError("a path with no directory to hold it must be refused")

assert failure.startswith(str(missing))
