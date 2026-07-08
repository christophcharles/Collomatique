pub mod user_readable;
pub mod violation_order;

use crate::ids::{
    GlobalWeek, GroupListId, GroupNum, IncompatId, PairingRuleId, PeriodId, SlotId,
    SlotPairingRuleId, StudentId, SubjectId, TeacherId,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExtraVarName {
    InterrogationHasGroups {
        slot: SlotId,
        week: GlobalWeek,
    },
    StudentInGroup {
        student: StudentId,
        group_list: GroupListId,
        group: GroupNum,
    },
    GroupHasStudents {
        group_list: GroupListId,
        group: GroupNum,
    },
    GroupHasStudentsForSubject {
        group_list: GroupListId,
        group: GroupNum,
        subject: SubjectId,
        period: PeriodId,
    },
    StudentAtInterrogationInGroup {
        student: StudentId,
        slot: SlotId,
        week: GlobalWeek,
        group_list: GroupListId,
        group: GroupNum,
    },
    StudentAtInterrogation {
        student: StudentId,
        slot: SlotId,
        week: GlobalWeek,
    },
    StudentNotAtIncompatSlot {
        student: StudentId,
        incompat: IncompatId,
        incompat_slot_index: usize,
        week: GlobalWeek,
    },
    StudentHasInterrogationIn {
        student: StudentId,
        subject: SubjectId,
        week: GlobalWeek,
    },
    PairingsPenalty {
        rule: PairingRuleId,
    },
    SlotPairingsPenalty {
        rule: SlotPairingRuleId,
    },
    LimitsMaxPerDayPenalty,
    LimitsMaxPerWeekPenalty,
    LimitsMinPerWeekPenalty,
    IsLastTeacherSeen {
        subject: SubjectId,
        student: StudentId,
        teacher: TeacherId,
        week: GlobalWeek,
    },
    BalancingRotationPenalty {
        subject: SubjectId,
        student: StudentId,
        teacher: TeacherId,
        week: GlobalWeek,
    },
    BalancingSlotRotationPenalty {
        subject: SubjectId,
        student: StudentId,
        slot: SlotId,
        week: GlobalWeek,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum InfeasibleConstraint {
    PeriodicityExactlyPeriodicInfeasible {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        periodicity: u32,
    },
    PeriodicityOncePerBlockInfeasible {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        weeks_per_block: u32,
    },
    BalancingAvoidTwiceUnsupported {
        subject: SubjectId,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum StructuralConstraint {
    OneInterrogationAtOnce {
        student: StudentId,
        slot_a: SlotId,
        slot_b: SlotId,
        week: GlobalWeek,
    },
    IncompatSaturated {
        student: StudentId,
        incompat: IncompatId,
        subject: SubjectId,
        week: GlobalWeek,
    },
    IncompatNonSaturated {
        student: StudentId,
        incompat: IncompatId,
        subject: SubjectId,
        week: GlobalWeek,
        minimum_free_slots: u32,
    },
    StudentHasGroup {
        student: StudentId,
        group_list: GroupListId,
    },
    ForbiddenGroup {
        group_list: GroupListId,
        group: GroupNum,
        slot: SlotId,
        week: GlobalWeek,
        subject: SubjectId,
    },
    SlotPairingUsedImpliesNotUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
    SlotPairingNotUsedImpliesNotUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum QualityConstraint {
    StudentsPerGroupMax {
        group_list: GroupListId,
        group: GroupNum,
        max_students: u32,
    },
    StudentsPerGroupForSubjectMax {
        group_list: GroupListId,
        group: GroupNum,
        subject: SubjectId,
        period: PeriodId,
        max_students: u32,
    },
    GroupCountPerInterrogationMax {
        slot: SlotId,
        week: GlobalWeek,
        max_groups: u32,
    },
    PeriodicityInterrogationCountMax {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        max_count: u32,
    },
    PeriodicitySeparation {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ProgressiveConstraint {
    StudentsPerGroupMin {
        group_list: GroupListId,
        group: GroupNum,
        min_students: u32,
    },
    StudentsPerGroupForSubjectMin {
        group_list: GroupListId,
        group: GroupNum,
        subject: SubjectId,
        period: PeriodId,
        min_students: u32,
    },
    GroupCountPerInterrogationMin {
        slot: SlotId,
        week: GlobalWeek,
        min_groups: u32,
    },
    PeriodicityInterrogationCountMin {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        min_count: u32,
    },
    PeriodicityInterrogationCountExact {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        count: u32,
    },
    SlotPairingUsedImpliesUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
    SlotPairingNotUsedImpliesUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PreferenceConstraint {
    GroupFilledByAscendingOrder {
        group_list: GroupListId,
        group: GroupNum,
    },
    PairingHavingImpliesHaving {
        student: StudentId,
        week: GlobalWeek,
        rule: PairingRuleId,
    },
    PairingHavingImpliesNotHaving {
        student: StudentId,
        week: GlobalWeek,
        rule: PairingRuleId,
    },
    PairingNotHavingImpliesHaving {
        student: StudentId,
        week: GlobalWeek,
        rule: PairingRuleId,
    },
    PairingNotHavingImpliesNotHaving {
        student: StudentId,
        week: GlobalWeek,
        rule: PairingRuleId,
    },
    MaxInterrogationsPerDay {
        student: StudentId,
        week: GlobalWeek,
        day: collomatique_time::Weekday,
        max: u32,
    },
    MaxInterrogationsPerWeek {
        student: StudentId,
        week: GlobalWeek,
        max: u32,
    },
    MinInterrogationsPerWeek {
        student: StudentId,
        week: GlobalWeek,
        min: u32,
    },
    BalancingAvoidTwiceInARow {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
    },
    BalancingAvoidTwiceInARowRecursive {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        week: GlobalWeek,
    },
    BalancingYearRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        max_count: u32,
    },
    BalancingRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        max_count: u32,
    },
    /// Soft L1-regularity term: the student's cumulative count of interrogations
    /// with `teacher` through `week` should track the ideal linear ramp. One per
    /// (student, subject, teacher, prefix-boundary week).
    BalancingRotationRegularity {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        week: GlobalWeek,
    },
    BalancingSlotRotation {
        student: StudentId,
        subject: SubjectId,
        slot: SlotId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        max_count: u32,
    },
    /// Soft L1-regularity term for slot rotation: the student's cumulative count
    /// of interrogations in `slot` through `week` should track the ideal linear
    /// ramp. One per (student, subject, slot, prefix-boundary week).
    BalancingSlotRotationRegularity {
        student: StudentId,
        subject: SubjectId,
        slot: SlotId,
        week: GlobalWeek,
    },
    BalancingPeriodRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        period: u32,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        max_count: u32,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConstraintDesc {
    Level0(InfeasibleConstraint),
    Level1(StructuralConstraint),
    Level2(QualityConstraint),
    Level3(ProgressiveConstraint),
    Level4(PreferenceConstraint),
}

impl From<InfeasibleConstraint> for ConstraintDesc {
    fn from(c: InfeasibleConstraint) -> Self {
        ConstraintDesc::Level0(c)
    }
}

impl From<StructuralConstraint> for ConstraintDesc {
    fn from(c: StructuralConstraint) -> Self {
        ConstraintDesc::Level1(c)
    }
}

impl From<QualityConstraint> for ConstraintDesc {
    fn from(c: QualityConstraint) -> Self {
        ConstraintDesc::Level2(c)
    }
}

impl From<ProgressiveConstraint> for ConstraintDesc {
    fn from(c: ProgressiveConstraint) -> Self {
        ConstraintDesc::Level3(c)
    }
}

impl From<PreferenceConstraint> for ConstraintDesc {
    fn from(c: PreferenceConstraint) -> Self {
        ConstraintDesc::Level4(c)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum SeverityLevel {
    Infeasibility = 0,
    Structural = 1,
    Quality = 2,
    Progressive = 3,
    Preference = 4,
}

pub const SEVERITY_LEVEL_COUNT: usize = 5;

impl SeverityLevel {
    pub fn label(&self) -> &'static str {
        match self {
            SeverityLevel::Infeasibility => "Infeasibility",
            SeverityLevel::Structural => "Structural",
            SeverityLevel::Quality => "Quality",
            SeverityLevel::Progressive => "Progressive",
            SeverityLevel::Preference => "Preference",
        }
    }
}

impl ConstraintDesc {
    pub fn severity_level(&self) -> SeverityLevel {
        match self {
            ConstraintDesc::Level0(_) => SeverityLevel::Infeasibility,
            ConstraintDesc::Level1(_) => SeverityLevel::Structural,
            ConstraintDesc::Level2(_) => SeverityLevel::Quality,
            ConstraintDesc::Level3(_) => SeverityLevel::Progressive,
            ConstraintDesc::Level4(_) => SeverityLevel::Preference,
        }
    }

    pub fn severity_label(&self) -> &'static str {
        self.severity_level().label()
    }
}
