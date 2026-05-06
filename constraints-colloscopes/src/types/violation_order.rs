use super::*;

#[derive(Debug, Clone, Copy)]
struct WeekRange {
    first_week: Option<GlobalWeek>,
    last_week: Option<GlobalWeek>,
}

impl WeekRange {
    fn bounded(first_week: GlobalWeek, last_week: GlobalWeek) -> Self {
        Self {
            first_week: Some(first_week),
            last_week: Some(last_week),
        }
    }

    fn unbounded() -> Self {
        Self {
            first_week: None,
            last_week: None,
        }
    }

    fn is_subset_of(&self, other: &Self) -> bool {
        let start_ok = match (self.first_week, other.first_week) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(a), Some(b)) => a >= b,
        };
        let end_ok = match (self.last_week, other.last_week) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(a), Some(b)) => a <= b,
        };
        start_ok && end_ok
    }
}

#[derive(Debug, Clone, Copy)]
enum CountBoundKind {
    Upper(u32),
    Lower(u32),
    Exact(u32),
}

#[derive(Debug)]
enum ViolationFamily {
    PeriodicityCount {
        student: StudentId,
        subject: SubjectId,
        range: WeekRange,
        kind: CountBoundKind,
    },
    TeacherRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        range: WeekRange,
        max_count: u32,
    },
    SlotRotation {
        student: StudentId,
        subject: SubjectId,
        slot: SlotId,
        range: WeekRange,
        max_count: u32,
    },
    StudentsInGroup {
        group_list: GroupListId,
        group: GroupNum,
        subject_scope: Option<(SubjectId, PeriodId)>,
        kind: CountBoundKind,
    },
    GroupCount {
        slot: SlotId,
        week: GlobalWeek,
        kind: CountBoundKind,
    },
    InterrogationsPerTimePeriod {
        student: StudentId,
        week: GlobalWeek,
        day: Option<collomatique_time::Weekday>,
        kind: CountBoundKind,
    },
}

impl ConstraintDesc {
    fn violation_family(&self) -> Option<ViolationFamily> {
        match self {
            ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
                student,
                subject,
                first_week,
                last_week,
                max_count,
            }) => Some(ViolationFamily::PeriodicityCount {
                student: *student,
                subject: *subject,
                range: WeekRange::bounded(*first_week, *last_week),
                kind: CountBoundKind::Upper(*max_count),
            }),
            ConstraintDesc::Level2(QualityConstraint::PeriodicitySeparation {
                student,
                subject,
                first_week,
                last_week,
            }) => Some(ViolationFamily::PeriodicityCount {
                student: *student,
                subject: *subject,
                range: WeekRange::bounded(*first_week, *last_week),
                kind: CountBoundKind::Upper(1),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
                student,
                subject,
                first_week,
                last_week,
                min_count,
            }) => Some(ViolationFamily::PeriodicityCount {
                student: *student,
                subject: *subject,
                range: WeekRange::bounded(*first_week, *last_week),
                kind: CountBoundKind::Lower(*min_count),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountExact {
                student,
                subject,
                first_week,
                last_week,
                count,
            }) => Some(ViolationFamily::PeriodicityCount {
                student: *student,
                subject: *subject,
                range: WeekRange::bounded(*first_week, *last_week),
                kind: CountBoundKind::Exact(*count),
            }),

            ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
                student,
                subject,
                teacher,
                first_week,
                last_week,
                max_count,
            }) => Some(ViolationFamily::TeacherRotation {
                student: *student,
                subject: *subject,
                teacher: *teacher,
                range: WeekRange::bounded(*first_week, *last_week),
                max_count: *max_count,
            }),
            ConstraintDesc::Level4(PreferenceConstraint::BalancingPeriodRotation {
                student,
                subject,
                teacher,
                first_week,
                last_week,
                max_count,
                ..
            }) => Some(ViolationFamily::TeacherRotation {
                student: *student,
                subject: *subject,
                teacher: *teacher,
                range: WeekRange::bounded(*first_week, *last_week),
                max_count: *max_count,
            }),
            ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
                student,
                subject,
                teacher,
                max_count,
            }) => Some(ViolationFamily::TeacherRotation {
                student: *student,
                subject: *subject,
                teacher: *teacher,
                range: WeekRange::unbounded(),
                max_count: *max_count,
            }),

            ConstraintDesc::Level4(PreferenceConstraint::BalancingSlotRotation {
                student,
                subject,
                slot,
                first_week,
                last_week,
                max_count,
            }) => Some(ViolationFamily::SlotRotation {
                student: *student,
                subject: *subject,
                slot: *slot,
                range: WeekRange::bounded(*first_week, *last_week),
                max_count: *max_count,
            }),

            ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupMax {
                group_list,
                group,
                max_students,
            }) => Some(ViolationFamily::StudentsInGroup {
                group_list: *group_list,
                group: *group,
                subject_scope: None,
                kind: CountBoundKind::Upper(*max_students),
            }),
            ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupForSubjectMax {
                group_list,
                group,
                subject,
                period,
                max_students,
            }) => Some(ViolationFamily::StudentsInGroup {
                group_list: *group_list,
                group: *group,
                subject_scope: Some((*subject, *period)),
                kind: CountBoundKind::Upper(*max_students),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::StudentsPerGroupMin {
                group_list,
                group,
                min_students,
            }) => Some(ViolationFamily::StudentsInGroup {
                group_list: *group_list,
                group: *group,
                subject_scope: None,
                kind: CountBoundKind::Lower(*min_students),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::StudentsPerGroupForSubjectMin {
                group_list,
                group,
                subject,
                period,
                min_students,
            }) => Some(ViolationFamily::StudentsInGroup {
                group_list: *group_list,
                group: *group,
                subject_scope: Some((*subject, *period)),
                kind: CountBoundKind::Lower(*min_students),
            }),

            ConstraintDesc::Level2(QualityConstraint::GroupCountPerInterrogationMax {
                slot,
                week,
                max_groups,
            }) => Some(ViolationFamily::GroupCount {
                slot: *slot,
                week: *week,
                kind: CountBoundKind::Upper(*max_groups),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::GroupCountPerInterrogationMin {
                slot,
                week,
                min_groups,
            }) => Some(ViolationFamily::GroupCount {
                slot: *slot,
                week: *week,
                kind: CountBoundKind::Lower(*min_groups),
            }),

            ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
                student,
                week,
                day,
                max,
            }) => Some(ViolationFamily::InterrogationsPerTimePeriod {
                student: *student,
                week: *week,
                day: Some(*day),
                kind: CountBoundKind::Upper(*max),
            }),
            ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerWeek {
                student,
                week,
                max,
            }) => Some(ViolationFamily::InterrogationsPerTimePeriod {
                student: *student,
                week: *week,
                day: None,
                kind: CountBoundKind::Upper(*max),
            }),
            ConstraintDesc::Level4(PreferenceConstraint::MinInterrogationsPerWeek {
                student,
                week,
                min,
            }) => Some(ViolationFamily::InterrogationsPerTimePeriod {
                student: *student,
                week: *week,
                day: None,
                kind: CountBoundKind::Lower(*min),
            }),

            _ => None,
        }
    }

    /// Returns true if violating `self` necessarily implies violating `other`.
    pub fn violation_implies(&self, other: &Self) -> bool {
        if self == other {
            return true;
        }

        let (Some(self_fam), Some(other_fam)) = (self.violation_family(), other.violation_family())
        else {
            return false;
        };

        match (&self_fam, &other_fam) {
            (
                ViolationFamily::PeriodicityCount {
                    student: s1,
                    subject: sub1,
                    range: r1,
                    kind: k1,
                },
                ViolationFamily::PeriodicityCount {
                    student: s2,
                    subject: sub2,
                    range: r2,
                    kind: k2,
                },
            ) => s1 == s2 && sub1 == sub2 && ranged_bound_implies(r1, k1, r2, k2),

            (
                ViolationFamily::TeacherRotation {
                    student: s1,
                    subject: sub1,
                    teacher: t1,
                    range: r1,
                    max_count: n1,
                },
                ViolationFamily::TeacherRotation {
                    student: s2,
                    subject: sub2,
                    teacher: t2,
                    range: r2,
                    max_count: n2,
                },
            ) => {
                s1 == s2
                    && sub1 == sub2
                    && t1 == t2
                    && ranged_bound_implies(
                        r1,
                        &CountBoundKind::Upper(*n1),
                        r2,
                        &CountBoundKind::Upper(*n2),
                    )
            }

            (
                ViolationFamily::SlotRotation {
                    student: s1,
                    subject: sub1,
                    slot: sl1,
                    range: r1,
                    max_count: n1,
                },
                ViolationFamily::SlotRotation {
                    student: s2,
                    subject: sub2,
                    slot: sl2,
                    range: r2,
                    max_count: n2,
                },
            ) => {
                s1 == s2
                    && sub1 == sub2
                    && sl1 == sl2
                    && ranged_bound_implies(
                        r1,
                        &CountBoundKind::Upper(*n1),
                        r2,
                        &CountBoundKind::Upper(*n2),
                    )
            }

            (
                ViolationFamily::StudentsInGroup {
                    group_list: gl1,
                    group: g1,
                    subject_scope: sc1,
                    kind: k1,
                },
                ViolationFamily::StudentsInGroup {
                    group_list: gl2,
                    group: g2,
                    subject_scope: sc2,
                    kind: k2,
                },
            ) => gl1 == gl2 && g1 == g2 && students_in_group_implies(sc1, k1, sc2, k2),

            (
                ViolationFamily::GroupCount {
                    slot: sl1,
                    week: w1,
                    kind: k1,
                },
                ViolationFamily::GroupCount {
                    slot: sl2,
                    week: w2,
                    kind: k2,
                },
            ) => sl1 == sl2 && w1 == w2 && bound_implies(k1, k2),

            (
                ViolationFamily::InterrogationsPerTimePeriod {
                    student: s1,
                    week: w1,
                    day: d1,
                    kind: k1,
                },
                ViolationFamily::InterrogationsPerTimePeriod {
                    student: s2,
                    week: w2,
                    day: d2,
                    kind: k2,
                },
            ) => s1 == s2 && w1 == w2 && time_period_implies(d1, k1, d2, k2),

            _ => false,
        }
    }
}

fn bound_implies(a: &CountBoundKind, b: &CountBoundKind) -> bool {
    match (a, b) {
        (CountBoundKind::Upper(n1), CountBoundKind::Upper(n2)) => n1 >= n2,
        (CountBoundKind::Upper(n), CountBoundKind::Exact(m)) => n >= m,
        (CountBoundKind::Lower(m1), CountBoundKind::Lower(m2)) => m1 <= m2,
        (CountBoundKind::Lower(m), CountBoundKind::Exact(n)) => m <= n,
        _ => false,
    }
}

fn ranged_bound_implies(
    range_a: &WeekRange,
    kind_a: &CountBoundKind,
    range_b: &WeekRange,
    kind_b: &CountBoundKind,
) -> bool {
    match (kind_a, kind_b) {
        (CountBoundKind::Upper(n1), CountBoundKind::Upper(n2)) => {
            range_a.is_subset_of(range_b) && n1 >= n2
        }
        (CountBoundKind::Upper(n), CountBoundKind::Exact(m)) => {
            range_a.is_subset_of(range_b) && n >= m
        }

        (CountBoundKind::Lower(m1), CountBoundKind::Lower(m2)) => {
            range_b.is_subset_of(range_a) && m1 <= m2
        }
        (CountBoundKind::Lower(m), CountBoundKind::Exact(n)) => {
            range_b.is_subset_of(range_a) && m <= n
        }

        _ => false,
    }
}

fn students_in_group_implies(
    scope_a: &Option<(SubjectId, PeriodId)>,
    kind_a: &CountBoundKind,
    scope_b: &Option<(SubjectId, PeriodId)>,
    kind_b: &CountBoundKind,
) -> bool {
    match (scope_a, scope_b) {
        (None, None) => bound_implies(kind_a, kind_b),
        (Some((s1, p1)), Some((s2, p2))) if s1 == s2 && p1 == p2 => bound_implies(kind_a, kind_b),

        (Some(_), None) => match (kind_a, kind_b) {
            (CountBoundKind::Upper(n1), CountBoundKind::Upper(n2)) => n1 >= n2,
            (CountBoundKind::Upper(n), CountBoundKind::Exact(m)) => n >= m,
            _ => false,
        },

        (None, Some(_)) => match (kind_a, kind_b) {
            (CountBoundKind::Lower(m1), CountBoundKind::Lower(m2)) => m1 <= m2,
            (CountBoundKind::Lower(m), CountBoundKind::Exact(n)) => m <= n,
            _ => false,
        },

        _ => false,
    }
}

fn time_period_implies(
    day_a: &Option<collomatique_time::Weekday>,
    kind_a: &CountBoundKind,
    day_b: &Option<collomatique_time::Weekday>,
    kind_b: &CountBoundKind,
) -> bool {
    match (day_a, day_b) {
        (None, None) => bound_implies(kind_a, kind_b),
        (Some(d1), Some(d2)) if d1 == d2 => bound_implies(kind_a, kind_b),

        (Some(_), None) => match (kind_a, kind_b) {
            (CountBoundKind::Upper(n1), CountBoundKind::Upper(n2)) => n1 >= n2,
            (CountBoundKind::Upper(n), CountBoundKind::Exact(m)) => n >= m,
            _ => false,
        },

        (None, Some(_)) => match (kind_a, kind_b) {
            (CountBoundKind::Lower(m1), CountBoundKind::Lower(m2)) => m1 <= m2,
            (CountBoundKind::Lower(m), CountBoundKind::Exact(n)) => m <= n,
            _ => false,
        },

        _ => false,
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ViolationCategoryKey {
    PeriodicityCount {
        student: StudentId,
        subject: SubjectId,
    },
    TeacherRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
    },
    SlotRotation {
        student: StudentId,
        subject: SubjectId,
        slot: SlotId,
    },
    StudentsInGroup {
        group_list: GroupListId,
        group: GroupNum,
    },
    GroupCount {
        slot: SlotId,
        week: GlobalWeek,
    },
    InterrogationsPerTimePeriod {
        student: StudentId,
        week: GlobalWeek,
    },
}

impl collomatique_ilp_modeler::ViolationImplication for ConstraintDesc {
    type CategoryKey = ViolationCategoryKey;

    fn violation_category(&self) -> Option<Self::CategoryKey> {
        self.violation_family().map(|fam| match fam {
            ViolationFamily::PeriodicityCount {
                student, subject, ..
            } => ViolationCategoryKey::PeriodicityCount { student, subject },
            ViolationFamily::TeacherRotation {
                student,
                subject,
                teacher,
                ..
            } => ViolationCategoryKey::TeacherRotation {
                student,
                subject,
                teacher,
            },
            ViolationFamily::SlotRotation {
                student,
                subject,
                slot,
                ..
            } => ViolationCategoryKey::SlotRotation {
                student,
                subject,
                slot,
            },
            ViolationFamily::StudentsInGroup {
                group_list, group, ..
            } => ViolationCategoryKey::StudentsInGroup { group_list, group },
            ViolationFamily::GroupCount { slot, week, .. } => {
                ViolationCategoryKey::GroupCount { slot, week }
            }
            ViolationFamily::InterrogationsPerTimePeriod { student, week, .. } => {
                ViolationCategoryKey::InterrogationsPerTimePeriod { student, week }
            }
        })
    }

    fn violation_implies(&self, other: &Self) -> bool {
        self.violation_implies(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collomatique_state_colloscopes::ids::Id;

    fn student(n: u64) -> StudentId {
        unsafe { StudentId::new(n) }
    }
    fn subject(n: u64) -> SubjectId {
        unsafe { SubjectId::new(n) }
    }
    fn teacher(n: u64) -> TeacherId {
        unsafe { TeacherId::new(n) }
    }
    fn slot(n: u64) -> SlotId {
        unsafe { SlotId::new(n) }
    }
    fn group_list(n: u64) -> GroupListId {
        unsafe { GroupListId::new(n) }
    }
    fn period(n: u64) -> PeriodId {
        unsafe { PeriodId::new(n) }
    }
    fn week(n: usize) -> GlobalWeek {
        GlobalWeek(n)
    }
    fn group(n: usize) -> GroupNum {
        GroupNum(n)
    }

    #[test]
    fn reflexivity() {
        let c = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(c.violation_implies(&c));
    }

    #[test]
    fn no_family_reflexivity() {
        let c = ConstraintDesc::Level1(StructuralConstraint::StudentHasGroup {
            student: student(1),
            group_list: group_list(1),
        });
        assert!(c.violation_implies(&c));
        let c2 = ConstraintDesc::Level1(StructuralConstraint::StudentHasGroup {
            student: student(2),
            group_list: group_list(1),
        });
        assert!(!c.violation_implies(&c2));
    }

    // === Family A: Periodicity ===

    #[test]
    fn max_implies_exact_same_range_same_bound() {
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let exact =
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountExact {
                student: student(1),
                subject: subject(1),
                first_week: week(0),
                last_week: week(5),
                count: 3,
            });
        assert!(max.violation_implies(&exact));
        assert!(!exact.violation_implies(&max));
    }

    #[test]
    fn max_higher_bound_implies_max_lower_bound_same_range() {
        let max5 = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(10),
            max_count: 5,
        });
        let max3 = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(10),
            max_count: 3,
        });
        assert!(max5.violation_implies(&max3));
        assert!(!max3.violation_implies(&max5));
    }

    #[test]
    fn max_inner_range_implies_max_outer_range_same_bound() {
        let inner = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(2),
            last_week: week(5),
            max_count: 3,
        });
        let outer = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(10),
            max_count: 3,
        });
        assert!(inner.violation_implies(&outer));
        assert!(!outer.violation_implies(&inner));
    }

    #[test]
    fn separation_implies_max_1_when_nested() {
        let sep = ConstraintDesc::Level2(QualityConstraint::PeriodicitySeparation {
            student: student(1),
            subject: subject(1),
            first_week: week(2),
            last_week: week(3),
        });
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 1,
        });
        assert!(sep.violation_implies(&max));
        assert!(!max.violation_implies(&sep));
    }

    #[test]
    fn separation_does_not_imply_max_2() {
        let sep = ConstraintDesc::Level2(QualityConstraint::PeriodicitySeparation {
            student: student(1),
            subject: subject(1),
            first_week: week(2),
            last_week: week(3),
        });
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 2,
        });
        assert!(!sep.violation_implies(&max));
    }

    #[test]
    fn min_implies_exact_same_range() {
        let min = ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            min_count: 3,
        });
        let exact =
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountExact {
                student: student(1),
                subject: subject(1),
                first_week: week(0),
                last_week: week(5),
                count: 3,
            });
        assert!(min.violation_implies(&exact));
        assert!(!exact.violation_implies(&min));
    }

    #[test]
    fn min_outer_range_implies_min_inner_range() {
        let outer =
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
                student: student(1),
                subject: subject(1),
                first_week: week(0),
                last_week: week(10),
                min_count: 3,
            });
        let inner =
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
                student: student(1),
                subject: subject(1),
                first_week: week(2),
                last_week: week(5),
                min_count: 4,
            });
        assert!(outer.violation_implies(&inner));
        assert!(!inner.violation_implies(&outer));
    }

    #[test]
    fn max_does_not_imply_min() {
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let min = ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            min_count: 2,
        });
        assert!(!max.violation_implies(&min));
        assert!(!min.violation_implies(&max));
    }

    #[test]
    fn different_students_incomparable() {
        let a = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let b = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(2),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(!a.violation_implies(&b));
        assert!(!b.violation_implies(&a));
    }

    // === Family B: Teacher rotation ===

    #[test]
    fn rotation_implies_year_rotation() {
        let rot = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let year = ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            max_count: 2,
        });
        assert!(rot.violation_implies(&year));
        assert!(!year.violation_implies(&rot));
    }

    #[test]
    fn rotation_does_not_imply_year_rotation_when_bound_too_low() {
        let rot = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 2,
        });
        let year = ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            max_count: 4,
        });
        assert!(!rot.violation_implies(&year));
    }

    #[test]
    fn rotation_implies_period_rotation_when_nested() {
        let rot = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(2),
            last_week: week(5),
            max_count: 3,
        });
        let period_rot = ConstraintDesc::Level4(PreferenceConstraint::BalancingPeriodRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            period: 1,
            first_week: week(0),
            last_week: week(10),
            max_count: 2,
        });
        assert!(rot.violation_implies(&period_rot));
        assert!(!period_rot.violation_implies(&rot));
    }

    #[test]
    fn year_rotation_higher_bound_implies_lower() {
        let year5 = ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            max_count: 5,
        });
        let year3 = ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            max_count: 3,
        });
        assert!(year5.violation_implies(&year3));
        assert!(!year3.violation_implies(&year5));
    }

    #[test]
    fn different_teachers_incomparable() {
        let a = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let b = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(2),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(!a.violation_implies(&b));
    }

    // === Family C: Slot rotation ===

    #[test]
    fn slot_rotation_nested_ranges() {
        let inner = ConstraintDesc::Level4(PreferenceConstraint::BalancingSlotRotation {
            student: student(1),
            subject: subject(1),
            slot: slot(1),
            first_week: week(0),
            last_week: week(3),
            max_count: 2,
        });
        let outer = ConstraintDesc::Level4(PreferenceConstraint::BalancingSlotRotation {
            student: student(1),
            subject: subject(1),
            slot: slot(1),
            first_week: week(0),
            last_week: week(10),
            max_count: 2,
        });
        assert!(inner.violation_implies(&outer));
        assert!(!outer.violation_implies(&inner));
    }

    // === Family D: Students in group ===

    #[test]
    fn for_subject_max_implies_group_max() {
        let for_subj = ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupForSubjectMax {
            group_list: group_list(1),
            group: group(0),
            subject: subject(1),
            period: period(1),
            max_students: 5,
        });
        let total = ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupMax {
            group_list: group_list(1),
            group: group(0),
            max_students: 4,
        });
        assert!(for_subj.violation_implies(&total));
        assert!(!total.violation_implies(&for_subj));
    }

    #[test]
    fn group_min_implies_for_subject_min() {
        let total = ConstraintDesc::Level3(ProgressiveConstraint::StudentsPerGroupMin {
            group_list: group_list(1),
            group: group(0),
            min_students: 2,
        });
        let for_subj =
            ConstraintDesc::Level3(ProgressiveConstraint::StudentsPerGroupForSubjectMin {
                group_list: group_list(1),
                group: group(0),
                subject: subject(1),
                period: period(1),
                min_students: 3,
            });
        assert!(total.violation_implies(&for_subj));
        assert!(!for_subj.violation_implies(&total));
    }

    #[test]
    fn group_max_does_not_imply_for_subject_max() {
        let total = ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupMax {
            group_list: group_list(1),
            group: group(0),
            max_students: 10,
        });
        let for_subj = ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupForSubjectMax {
            group_list: group_list(1),
            group: group(0),
            subject: subject(1),
            period: period(1),
            max_students: 5,
        });
        assert!(!total.violation_implies(&for_subj));
    }

    // === Family E: Group count ===

    #[test]
    fn group_count_max_higher_implies_lower() {
        let max5 = ConstraintDesc::Level2(QualityConstraint::GroupCountPerInterrogationMax {
            slot: slot(1),
            week: week(0),
            max_groups: 5,
        });
        let max3 = ConstraintDesc::Level2(QualityConstraint::GroupCountPerInterrogationMax {
            slot: slot(1),
            week: week(0),
            max_groups: 3,
        });
        assert!(max5.violation_implies(&max3));
        assert!(!max3.violation_implies(&max5));
    }

    #[test]
    fn group_count_max_does_not_imply_min() {
        let max = ConstraintDesc::Level2(QualityConstraint::GroupCountPerInterrogationMax {
            slot: slot(1),
            week: week(0),
            max_groups: 5,
        });
        let min = ConstraintDesc::Level3(ProgressiveConstraint::GroupCountPerInterrogationMin {
            slot: slot(1),
            week: week(0),
            min_groups: 2,
        });
        assert!(!max.violation_implies(&min));
        assert!(!min.violation_implies(&max));
    }

    // === Family F: Interrogations per time period ===

    #[test]
    fn max_per_day_implies_max_per_week() {
        let day = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().next().unwrap(),
            max: 3,
        });
        let wk = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerWeek {
            student: student(1),
            week: week(0),
            max: 2,
        });
        assert!(day.violation_implies(&wk));
        assert!(!wk.violation_implies(&day));
    }

    #[test]
    fn max_per_day_does_not_imply_max_per_week_when_bound_too_low() {
        let day = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().next().unwrap(),
            max: 2,
        });
        let wk = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerWeek {
            student: student(1),
            week: week(0),
            max: 5,
        });
        assert!(!day.violation_implies(&wk));
    }

    #[test]
    fn max_per_week_does_not_imply_max_per_day() {
        let wk = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerWeek {
            student: student(1),
            week: week(0),
            max: 3,
        });
        let day = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().next().unwrap(),
            max: 2,
        });
        assert!(!wk.violation_implies(&day));
    }

    #[test]
    fn different_days_incomparable() {
        let mon = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().next().unwrap(),
            max: 3,
        });
        let tue = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().nth(1).unwrap(),
            max: 3,
        });
        assert!(!mon.violation_implies(&tue));
    }

    // === Cross-family ===

    #[test]
    fn periodicity_vs_rotation_incomparable() {
        let periodicity =
            ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
                student: student(1),
                subject: subject(1),
                first_week: week(0),
                last_week: week(5),
                max_count: 3,
            });
        let rotation = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(!periodicity.violation_implies(&rotation));
        assert!(!rotation.violation_implies(&periodicity));
    }

    #[test]
    fn no_family_vs_family_incomparable() {
        let structural = ConstraintDesc::Level1(StructuralConstraint::StudentHasGroup {
            student: student(1),
            group_list: group_list(1),
        });
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(!structural.violation_implies(&max));
        assert!(!max.violation_implies(&structural));
    }
}
