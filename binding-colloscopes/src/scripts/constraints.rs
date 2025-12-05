pub const GROUP_COUNT_PER_INTERROGATION: &'static str =
    include_str!("constraints/group_count_per_interrogation.collo-ml");

pub const STUDENTS_PER_GROUP: &'static str =
    include_str!("constraints/students_per_group.collo-ml");

pub const GROUPS_FILLED_BY_ASCENDING_ORDER: &'static str =
    include_str!("constraints/groups_filled_by_ascending_order.collo-ml");

pub const GROUP_COUNT_PER_GROUP_LIST: &'static str =
    include_str!("constraints/group_count_per_group_list.collo-ml");

pub const SEALED_GROUPS: &'static str = include_str!("constraints/sealed_groups.collo-ml");

pub const ONE_INTERROGATION_AT_ONCE: &'static str =
    include_str!("constraints/one_interrogation_at_once.collo-ml");

pub const LIMITS: &'static str = include_str!("constraints/limits.collo-ml");

pub const DEFAULT_CONSTRAINT_LIST: &'static [(&'static str, &'static str)] = &[
    (
        "group_count_per_interrogation",
        GROUP_COUNT_PER_INTERROGATION,
    ),
    ("students_per_group", STUDENTS_PER_GROUP),
    (
        "groups_filled_by_ascending_order",
        GROUPS_FILLED_BY_ASCENDING_ORDER,
    ),
    ("group_count_per_group_list", GROUP_COUNT_PER_GROUP_LIST),
    ("sealed_groups", SEALED_GROUPS),
    ("one_interrogation_at_once", ONE_INTERROGATION_AT_ONCE),
    ("limits", LIMITS),
];
