import dataclasses

import collomatique

# `source` is a document written by the test: an export configuration away
# from the default on every field — switched-off sheets, an auto-detected
# orientation, an extra color. The numeric values are compared against the
# model on the rust side; what this script pins is the api's own shape.
doc = collomatique.load(source)

config = doc.export_config

# The whole tree, detached. It is a real tree of values: the nested sections
# are mutable objects of their own, so this is a genuine mutation of a
# detached builder — and one the document cannot see.
tree = config.to_data()
assert isinstance(tree, collomatique.ExportConfigData)
assert isinstance(tree.global_config, collomatique.ExportGlobalConfigData)
assert isinstance(tree.colloscope_config, collomatique.ExportColloscopeConfigData)
assert isinstance(tree.all_groups_config, collomatique.ExportStudentGroupsConfigData)
assert isinstance(tree.automatic_groups_config, collomatique.ExportStudentGroupsConfigData)
assert isinstance(tree.prefilled_groups_config, collomatique.ExportStudentGroupsConfigData)
assert isinstance(tree.per_group_list_config, collomatique.ExportGroupListConfigData)
assert tree.colloscope_enabled is False
assert tree.all_groups_enabled is False
assert tree.automatic_groups_enabled is True
assert tree.prefilled_groups_enabled is True
assert tree.per_group_list_enabled is False

# The extra colors come out as a plain dict — the read surface hands a
# read-only mapping, because reading is reading the document; a value is a
# builder, and it can be written as well as read.
extra_colors = tree.colloscope_config.extra_colors
assert type(extra_colors).__name__ == "dict"
assert isinstance(extra_colors["Vacances"], collomatique.Color)
extra_colors_copy = dict(extra_colors)
extra_colors_copy["Vacances"] = collomatique.Color(1, 2, 3)
extra_colors_copy["Examens"] = collomatique.Color(200, 0, 0)
assert extra_colors["Vacances"] != collomatique.Color(1, 2, 3)

# The auto-detected orientation of the all-groups sheet reads as `None`, the
# model's own spelling for "chosen from the group count when the sheet is
# written".
assert tree.all_groups_config.orientation is None
assert tree.automatic_groups_config.orientation == collomatique.Orientation.PORTRAIT
assert tree.prefilled_groups_config.orientation == collomatique.Orientation.LANDSCAPE
assert tree.colloscope_config.orientation == collomatique.Orientation.PORTRAIT
assert tree.per_group_list_config.orientation == collomatique.Orientation.LANDSCAPE

# Mutating a detached value is a real mutation — the point of the dataclasses
# — and the document does not see it. Rust compares the modified tree against
# the same configuration with the stripes repainted.
mutated = config.to_data()
mutated.global_config.stripes_color = collomatique.Color(9, 9, 9)
assert config.global_config.stripes_color != mutated.global_config.stripes_color

# A fresh object every call. Two of them are equal and share nothing.
fresh = config.to_data()
assert fresh == tree
assert fresh is not tree
assert fresh.global_config is not tree.global_config

# A value has no identity: an id names a place in a document, and a value has
# none.
assert not hasattr(tree, "id")

# Each section, detached on its own — what a script that wants one section
# calls instead of the whole tree.
global_value = config.global_config.to_data()
assert isinstance(global_value, collomatique.ExportGlobalConfigData)
assert global_value == tree.global_config
colloscope_value = config.colloscope_config.to_data()
assert isinstance(colloscope_value, collomatique.ExportColloscopeConfigData)
assert colloscope_value == tree.colloscope_config
all_groups_value = config.all_groups_config.to_data()
assert isinstance(all_groups_value, collomatique.ExportStudentGroupsConfigData)
assert all_groups_value == tree.all_groups_config
automatic_value = config.automatic_groups_config.to_data()
assert isinstance(automatic_value, collomatique.ExportStudentGroupsConfigData)
assert automatic_value == tree.automatic_groups_config
prefilled_value = config.prefilled_groups_config.to_data()
assert isinstance(prefilled_value, collomatique.ExportStudentGroupsConfigData)
assert prefilled_value == tree.prefilled_groups_config
group_list_value = config.per_group_list_config.to_data()
assert isinstance(group_list_value, collomatique.ExportGroupListConfigData)
assert group_list_value == tree.per_group_list_config

# The model has three constructors rather than one default for the
# per-student-groups sheets, and the dataclass mirrors them as three
# classmethods. `sheet_name` is the one required field of the whole family.
assert collomatique.ExportStudentGroupsConfigData.all_groups().sheet_name == "Tous les groupes"
assert collomatique.ExportStudentGroupsConfigData.automatic_groups().sheet_name == "Groupes automatiques"
assert collomatique.ExportStudentGroupsConfigData.prefilled_groups().sheet_name == "Groupes préremplis"
assert collomatique.ExportStudentGroupsConfigData.all_groups() == \
    collomatique.ExportStudentGroupsConfigData("Tous les groupes")
try:
    collomatique.ExportStudentGroupsConfigData()
except TypeError:
    pass
else:
    raise AssertionError("sheet_name says which sheet a value is for, and it is required")

# The field order, which is what a positional call depends on: the shared
# settings and the five enabled flags first, then the four per-sheet configs.
assert [f.name for f in dataclasses.fields(collomatique.ExportConfigData)] == [
    "global_config",
    "colloscope_enabled",
    "all_groups_enabled",
    "automatic_groups_enabled",
    "prefilled_groups_enabled",
    "per_group_list_enabled",
    "colloscope_config",
    "all_groups_config",
    "automatic_groups_config",
    "prefilled_groups_config",
    "per_group_list_config",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import ExportConfigData as _same_class  # noqa: E402

assert _same_class is collomatique.ExportConfigData
assert collomatique.ExportConfigData.__module__ == "collomatique"

# The defaults: every field the model's own. Rust pins each of them against
# the model's own builders — the six section-level ones, the three
# classmethods included, and the whole tree.
defaults_global = collomatique.ExportGlobalConfigData()
defaults_colloscope = collomatique.ExportColloscopeConfigData()
defaults_student_all = collomatique.ExportStudentGroupsConfigData.all_groups()
defaults_student_automatic = collomatique.ExportStudentGroupsConfigData.automatic_groups()
defaults_student_prefilled = collomatique.ExportStudentGroupsConfigData.prefilled_groups()
defaults_group_list = collomatique.ExportGroupListConfigData()
defaults_tree = collomatique.ExportConfigData()
assert defaults_tree.global_config == defaults_global
assert defaults_tree.colloscope_config == defaults_colloscope
assert defaults_tree.all_groups_config == defaults_student_all
assert defaults_tree.automatic_groups_config == defaults_student_automatic
assert defaults_tree.prefilled_groups_config == defaults_student_prefilled
assert defaults_tree.per_group_list_config == defaults_group_list

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
bad_global = collomatique.ExportGlobalConfigData(background_color="blanc")
bad_orientation = collomatique.ExportColloscopeConfigData(orientation="auto")
bad_colors = collomatique.ExportColloscopeConfigData(extra_colors={"Vacances": "jaune"})
bad_map = collomatique.ExportColloscopeConfigData(extra_colors=["Vacances"])
bad_student_orientation = collomatique.ExportStudentGroupsConfigData("Tous", orientation="auto")
