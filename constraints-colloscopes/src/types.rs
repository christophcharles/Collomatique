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

impl ConstraintDesc {
    pub fn user_readable(
        &self,
        env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> String {
        match self {
            ConstraintDesc::Script(Some(origin)) => origin.to_string(),
            ConstraintDesc::Script(None) => "Script (origine inconnue)".to_string(),
            ConstraintDesc::StudentsPerGroupMin {
                group_list,
                group,
                min_students,
            } => {
                let gl_name = group_list_name(env, *group_list);
                format!(
                    "Au moins {} élèves dans le groupe {} de la liste {}",
                    min_students,
                    group.0 + 1,
                    gl_name
                )
            }
            ConstraintDesc::StudentsPerGroupMax {
                group_list,
                group,
                max_students,
            } => {
                let gl_name = group_list_name(env, *group_list);
                format!(
                    "Au plus {} élèves dans le groupe {} de la liste {}",
                    max_students,
                    group.0 + 1,
                    gl_name
                )
            }
        }
    }
}

fn group_list_name(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    group_list: GroupListId,
) -> String {
    env.group_lists
        .group_list_map
        .get(&group_list)
        .map(|gl| gl.params.name.clone())
        .unwrap_or_else(|| format!("{:?}", group_list))
}

impl From<Option<Origin<SqliteDatabaseConnection>>> for ConstraintDesc {
    fn from(v: Option<Origin<SqliteDatabaseConnection>>) -> Self {
        ConstraintDesc::Script(v)
    }
}
