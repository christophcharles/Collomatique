import collomatique

# `source` is a document written by the test: an export configuration away from
# the default on every field — a switched-off colloscope sheet, an extra color,
# an auto-detected orientation — `target` is where the script leaves the
# document for rust to read back, and `labels` are the french names `ops` gives
# the eleven operations of this family, in the order this script writes them,
# handed in from rust so that the undo assertions pin the operations' own labels
# and not merely some strings.
doc = collomatique.load(source)
config = doc.export_config

opened = config.to_data()

# Views held from before every write. Nothing of this family can go stale — the
# whole configuration is one atom of value data, replaced wholesale, and there
# is nothing to remove from under a view — so each of these reads whatever the
# writes leave behind, for as long as the document lives.
global_view = config.global_config
colloscope_view = config.colloscope_config
all_groups_view = config.all_groups_config
automatic_view = config.automatic_groups_config
prefilled_view = config.prefilled_groups_config
group_list_view = config.per_group_list_config

# The section shared by every sheet is rewritten whole: what the value says is
# what it becomes, field for field, and nothing is merged with what was there.
new_global = collomatique.ExportGlobalConfigData(
    background_color=collomatique.Color(16, 17, 18),
    stripes_color_enabled=True,
    stripes_color=collomatique.Color(19, 20, 21))
result = config.set_global(new_global)
assert isinstance(result, collomatique.OpResult)
# A write of this family creates nothing, so it answers a plain `OpResult` and
# never the `AddResult` subclass: there is no `created` at all, rather than one
# holding `None`.
assert not isinstance(result, collomatique.AddResult)
assert not hasattr(result, "created")
# The export configuration names no entity, so no write of this family ever has
# anything to repair — but the result says so rather than the call saying
# nothing at all.
assert result.warnings == []

# The views read the document, so the one held from before the write reads the
# section it left.
assert global_view.to_data() == new_global
assert global_view.background_color == collomatique.Color(16, 17, 18)
assert global_view.stripes_color_enabled is True
assert global_view.stripes_color == collomatique.Color(19, 20, 21)

# One setter writes one section: the five flags and the four sheets are what the
# document opened with.
assert colloscope_view.to_data() == opened.colloscope_config
assert config.colloscope_enabled == opened.colloscope_enabled

# The five flags sit beside the sections they gate rather than inside them, so
# switching a sheet on or off leaves everything its section holds — the
# interface's memory of what was chosen before the sheet was switched off.
assert config.colloscope_enabled is False
assert config.set_colloscope_enabled(True).warnings == []
assert config.colloscope_enabled is True
assert colloscope_view.to_data() == opened.colloscope_config

assert config.all_groups_enabled is False
assert config.set_all_groups_enabled(True).warnings == []
assert config.all_groups_enabled is True
assert all_groups_view.to_data() == opened.all_groups_config

assert config.automatic_groups_enabled is True
assert config.set_automatic_groups_enabled(False).warnings == []
assert config.automatic_groups_enabled is False
assert automatic_view.to_data() == opened.automatic_groups_config

assert config.prefilled_groups_enabled is True
assert config.set_prefilled_groups_enabled(False).warnings == []
assert config.prefilled_groups_enabled is False
assert prefilled_view.to_data() == opened.prefilled_groups_config

assert config.per_group_list_enabled is False
assert config.set_per_group_list_enabled(True).warnings == []
assert config.per_group_list_enabled is True
assert group_list_view.to_data() == opened.per_group_list_config

# The colloscope section, whole — the extra colors included: the map the value
# holds is the whole of the sheet's afterwards, so the label the document opened
# with is gone and the two written here are all there is.
new_colloscope = collomatique.ExportColloscopeConfigData(
    sheet_name="Colles",
    extra_info_column_enabled=True,
    extra_info_column_name="Remarques",
    teacher_email_enabled=True,
    teacher_email="Courriel",
    teacher_tel_enabled=False,
    teacher_tel="Téléphone",
    orientation=collomatique.Orientation.LANDSCAPE,
    display_week_dates=True,
    display_annotations=True,
    no_interrogation_color=collomatique.Color(22, 23, 24),
    annotation_color_enabled=True,
    annotation_color=collomatique.Color(25, 26, 27),
    extra_colors={
        "Vacances": collomatique.Color(28, 29, 30),
        "Examens": collomatique.Color(31, 32, 33),
    })
assert config.set_colloscope_config(new_colloscope).warnings == []
assert colloscope_view.to_data() == new_colloscope
assert colloscope_view.sheet_name == "Colles"
assert colloscope_view.orientation == collomatique.Orientation.LANDSCAPE
assert sorted(colloscope_view.extra_colors) == ["Examens", "Vacances"]
assert colloscope_view.extra_colors["Vacances"] == collomatique.Color(28, 29, 30)
# The flag beside it is what the toggle above left, not what the value said —
# there is nothing about it in the value at all.
assert config.colloscope_enabled is True

# One value class serves the three per-student-groups sheets, and what says
# which of them is being written is the setter, never the value.
all_groups_data = collomatique.ExportStudentGroupsConfigData(
    sheet_name="Groupes",
    orientation=collomatique.Orientation.PORTRAIT,
    show_emails=True,
    show_tel=False)
assert config.set_all_groups_config(all_groups_data).warnings == []
assert all_groups_view.to_data() == all_groups_data
assert all_groups_view.sheet_name == "Groupes"
assert all_groups_view.orientation == collomatique.Orientation.PORTRAIT
# The two sibling sheets are untouched by a write to this one.
assert automatic_view.to_data() == opened.automatic_groups_config
assert prefilled_view.to_data() == opened.prefilled_groups_config

# The automatic-groups sheet takes the model's own default for it, spelled by
# the classmethod that names it. Its `orientation` is `None`, which is a value
# the model holds — the auto-detect case — and not a field left unfilled: the
# sheet the document opened with a portrait orientation has none at all now.
automatic_data = collomatique.ExportStudentGroupsConfigData.automatic_groups()
assert config.set_automatic_groups_config(automatic_data).warnings == []
assert automatic_view.to_data() == automatic_data
assert automatic_view.sheet_name == "Groupes automatiques"
assert automatic_view.orientation is None

# The name a value carries is a field like any other: this one is the
# all-groups sheet's default and it is written to the prefilled-groups sheet,
# which renames that sheet rather than being refused — the setter is the
# address, and the value is only ever what is written there.
misnamed = collomatique.ExportStudentGroupsConfigData.all_groups()
assert config.set_prefilled_groups_config(misnamed).warnings == []
assert prefilled_view.to_data() == misnamed
assert prefilled_view.sheet_name == "Tous les groupes"
# The sheet whose default that was is what its own write left.
assert all_groups_view.to_data() == all_groups_data

# The last section: one setting for every per-group-list sheet, as the model
# holds them.
new_group_list = collomatique.ExportGroupListConfigData(
    orientation=collomatique.Orientation.PORTRAIT,
    show_emails=True,
    show_tel=True,
    center_vertically=False)
assert config.set_per_group_list_config(new_group_list).warnings == []
assert group_list_view.to_data() == new_group_list

# The whole tree, read at once, is what the eleven writes left — and it is not
# what the document opened with.
written = config.to_data()
assert written != opened
assert written.global_config == new_global
assert written.colloscope_enabled is True
assert written.all_groups_enabled is True
assert written.automatic_groups_enabled is False
assert written.prefilled_groups_enabled is False
assert written.per_group_list_enabled is True
assert written.colloscope_config == new_colloscope
assert written.all_groups_config == all_groups_data
assert written.automatic_groups_config == automatic_data
assert written.prefilled_groups_config == misnamed
assert written.per_group_list_config == new_group_list

# A value is refused before any op is built, and nothing is written: a field
# that was never of the right shape, read at its own site inside the value.
for call in (
    lambda: config.set_global(
        collomatique.ExportGlobalConfigData(background_color=3)),
    lambda: config.set_global(
        collomatique.ExportGlobalConfigData(stripes_color_enabled="oui")),
    lambda: config.set_colloscope_config(
        collomatique.ExportColloscopeConfigData(sheet_name=None)),
    lambda: config.set_colloscope_config(
        collomatique.ExportColloscopeConfigData(orientation="portrait")),
    lambda: config.set_colloscope_config(
        collomatique.ExportColloscopeConfigData(
            extra_colors={"Vacances": "rouge"})),
    lambda: config.set_all_groups_config(
        collomatique.ExportStudentGroupsConfigData(sheet_name=3)),
    lambda: config.set_automatic_groups_config(
        collomatique.ExportStudentGroupsConfigData(
            sheet_name="Auto", orientation="portrait")),
    lambda: config.set_prefilled_groups_config(
        collomatique.ExportStudentGroupsConfigData(
            sheet_name="Prérempli", show_tel=1)),
    lambda: config.set_per_group_list_config(
        collomatique.ExportGroupListConfigData(center_vertically="oui")),
):
    try:
        call()
    except TypeError:
        pass
    else:
        raise AssertionError("a field of the wrong shape must be refused")

# A value of another class is refused the same way, and for the same reason: it
# has none of the fields the section is read for.
try:
    config.set_global(collomatique.ExportGroupListConfigData())
except TypeError:
    pass
else:
    raise AssertionError("a value of another class must be refused")

# The five toggles take `True` or `False` and nothing else: a truthy value of
# another kind is refused rather than guessed at, the way every field of a value
# refuses one.
for call in (
    lambda: config.set_colloscope_enabled("oui"),
    lambda: config.set_all_groups_enabled(1),
    lambda: config.set_per_group_list_enabled(None),
):
    try:
        call()
    except TypeError:
        pass
    else:
        raise AssertionError("a flag that is not True or False must be refused")

# Nothing of that was written: the configuration is what the eleven accepted
# writes left.
assert config.to_data() == written

# This is what rust reads back off the disk.
doc.save(target)

# Each accepted call was its own undo slot, named by the operation itself — and
# the refused ones left no slot at all. The labels arrive in the order this
# script wrote them, so undoing walks them backwards.
assert doc.undo_name == labels[-1]
doc.undo()
assert doc.redo_name == labels[-1]
assert group_list_view.to_data() == opened.per_group_list_config

for label in reversed(labels[:-1]):
    assert doc.undo_name == label
    doc.undo()

assert doc.can_undo is False
# Undoing every one of them puts back the configuration the document opened
# with, whole.
assert config.to_data() == opened
