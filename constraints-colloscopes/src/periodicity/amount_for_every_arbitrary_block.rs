use crate::ids::GlobalWeek;
use crate::native_extras::{MyBundle, subject_interrogation_params};
use crate::types::ConstraintDesc;
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::subjects::{SubjectPeriodicity, WeekBlock};

use super::helpers::{
    all_active_global_weeks, count_interrogations_expr, enrolled_students_for_subject,
    last_global_week, slot_week_pairs_for_subject,
};

struct BlockRange {
    first_week: GlobalWeek,
    last_week: GlobalWeek,
    count_min: u32,
    count_max: u32,
}

fn compute_block_ranges(blocks: &[WeekBlock]) -> Vec<BlockRange> {
    let mut ranges = Vec::new();
    let mut prev_last: Option<u32> = None;
    for block in blocks {
        let first = match prev_last {
            None => block.delay_in_weeks,
            Some(pl) => pl + 1 + block.delay_in_weeks,
        };
        let last = first + block.size_in_weeks.get() - 1;
        ranges.push(BlockRange {
            first_week: GlobalWeek(first as usize),
            last_week: GlobalWeek(last as usize),
            count_min: *block.interrogation_count_in_block.start(),
            count_max: *block.interrogation_count_in_block.end(),
        });
        prev_last = Some(last);
    }
    ranges
}

fn compute_gap_ranges(
    block_ranges: &[BlockRange],
    last_week: GlobalWeek,
) -> Vec<(GlobalWeek, GlobalWeek)> {
    let mut gaps = Vec::new();
    if block_ranges.is_empty() {
        return gaps;
    }

    let first_block_start = block_ranges[0].first_week.0;
    if first_block_start > 0 {
        gaps.push((GlobalWeek(0), GlobalWeek(first_block_start - 1)));
    }

    for window in block_ranges.windows(2) {
        let gap_first = window[0].last_week.0 + 1;
        if let Some(gap_last) = window[1].first_week.0.checked_sub(1) {
            if gap_first <= gap_last {
                gaps.push((GlobalWeek(gap_first), GlobalWeek(gap_last)));
            }
        }
    }

    let last_block_end = block_ranges.last().unwrap().last_week.0;
    if last_block_end < last_week.0 {
        gaps.push((GlobalWeek(last_block_end + 1), last_week));
    }

    gaps
}

pub(super) fn build(env: &VarEnv, mut bundle: MyBundle) -> MyBundle {
    let lw = last_global_week(env);
    let all_active_weeks = all_active_global_weeks(env);

    for (subject_id, subject) in &env.subjects.ordered_subject_list {
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let SubjectPeriodicity::AmountForEveryArbitraryBlock {
            blocks,
            minimum_week_separation,
        } = &params.periodicity
        else {
            continue;
        };

        let min_sep = *minimum_week_separation as usize;
        let block_ranges = compute_block_ranges(blocks);
        let gap_ranges = compute_gap_ranges(&block_ranges, lw);
        let slot_week_pairs =
            slot_week_pairs_for_subject(env, *subject_id, &subject.excluded_periods);
        let enrolled = enrolled_students_for_subject(env, *subject_id);

        for &student in &enrolled {
            for br in &block_ranges {
                let count_expr = count_interrogations_expr(
                    &slot_week_pairs,
                    student,
                    br.first_week,
                    br.last_week,
                );

                if br.count_min == br.count_max {
                    bundle = bundle.with_constraint(
                        count_expr.eq(&IntLinExpr::constant(i64::from(br.count_min))),
                        ConstraintDesc::PeriodicityInterrogationCountExact {
                            student,
                            subject: *subject_id,
                            first_week: br.first_week,
                            last_week: br.last_week,
                            count: br.count_min,
                        },
                    );
                    bundle = bundle.with_constraint(
                        count_expr.leq(&IntLinExpr::constant(i64::from(br.count_max))),
                        ConstraintDesc::PeriodicityInterrogationCountMax {
                            student,
                            subject: *subject_id,
                            first_week: br.first_week,
                            last_week: br.last_week,
                            max_count: br.count_max,
                        },
                    );
                } else {
                    if br.count_min > 0 {
                        bundle = bundle.with_constraint(
                            count_expr.geq(&IntLinExpr::constant(i64::from(br.count_min))),
                            ConstraintDesc::PeriodicityInterrogationCountMin {
                                student,
                                subject: *subject_id,
                                first_week: br.first_week,
                                last_week: br.last_week,
                                min_count: br.count_min,
                            },
                        );
                    }
                    bundle = bundle.with_constraint(
                        count_expr.leq(&IntLinExpr::constant(i64::from(br.count_max))),
                        ConstraintDesc::PeriodicityInterrogationCountMax {
                            student,
                            subject: *subject_id,
                            first_week: br.first_week,
                            last_week: br.last_week,
                            max_count: br.count_max,
                        },
                    );
                }
            }

            for &(gap_first, gap_last) in &gap_ranges {
                let gap_expr =
                    count_interrogations_expr(&slot_week_pairs, student, gap_first, gap_last);
                bundle = bundle.with_constraint(
                    gap_expr.eq(&IntLinExpr::constant(0)),
                    ConstraintDesc::PeriodicityInterrogationCountExact {
                        student,
                        subject: *subject_id,
                        first_week: gap_first,
                        last_week: gap_last,
                        count: 0,
                    },
                );
                bundle = bundle.with_constraint(
                    gap_expr.leq(&IntLinExpr::constant(0)),
                    ConstraintDesc::PeriodicityInterrogationCountMax {
                        student,
                        subject: *subject_id,
                        first_week: gap_first,
                        last_week: gap_last,
                        max_count: 0,
                    },
                );
            }

            if min_sep > 0 {
                for window in all_active_weeks.windows(min_sep) {
                    let win_first = window[0];
                    let win_last = window[window.len() - 1];
                    let sep_expr =
                        count_interrogations_expr(&slot_week_pairs, student, win_first, win_last);
                    bundle = bundle.with_constraint(
                        sep_expr.leq(&IntLinExpr::constant(1)),
                        ConstraintDesc::PeriodicitySeparation {
                            student,
                            subject: *subject_id,
                            first_week: win_first,
                            last_week: win_last,
                        },
                    );
                }
            }
        }
    }
    bundle
}
