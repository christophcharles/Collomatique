//! SQL schema definition for SQLite state storage

/// Complete SQL schema for the Collomatique database
pub const SCHEMA_SQL: &str = r#"
-- ============================================================================
-- PRAGMA settings for better performance and safety
-- ============================================================================

PRAGMA foreign_keys = ON;

-- ============================================================================
-- 1. Global ID Tracking
-- ============================================================================

CREATE TABLE all_ids (
    id INTEGER NOT NULL PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (entity_type IN (
        'student', 'teacher', 'subject', 'period',
        'slot', 'week_pattern', 'incompat', 'group_list'
    ))
);

-- ============================================================================
-- 2. Metadata
-- ============================================================================

CREATE TABLE metadata (
    id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    first_week TEXT,
    main_script TEXT
);

-- ============================================================================
-- 3. Periods
-- ============================================================================

CREATE TABLE periods (
    id INTEGER NOT NULL PRIMARY KEY,
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0)
);

CREATE TABLE period_weeks (
    period_id INTEGER NOT NULL REFERENCES periods(id) ON DELETE CASCADE,
    week_index INTEGER NOT NULL CHECK (week_index >= 0),
    has_interrogations INTEGER NOT NULL CHECK (has_interrogations IN (0, 1)),
    annotation TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (period_id, week_index)
);

-- ============================================================================
-- 4. Subjects
-- ============================================================================

CREATE TABLE subjects (
    id INTEGER NOT NULL PRIMARY KEY,
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    name TEXT NOT NULL
);

CREATE TABLE subject_interrogation_params (
    subject_id INTEGER NOT NULL PRIMARY KEY REFERENCES subjects(id) ON DELETE CASCADE,
    students_per_group_min INTEGER NOT NULL CHECK (students_per_group_min > 0),
    students_per_group_max INTEGER NOT NULL CHECK (students_per_group_max >= students_per_group_min),
    groups_per_interrogation_min INTEGER NOT NULL CHECK (groups_per_interrogation_min > 0),
    groups_per_interrogation_max INTEGER NOT NULL CHECK (groups_per_interrogation_max >= groups_per_interrogation_min),
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    take_duration_into_account INTEGER NOT NULL CHECK (take_duration_into_account IN (0, 1)),

    -- ExactlyPeriodic (1 field)
    ep_periodicity_in_weeks INTEGER CHECK (ep_periodicity_in_weeks > 0),

    -- OnceForEveryBlockOfWeeks (2 fields)
    ofeb_weeks_per_block INTEGER CHECK (ofeb_weeks_per_block > 0),
    ofeb_minimum_week_separation INTEGER CHECK (ofeb_minimum_week_separation > 0),

    -- AmountInYear (3 fields)
    aiy_count_min INTEGER CHECK (aiy_count_min >= 0),
    aiy_count_max INTEGER CHECK (aiy_count_max >= aiy_count_min),
    aiy_minimum_week_separation INTEGER CHECK (aiy_minimum_week_separation >= 0),

    -- AmountForEveryArbitraryBlock (1 field here, blocks in separate table)
    afab_minimum_week_separation INTEGER CHECK (afab_minimum_week_separation >= 0),

    -- Consistency: OnceForEveryBlockOfWeeks fields must be all NULL or all non-NULL
    CHECK (
        (ofeb_weeks_per_block IS NULL) = (ofeb_minimum_week_separation IS NULL)
    ),

    -- Consistency: AmountInYear fields must be all NULL or all non-NULL
    CHECK (
        (aiy_count_min IS NULL) = (aiy_count_max IS NULL) AND
        (aiy_count_min IS NULL) = (aiy_minimum_week_separation IS NULL)
    ),

    -- Exactly one periodicity type must be set
    CHECK (
        (ep_periodicity_in_weeks IS NOT NULL) +
        (ofeb_weeks_per_block IS NOT NULL) +
        (aiy_count_min IS NOT NULL) +
        (afab_minimum_week_separation IS NOT NULL) = 1
    )
);

CREATE TABLE subject_excluded_periods (
    subject_id INTEGER NOT NULL REFERENCES subjects(id) ON DELETE CASCADE,
    period_id INTEGER NOT NULL REFERENCES periods(id) ON DELETE RESTRICT,
    PRIMARY KEY (subject_id, period_id)
);

CREATE TABLE periodicity_week_blocks (
    subject_id INTEGER NOT NULL REFERENCES subject_interrogation_params(subject_id) ON DELETE CASCADE,
    block_index INTEGER NOT NULL CHECK (block_index >= 0),
    delay_in_weeks INTEGER NOT NULL CHECK (delay_in_weeks >= 0),
    size_in_weeks INTEGER NOT NULL CHECK (size_in_weeks > 0),
    interrogation_count_min INTEGER NOT NULL CHECK (interrogation_count_min >= 0),
    interrogation_count_max INTEGER NOT NULL CHECK (interrogation_count_max >= interrogation_count_min),
    PRIMARY KEY (subject_id, block_index)
);

-- ============================================================================
-- 5. Students
-- ============================================================================

CREATE TABLE students (
    id INTEGER NOT NULL PRIMARY KEY,
    surname TEXT NOT NULL,
    firstname TEXT NOT NULL,
    tel TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT ''
);

CREATE TABLE student_excluded_periods (
    student_id INTEGER NOT NULL REFERENCES students(id) ON DELETE CASCADE,
    period_id INTEGER NOT NULL REFERENCES periods(id) ON DELETE RESTRICT,
    PRIMARY KEY (student_id, period_id)
);

-- ============================================================================
-- 6. Teachers
-- ============================================================================

CREATE TABLE teachers (
    id INTEGER NOT NULL PRIMARY KEY,
    surname TEXT NOT NULL,
    firstname TEXT NOT NULL,
    tel TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT ''
);

CREATE TABLE teacher_subjects (
    teacher_id INTEGER NOT NULL REFERENCES teachers(id) ON DELETE CASCADE,
    subject_id INTEGER NOT NULL REFERENCES subject_interrogation_params(subject_id) ON DELETE RESTRICT,
    PRIMARY KEY (teacher_id, subject_id)
);

-- ============================================================================
-- 7. Week Patterns
-- ============================================================================

CREATE TABLE week_patterns (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE week_pattern_disabled_weeks (
    week_pattern_id INTEGER NOT NULL REFERENCES week_patterns(id) ON DELETE CASCADE,
    period_id INTEGER NOT NULL,
    week_index INTEGER NOT NULL,
    PRIMARY KEY (week_pattern_id, period_id, week_index),
    FOREIGN KEY (period_id, week_index) REFERENCES period_weeks(period_id, week_index) ON DELETE RESTRICT
);

-- ============================================================================
-- 8. Slots
-- ============================================================================

CREATE TABLE slots (
    id INTEGER NOT NULL PRIMARY KEY,
    subject_id INTEGER NOT NULL REFERENCES subject_interrogation_params(subject_id) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    teacher_id INTEGER NOT NULL REFERENCES teachers(id) ON DELETE RESTRICT,
    day INTEGER NOT NULL CHECK (day >= 0 AND day <= 6),
    start_time INTEGER NOT NULL CHECK (start_time >= 0 AND start_time < 1440),
    extra_info TEXT NOT NULL DEFAULT '',
    week_pattern_id INTEGER REFERENCES week_patterns(id) ON DELETE RESTRICT,
    cost INTEGER NOT NULL DEFAULT 0,
    UNIQUE (subject_id, position)
);

-- ============================================================================
-- 9. Incompatibilities
-- ============================================================================

CREATE TABLE incompats (
    id INTEGER NOT NULL PRIMARY KEY,
    subject_id INTEGER NOT NULL REFERENCES subjects(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    minimum_free_slots INTEGER NOT NULL CHECK (minimum_free_slots > 0),
    week_pattern_id INTEGER REFERENCES week_patterns(id) ON DELETE RESTRICT
);

CREATE TABLE incompat_slots (
    incompat_id INTEGER NOT NULL REFERENCES incompats(id) ON DELETE CASCADE,
    slot_index INTEGER NOT NULL CHECK (slot_index >= 0),
    day INTEGER NOT NULL CHECK (day >= 0 AND day <= 6),
    start_time INTEGER NOT NULL CHECK (start_time >= 0 AND start_time < 1440),
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    CHECK (start_time + duration_minutes <= 1440),
    PRIMARY KEY (incompat_id, slot_index)
);

-- ============================================================================
-- 10. Group Lists
-- ============================================================================

CREATE TABLE group_lists (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    students_per_group_min INTEGER NOT NULL CHECK (students_per_group_min > 0),
    students_per_group_max INTEGER NOT NULL CHECK (students_per_group_max >= students_per_group_min),
    filling_type TEXT NOT NULL CHECK (filling_type IN ('prefilled', 'automatic'))
);

CREATE TABLE group_list_group_names (
    group_list_id INTEGER NOT NULL REFERENCES group_lists(id) ON DELETE CASCADE,
    group_index INTEGER NOT NULL CHECK (group_index >= 0),
    name TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (group_list_id, group_index)
);

CREATE TABLE prefilled_group_students (
    group_list_id INTEGER NOT NULL REFERENCES group_lists(id) ON DELETE CASCADE,
    group_index INTEGER NOT NULL CHECK (group_index >= 0),
    student_id INTEGER NOT NULL REFERENCES students(id) ON DELETE RESTRICT,
    PRIMARY KEY (group_list_id, student_id),
    FOREIGN KEY (group_list_id, group_index) REFERENCES group_list_group_names(group_list_id, group_index)
);

CREATE TABLE automatic_group_excluded_students (
    group_list_id INTEGER NOT NULL REFERENCES group_lists(id) ON DELETE CASCADE,
    student_id INTEGER NOT NULL REFERENCES students(id) ON DELETE RESTRICT,
    PRIMARY KEY (group_list_id, student_id)
);

CREATE TABLE group_list_subject_associations (
    period_id INTEGER NOT NULL REFERENCES periods(id) ON DELETE RESTRICT,
    subject_id INTEGER NOT NULL REFERENCES subject_interrogation_params(subject_id) ON DELETE RESTRICT,
    group_list_id INTEGER NOT NULL REFERENCES group_lists(id) ON DELETE RESTRICT,
    PRIMARY KEY (period_id, subject_id)
);

-- ============================================================================
-- 11. Assignments
-- ============================================================================

CREATE TABLE assignments (
    period_id INTEGER NOT NULL REFERENCES periods(id) ON DELETE RESTRICT,
    subject_id INTEGER NOT NULL REFERENCES subjects(id) ON DELETE RESTRICT,
    student_id INTEGER NOT NULL REFERENCES students(id) ON DELETE RESTRICT,
    PRIMARY KEY (period_id, subject_id, student_id)
);

-- ============================================================================
-- 12. Settings
-- ============================================================================

CREATE TABLE settings_global (
    id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    interrogations_per_week_min_value INTEGER,
    interrogations_per_week_min_soft INTEGER CHECK (interrogations_per_week_min_soft IN (0, 1)),
    interrogations_per_week_max_value INTEGER,
    interrogations_per_week_max_soft INTEGER CHECK (interrogations_per_week_max_soft IN (0, 1)),
    max_interrogations_per_day_value INTEGER CHECK (max_interrogations_per_day_value > 0),
    max_interrogations_per_day_soft INTEGER CHECK (max_interrogations_per_day_soft IN (0, 1)),
    CHECK ((interrogations_per_week_min_value IS NULL) = (interrogations_per_week_min_soft IS NULL)),
    CHECK ((interrogations_per_week_max_value IS NULL) = (interrogations_per_week_max_soft IS NULL)),
    CHECK ((max_interrogations_per_day_value IS NULL) = (max_interrogations_per_day_soft IS NULL))
);

CREATE TABLE settings_students (
    student_id INTEGER NOT NULL PRIMARY KEY REFERENCES students(id) ON DELETE RESTRICT,
    interrogations_per_week_min_value INTEGER,
    interrogations_per_week_min_soft INTEGER CHECK (interrogations_per_week_min_soft IN (0, 1)),
    interrogations_per_week_max_value INTEGER,
    interrogations_per_week_max_soft INTEGER CHECK (interrogations_per_week_max_soft IN (0, 1)),
    max_interrogations_per_day_value INTEGER CHECK (max_interrogations_per_day_value > 0),
    max_interrogations_per_day_soft INTEGER CHECK (max_interrogations_per_day_soft IN (0, 1)),
    CHECK ((interrogations_per_week_min_value IS NULL) = (interrogations_per_week_min_soft IS NULL)),
    CHECK ((interrogations_per_week_max_value IS NULL) = (interrogations_per_week_max_soft IS NULL)),
    CHECK ((max_interrogations_per_day_value IS NULL) = (max_interrogations_per_day_soft IS NULL))
);

-- View for effective settings with automatic fallback to global defaults
CREATE VIEW settings_effective AS
SELECT
    st.id AS student_id,
    COALESCE(ss.interrogations_per_week_min_value, sg.interrogations_per_week_min_value) AS interrogations_per_week_min_value,
    COALESCE(ss.interrogations_per_week_min_soft, sg.interrogations_per_week_min_soft) AS interrogations_per_week_min_soft,
    COALESCE(ss.interrogations_per_week_max_value, sg.interrogations_per_week_max_value) AS interrogations_per_week_max_value,
    COALESCE(ss.interrogations_per_week_max_soft, sg.interrogations_per_week_max_soft) AS interrogations_per_week_max_soft,
    COALESCE(ss.max_interrogations_per_day_value, sg.max_interrogations_per_day_value) AS max_interrogations_per_day_value,
    COALESCE(ss.max_interrogations_per_day_soft, sg.max_interrogations_per_day_soft) AS max_interrogations_per_day_soft
FROM students st
CROSS JOIN settings_global sg
LEFT JOIN settings_students ss ON ss.student_id = st.id;

-- ============================================================================
-- 13. Colloscope (Schedule Data)
-- ============================================================================

CREATE TABLE colloscope_slots (
    period_id INTEGER NOT NULL REFERENCES periods(id) ON DELETE RESTRICT,
    slot_id INTEGER NOT NULL REFERENCES slots(id) ON DELETE RESTRICT,
    week_index INTEGER NOT NULL CHECK (week_index >= 0),
    has_interrogation INTEGER NOT NULL CHECK (has_interrogation IN (0, 1)),
    PRIMARY KEY (period_id, slot_id, week_index)
);

CREATE TABLE colloscope_interrogation_groups (
    period_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    week_index INTEGER NOT NULL,
    group_number INTEGER NOT NULL CHECK (group_number >= 0),
    PRIMARY KEY (period_id, slot_id, week_index, group_number),
    FOREIGN KEY (period_id, slot_id, week_index) REFERENCES colloscope_slots(period_id, slot_id, week_index) ON DELETE CASCADE
);

CREATE TABLE colloscope_group_list_students (
    group_list_id INTEGER NOT NULL REFERENCES group_lists(id) ON DELETE RESTRICT,
    student_id INTEGER NOT NULL REFERENCES students(id) ON DELETE RESTRICT,
    group_number INTEGER NOT NULL CHECK (group_number >= 0),
    PRIMARY KEY (group_list_id, student_id)
);

-- ============================================================================
-- Triggers for Global ID Uniqueness
-- ============================================================================

-- STUDENTS
CREATE TRIGGER student_id_insert AFTER INSERT ON students
BEGIN
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'student');
END;

CREATE TRIGGER student_id_delete AFTER DELETE ON students
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'student';
END;

CREATE TRIGGER student_id_update AFTER UPDATE OF id ON students
WHEN OLD.id != NEW.id
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'student';
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'student');
END;

-- TEACHERS
CREATE TRIGGER teacher_id_insert AFTER INSERT ON teachers
BEGIN
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'teacher');
END;

CREATE TRIGGER teacher_id_delete AFTER DELETE ON teachers
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'teacher';
END;

CREATE TRIGGER teacher_id_update AFTER UPDATE OF id ON teachers
WHEN OLD.id != NEW.id
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'teacher';
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'teacher');
END;

-- SUBJECTS
CREATE TRIGGER subject_id_insert AFTER INSERT ON subjects
BEGIN
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'subject');
END;

CREATE TRIGGER subject_id_delete AFTER DELETE ON subjects
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'subject';
END;

CREATE TRIGGER subject_id_update AFTER UPDATE OF id ON subjects
WHEN OLD.id != NEW.id
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'subject';
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'subject');
END;

-- PERIODS
CREATE TRIGGER period_id_insert AFTER INSERT ON periods
BEGIN
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'period');
END;

CREATE TRIGGER period_id_delete AFTER DELETE ON periods
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'period';
END;

CREATE TRIGGER period_id_update AFTER UPDATE OF id ON periods
WHEN OLD.id != NEW.id
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'period';
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'period');
END;

-- SLOTS
CREATE TRIGGER slot_id_insert AFTER INSERT ON slots
BEGIN
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'slot');
END;

CREATE TRIGGER slot_id_delete AFTER DELETE ON slots
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'slot';
END;

CREATE TRIGGER slot_id_update AFTER UPDATE OF id ON slots
WHEN OLD.id != NEW.id
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'slot';
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'slot');
END;

-- WEEK_PATTERNS
CREATE TRIGGER week_pattern_id_insert AFTER INSERT ON week_patterns
BEGIN
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'week_pattern');
END;

CREATE TRIGGER week_pattern_id_delete AFTER DELETE ON week_patterns
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'week_pattern';
END;

CREATE TRIGGER week_pattern_id_update AFTER UPDATE OF id ON week_patterns
WHEN OLD.id != NEW.id
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'week_pattern';
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'week_pattern');
END;

-- INCOMPATS
CREATE TRIGGER incompat_id_insert AFTER INSERT ON incompats
BEGIN
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'incompat');
END;

CREATE TRIGGER incompat_id_delete AFTER DELETE ON incompats
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'incompat';
END;

CREATE TRIGGER incompat_id_update AFTER UPDATE OF id ON incompats
WHEN OLD.id != NEW.id
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'incompat';
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'incompat');
END;

-- GROUP_LISTS
CREATE TRIGGER group_list_id_insert AFTER INSERT ON group_lists
BEGIN
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'group_list');
END;

CREATE TRIGGER group_list_id_delete AFTER DELETE ON group_lists
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'group_list';
END;

CREATE TRIGGER group_list_id_update AFTER UPDATE OF id ON group_lists
WHEN OLD.id != NEW.id
BEGIN
    DELETE FROM all_ids WHERE id = OLD.id AND entity_type = 'group_list';
    INSERT INTO all_ids (id, entity_type) VALUES (NEW.id, 'group_list');
END;
"#;
