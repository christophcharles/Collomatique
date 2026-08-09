import collomatique

# `source` is a document written by the test: an export configuration away
# from the default on every field — the shapes the example file never shows, a
# switched-off colloscope sheet, extra colors, an auto-detected orientation.
doc = collomatique.load(source)

config = doc.export_config
assert isinstance(config, collomatique.ExportConfig)
assert repr(config) == "<collomatique.ExportConfig>"

# The five flags, sitting beside the sections they gate — the model's memory
# of what was chosen before a section was switched off.
flags = (
    config.colloscope_enabled,
    config.all_groups_enabled,
    config.automatic_groups_enabled,
    config.prefilled_groups_enabled,
    config.per_group_list_enabled,
)

# The global section.
global_config = config.global_config
assert isinstance(global_config, collomatique.ExportGlobalConfig)
assert repr(global_config) == "<collomatique.ExportGlobalConfig>"
assert global_config == config.global_config
global_reading = (
    (
        global_config.background_color.red,
        global_config.background_color.green,
        global_config.background_color.blue,
    ),
    global_config.stripes_color_enabled,
    (
        global_config.stripes_color.red,
        global_config.stripes_color.green,
        global_config.stripes_color.blue,
    ),
)

# The colloscope section, field by field. The orientations compare as the
# class attributes themselves, and spell their repr like the other enums.
colloscope_config = config.colloscope_config
assert isinstance(colloscope_config, collomatique.ExportColloscopeConfig)
assert repr(colloscope_config) == "<collomatique.ExportColloscopeConfig>"
assert colloscope_config == config.colloscope_config
colloscope_reading = (
    (
        colloscope_config.sheet_name,
        colloscope_config.extra_info_column_enabled,
        colloscope_config.extra_info_column_name,
    ),
    (
        colloscope_config.teacher_email_enabled,
        colloscope_config.teacher_email,
        colloscope_config.teacher_tel_enabled,
        colloscope_config.teacher_tel,
    ),
    (
        repr(colloscope_config.orientation),
        colloscope_config.display_week_dates,
        colloscope_config.display_annotations,
    ),
    (
        colloscope_config.no_interrogation_color.red,
        colloscope_config.no_interrogation_color.green,
        colloscope_config.no_interrogation_color.blue,
    ),
    (
        colloscope_config.annotation_color_enabled,
        colloscope_config.annotation_color.red,
        colloscope_config.annotation_color.green,
        colloscope_config.annotation_color.blue,
    ),
)
assert colloscope_config.orientation == collomatique.Orientation.PORTRAIT
assert colloscope_config.orientation is not collomatique.Orientation.LANDSCAPE

# The extra colors, as a read-only mapping of labels to Color values — a
# `types.MappingProxyType` over a fresh dict, like the colloscope placements.
extra_colors = colloscope_config.extra_colors
assert type(extra_colors).__name__ == "mappingproxy"
assert all(isinstance(color, collomatique.Color) for color in extra_colors.values())
extra_colors_items = tuple(
    (name, (color.red, color.green, color.blue))
    for name, color in sorted(extra_colors.items())
)
try:
    extra_colors["Vacances"] = collomatique.Color(1, 2, 3)
except TypeError:
    pass
else:
    raise AssertionError("the extra colors mapping refuses assignment")
try:
    del extra_colors["Vacances"]
except TypeError:
    pass
else:
    raise AssertionError("the extra colors mapping refuses deletion")

# A color reads back equal to the one a script builds, and refuses nonsense:
# each channel is 0-255.
vacances = extra_colors["Vacances"]
assert vacances == collomatique.Color(13, 14, 15)
assert repr(vacances) == "Color(red=13, green=14, blue=15)"
for bad in (256, -1):
    try:
        collomatique.Color(bad, 0, 0)
    except ValueError:
        pass
    else:
        raise AssertionError("a channel of a Color is 0-255")

# The orientation is an enum of its own: `repr` echoes the member's
# identifier, `str` is the gui's own french word.
assert repr(collomatique.Orientation.PORTRAIT) == "Orientation.PORTRAIT"
assert repr(collomatique.Orientation.LANDSCAPE) == "Orientation.LANDSCAPE"
assert str(collomatique.Orientation.PORTRAIT) == "Portrait"
assert str(collomatique.Orientation.LANDSCAPE) == "Paysage"
assert collomatique.Orientation.PORTRAIT != collomatique.Orientation.LANDSCAPE
assert collomatique.Orientation.PORTRAIT == collomatique.Orientation.PORTRAIT

# The three per-student-groups sections: one class, three instances, told
# apart by the kind — and the auto-detected orientation reads as `None`, not
# as an orientation of its own.
def reading(section):
    orientation = section.orientation
    return (
        section.sheet_name,
        None if orientation is None else repr(orientation),
        section.show_emails,
        section.show_tel,
    )


all_groups = config.all_groups_config
automatic = config.automatic_groups_config
prefilled = config.prefilled_groups_config
for section in (all_groups, automatic, prefilled):
    assert isinstance(section, collomatique.ExportStudentGroupsConfig)
assert repr(all_groups) == "<collomatique.ExportStudentGroupsConfig all_groups>"
assert repr(automatic) == "<collomatique.ExportStudentGroupsConfig automatic_groups>"
assert repr(prefilled) == "<collomatique.ExportStudentGroupsConfig prefilled_groups>"
assert all_groups == config.all_groups_config
assert automatic == config.automatic_groups_config
assert prefilled == config.prefilled_groups_config
assert all_groups != automatic
assert automatic != prefilled
student_groups_readings = (
    reading(all_groups),
    reading(automatic),
    reading(prefilled),
)

# The per-group-list section.
group_list_config = config.per_group_list_config
assert isinstance(group_list_config, collomatique.ExportGroupListConfig)
assert repr(group_list_config) == "<collomatique.ExportGroupListConfig>"
assert group_list_config == config.per_group_list_config
group_list_reading = (
    repr(group_list_config.orientation),
    group_list_config.show_emails,
    group_list_config.show_tel,
    group_list_config.center_vertically,
)
