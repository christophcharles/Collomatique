use crate::ids::{GlobalWeek, GroupListId, GroupNum, PeriodId, SlotId, StudentId, SubjectId};
use collo_ml::SqliteDatabaseConnection;
use collo_ml::eval::Origin;
use collo_ml::script_feeder::ReifiedVar;
use derivative::Derivative;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub enum ReifiedVarName {
    Script(ReifiedVar<SqliteDatabaseConnection>),
    GroupInInterrogation {
        slot: SlotId,
        week: GlobalWeek,
        group: GroupNum,
    },
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
}

impl From<ReifiedVar<SqliteDatabaseConnection>> for ReifiedVarName {
    fn from(v: ReifiedVar<SqliteDatabaseConnection>) -> Self {
        ReifiedVarName::Script(v)
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub enum ConstraintDesc {
    Script(Option<Origin<SqliteDatabaseConnection>>),
    StudentsPerGroupMin {
        group_list: GroupListId,
        group: GroupNum,
        min_students: u32,
    },
    StudentsPerGroupMax {
        group_list: GroupListId,
        group: GroupNum,
        max_students: u32,
    },
}

impl std::fmt::Display for ConstraintDesc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstraintDesc::Script(Some(origin)) => write!(f, "{}", origin),
            ConstraintDesc::Script(None) => write!(f, "Script (origine inconnue)"),
            ConstraintDesc::StudentsPerGroupMin {
                group_list,
                group,
                min_students,
            } => write!(
                f,
                "Au moins {} élèves dans le groupe {} de la liste {:?}",
                min_students, group.0, group_list
            ),
            ConstraintDesc::StudentsPerGroupMax {
                group_list,
                group,
                max_students,
            } => write!(
                f,
                "Au plus {} élèves dans le groupe {} de la liste {:?}",
                max_students, group.0, group_list
            ),
        }
    }
}

impl From<Option<Origin<SqliteDatabaseConnection>>> for ConstraintDesc {
    fn from(v: Option<Origin<SqliteDatabaseConnection>>) -> Self {
        ConstraintDesc::Script(v)
    }
}
