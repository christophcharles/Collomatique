pub const INTERROGATION_HAS_GROUPS: &'static str =
    include_str!("reifications/interrogation_has_groups.collo-ml");

pub const STUDENT_IN_GROUP: &'static str = include_str!("reifications/student_in_group.collo-ml");

pub const GROUP_HAS_STUDENTS: &'static str =
    include_str!("reifications/group_has_students.collo-ml");

pub const STUDENT_AT_INTERROGATION: &'static str =
    include_str!("reifications/student_at_interrogation.collo-ml");

// Warning! Order matters!
pub const DEFAULT_REIFICATION_LIST: &'static [(&'static str, &'static str)] = &[
    ("interrogation_has_groups", INTERROGATION_HAS_GROUPS),
    ("student_in_group", STUDENT_IN_GROUP),
    ("group_has_students", GROUP_HAS_STUDENTS),
    ("student_at_interrogation", STUDENT_AT_INTERROGATION),
];
