pub const GROUP_COUNT_PER_INTERROGATION: &'static str =
    include_str!("constraints/group_count_per_interrogation.collo-ml");

pub const STUDENTS_PER_GROUP: &'static str =
    include_str!("constraints/students_per_group.collo-ml");

pub const DEFAULT_CONSTRAINT_LIST: &'static [(&'static str, &'static str)] = &[
    (
        "group_count_per_interrogation",
        GROUP_COUNT_PER_INTERROGATION,
    ),
    ("students_per_group", STUDENTS_PER_GROUP),
];
