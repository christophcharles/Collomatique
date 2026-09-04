//! Walking the ids of a document (spec §3)
//!
//! The spec gives ids a life of their own: they live in one flat space
//! shared by every kind of entity, they are at most 2⁶³ − 1, and they are
//! the only thing that ties two blocks together. This module is the one
//! place that knows *where* the ids are — every id-valued field of every
//! block, defining and referencing alike — so that whole-document id
//! work (checking the ceiling before writing) is written once, over the
//! format structs, and cannot silently miss a field when a block gains
//! one.
//!
//! What is deliberately **not** an id, and therefore not visited: the
//! colloscope's group numbers, the group numbers of a placement, and the
//! limit values of the settings.

use super::Blocks;

/// Visits every id-valued field of the document, in block order
pub fn visit_ids(blocks: &Blocks, f: &mut impl FnMut(u64)) {
    if let Some(block) = &blocks.general_planning {
        for period in block.periods.iter() {
            f(period.id);
            for week in period.weeks.iter() {
                f(week.id);
            }
        }
    }
    if let Some(block) = &blocks.subjects {
        for subject in block.iter() {
            f(subject.id);
            visit_unique(&subject.excluded_periods, f);
        }
    }
    if let Some(block) = &blocks.teachers {
        for teacher in block.iter() {
            f(teacher.id);
            visit_unique(&teacher.subjects, f);
        }
    }
    if let Some(block) = &blocks.students {
        for student in block.iter() {
            f(student.id);
            visit_unique(&student.excluded_periods, f);
        }
    }
    if let Some(block) = &blocks.assignments {
        for row in block.iter() {
            f(row.period_id);
            f(row.subject_id);
            visit_unique(&row.students, f);
        }
    }
    if let Some(block) = &blocks.week_patterns {
        for week_pattern in block.iter() {
            f(week_pattern.id);
            visit_unique(&week_pattern.excluded_weeks, f);
        }
    }
    if let Some(block) = &blocks.slots {
        for row in block.iter() {
            f(row.subject_id);
            for slot in row.slots.iter() {
                f(slot.id);
                f(slot.teacher_id);
                if let Some(week_pattern_id) = slot.week_pattern_id {
                    f(week_pattern_id);
                }
            }
        }
    }
    if let Some(block) = &blocks.incompatibilities {
        for incompat in block.iter() {
            f(incompat.id);
            f(incompat.subject_id);
            if let Some(week_pattern_id) = incompat.week_pattern_id {
                f(week_pattern_id);
            }
        }
    }
    if let Some(block) = &blocks.group_lists {
        for group_list in block.iter() {
            f(group_list.id);
            match &group_list.filling {
                super::group_lists::Filling::Prefilled(prefilled) => {
                    for group in prefilled.groups.iter() {
                        visit_unique(&group.students, f);
                    }
                }
                super::group_lists::Filling::Automatic(automatic) => {
                    visit_unique(&automatic.excluded_students, f);
                }
            }
        }
    }
    if let Some(block) = &blocks.group_list_associations {
        for row in block.iter() {
            f(row.period_id);
            f(row.subject_id);
            f(row.group_list_id);
        }
    }
    if let Some(block) = &blocks.pairings {
        for rule in block.iter() {
            f(rule.id);
            f(rule.antecedent.subject_id);
            f(rule.consequent.subject_id);
            visit_unique(&rule.excluded_periods, f);
        }
    }
    if let Some(block) = &blocks.slot_pairings {
        for rule in block.iter() {
            f(rule.id);
            f(rule.antecedent.slot_id);
            f(rule.consequent.slot_id);
            visit_unique(&rule.excluded_periods, f);
        }
    }
    if let Some(block) = &blocks.settings {
        for row in block.students.iter() {
            f(row.student_id);
        }
    }
    if let Some(block) = &blocks.balancing {
        for row in block.subjects.iter() {
            f(row.subject_id);
        }
    }
    if let Some(block) = &blocks.colloscope {
        for row in block.interrogations.iter() {
            f(row.slot_id);
            f(row.week_id);
        }
        for row in block.group_lists.iter() {
            f(row.group_list_id);
            for placement in row.students.iter() {
                f(placement.student_id);
            }
        }
    }
    // ExportConfig holds no ids at all.
    if let Some(block) = &blocks.subject_week_patterns {
        for row in block.iter() {
            f(row.subject_id);
            f(row.week_pattern_id);
        }
    }
}

/// Visits the elements of an id set
fn visit_unique(vec: &super::keyed::UniqueVec<u64>, f: &mut impl FnMut(u64)) {
    for element in vec.iter() {
        f(*element);
    }
}
