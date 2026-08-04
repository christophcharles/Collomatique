//! Walking the ids of a document (spec §3)
//!
//! The spec gives ids a life of their own: they live in one flat space
//! shared by every kind of entity, they are at most 2⁶³ − 1, and they are
//! the only thing that ties two blocks together. This module is the one
//! place that knows *where* the ids are — every id-valued field of every
//! block, defining and referencing alike — so that whole-document id
//! work (checking the ceiling before writing, renumbering) is written
//! once, over the format structs, and cannot silently miss a field when
//! a block gains one.
//!
//! What is deliberately **not** an id, and therefore not visited: the
//! colloscope's global week index and group numbers, the group numbers
//! of a placement, the positional week bitmask of a week pattern, and
//! the limit values of the settings. Week ids do not appear at all —
//! weeks are positional in the file and their ids are synthesized by the
//! decoder.

use super::Blocks;
use super::keyed::{KeyedRow, KeyedVec, UniqueVec};

use std::collections::{BTreeMap, BTreeSet};

/// Visits every id-valued field of the document, in block order
///
/// The ids are handed over as `&mut u64` so a caller may rewrite them in
/// place; a caller that only reads simply ignores that.
pub fn visit_ids(blocks: &mut Blocks, f: &mut impl FnMut(&mut u64)) {
    if let Some(block) = &mut blocks.general_planning {
        for period in block.periods.iter_mut() {
            f(&mut period.id);
        }
    }
    if let Some(block) = &mut blocks.subjects {
        for subject in block.iter_mut() {
            f(&mut subject.id);
            visit_unique(&mut subject.excluded_periods, f);
        }
    }
    if let Some(block) = &mut blocks.teachers {
        visit_keyed(block, |teacher| {
            f(&mut teacher.id);
            visit_unique(&mut teacher.subjects, f);
        });
    }
    if let Some(block) = &mut blocks.students {
        visit_keyed(block, |student| {
            f(&mut student.id);
            visit_unique(&mut student.excluded_periods, f);
        });
    }
    if let Some(block) = &mut blocks.assignments {
        visit_keyed(block, |row| {
            f(&mut row.period_id);
            f(&mut row.subject_id);
            visit_unique(&mut row.students, f);
        });
    }
    if let Some(block) = &mut blocks.week_patterns {
        visit_keyed(block, |week_pattern| {
            f(&mut week_pattern.id);
        });
    }
    if let Some(block) = &mut blocks.slots {
        visit_keyed(block, |row| {
            f(&mut row.subject_id);
            for slot in row.slots.iter_mut() {
                f(&mut slot.id);
                f(&mut slot.teacher_id);
                if let Some(week_pattern_id) = &mut slot.week_pattern_id {
                    f(week_pattern_id);
                }
            }
        });
    }
    if let Some(block) = &mut blocks.incompatibilities {
        visit_keyed(block, |incompat| {
            f(&mut incompat.id);
            f(&mut incompat.subject_id);
            if let Some(week_pattern_id) = &mut incompat.week_pattern_id {
                f(week_pattern_id);
            }
        });
    }
    if let Some(block) = &mut blocks.group_lists {
        visit_keyed(block, |group_list| {
            f(&mut group_list.id);
            match &mut group_list.filling {
                super::group_lists::Filling::Prefilled(prefilled) => {
                    for group in prefilled.groups.iter_mut() {
                        visit_unique(&mut group.students, f);
                    }
                }
                super::group_lists::Filling::Automatic(automatic) => {
                    visit_unique(&mut automatic.excluded_students, f);
                }
            }
        });
    }
    if let Some(block) = &mut blocks.group_list_associations {
        visit_keyed(block, |row| {
            f(&mut row.period_id);
            f(&mut row.subject_id);
            f(&mut row.group_list_id);
        });
    }
    if let Some(block) = &mut blocks.pairings {
        visit_keyed(block, |rule| {
            f(&mut rule.id);
            f(&mut rule.antecedent.subject_id);
            f(&mut rule.consequent.subject_id);
            visit_unique(&mut rule.excluded_periods, f);
        });
    }
    if let Some(block) = &mut blocks.slot_pairings {
        visit_keyed(block, |rule| {
            f(&mut rule.id);
            f(&mut rule.antecedent.slot_id);
            f(&mut rule.consequent.slot_id);
            visit_unique(&mut rule.excluded_periods, f);
        });
    }
    if let Some(block) = &mut blocks.settings {
        visit_keyed(&mut block.students, |row| {
            f(&mut row.student_id);
        });
    }
    if let Some(block) = &mut blocks.balancing {
        visit_keyed(&mut block.subjects, |row| {
            f(&mut row.subject_id);
        });
    }
    if let Some(block) = &mut blocks.colloscope {
        visit_keyed(&mut block.interrogations, |row| {
            f(&mut row.slot_id);
        });
        visit_keyed(&mut block.group_lists, |row| {
            f(&mut row.group_list_id);
            visit_keyed(&mut row.students, |placement| {
                f(&mut placement.student_id);
            });
        });
    }
    // ExportConfig holds no ids at all.
}

/// Renumbers every id of the document densely from 0, in ascending order
/// of the old values
///
/// The map is built from the very same walk that applies it, so it covers
/// every id the document holds — defining and referencing alike — and the
/// rewrite cannot fail. It is strictly monotone, so it preserves every
/// order the canonical form is built on (rows sorted by id, ascending id
/// sets) and, being injective, it preserves distinctness: two ids that
/// differed still differ, and two ids that were equal — the same entity
/// named twice, or a duplicate — stay equal.
///
/// This is deliberately *not* a repair pass. A dangling reference is an
/// id no entity defines; after renumbering it is still an id no entity
/// defines, because the map is injective. A duplicated id is likewise
/// still duplicated. Only the *values* change, so only the one rule that
/// is about values — the 2^63 - 1 ceiling of spec §3 — is affected.
pub fn remap_ids(blocks: &mut Blocks) {
    let mut all_ids = BTreeSet::new();
    visit_ids(blocks, &mut |id| {
        all_ids.insert(*id);
    });
    let map: BTreeMap<u64, u64> = all_ids.into_iter().zip(0u64..).collect();
    visit_ids(blocks, &mut |id| {
        *id = *map
            .get(id)
            .expect("The map was built from this very walk, so it covers every id");
    });
}

/// Visits the rows of a keyed collection, mutably
///
/// The contents of a [KeyedVec] are private on purpose (the uniqueness
/// of the keys is its invariant), so mutating means taking the rows out
/// and rebuilding. The rebuild re-checks uniqueness, which is a free
/// safety net: every caller here either leaves the ids alone or rewrites
/// them injectively, and both preserve distinct keys.
fn visit_keyed<R: KeyedRow>(vec: &mut KeyedVec<R>, mut visit_row: impl FnMut(&mut R)) {
    let mut rows = std::mem::take(vec).into_inner();
    for row in rows.iter_mut() {
        visit_row(row);
    }
    *vec = KeyedVec::new(rows).expect("An injective id rewrite keeps the keys distinct");
}

/// Visits the elements of an id set, mutably (see [visit_keyed] for why
/// this rebuilds rather than mutates in place)
fn visit_unique(vec: &mut UniqueVec<u64>, f: &mut impl FnMut(&mut u64)) {
    let mut elements = std::mem::take(vec).into_inner();
    for element in elements.iter_mut() {
        f(element);
    }
    *vec = UniqueVec::new(elements).expect("An injective id rewrite keeps the elements distinct");
}
