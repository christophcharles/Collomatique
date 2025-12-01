use crate::eval::ExprValue;
use crate::semantics::ExprType;
use crate::traits::FieldConversionError;

use super::*;

// Simple EvalObject with 9 students
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SimpleObject {
    Student1,
    Student2,
    Student3,
    Student4,
    Student5,
    Student6,
    Student7,
    Student8,
    Student9,
}

struct SimpleEnv {}

impl EvalObject for SimpleObject {
    type Env = SimpleEnv;
    type Cache = ();

    fn objects_with_typ(_env: &Self::Env, name: &str) -> BTreeSet<Self> {
        match name {
            "Student" => BTreeSet::from([
                SimpleObject::Student1,
                SimpleObject::Student2,
                SimpleObject::Student3,
                SimpleObject::Student4,
                SimpleObject::Student5,
                SimpleObject::Student6,
                SimpleObject::Student7,
                SimpleObject::Student8,
                SimpleObject::Student9,
            ]),
            _ => BTreeSet::new(),
        }
    }

    fn typ_name(&self, _env: &Self::Env) -> String {
        "Student".into()
    }

    fn type_id_to_name(_type_id: std::any::TypeId) -> Result<String, FieldConversionError> {
        // All our objects are Students
        Ok("Student".into())
    }

    fn field_access(
        &self,
        _env: &Self::Env,
        _cache: &mut Self::Cache,
        field: &str,
    ) -> Option<ExprValue<Self>> {
        // Students have an 'id' field for convenience
        match field {
            "id" => {
                let id = match self {
                    SimpleObject::Student1 => 1,
                    SimpleObject::Student2 => 2,
                    SimpleObject::Student3 => 3,
                    SimpleObject::Student4 => 4,
                    SimpleObject::Student5 => 5,
                    SimpleObject::Student6 => 6,
                    SimpleObject::Student7 => 7,
                    SimpleObject::Student8 => 8,
                    SimpleObject::Student9 => 9,
                };
                Some(ExprValue::Int(id))
            }
            _ => None,
        }
    }

    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        let student_type = HashMap::from([("id".to_string(), ExprType::Int)]);
        HashMap::from([("Student".into(), student_type)])
    }
}

// Variable system: StudentGroup takes a Student and returns a group number (1-3)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Var {
    StudentGroup(SimpleObject),
}

impl<T: EvalObject> EvalVar<T> for Var {
    fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
        HashMap::from([(
            "StudentGroup".to_string(),
            vec![crate::traits::FieldType::Object(std::any::TypeId::of::<
                SimpleObject,
            >())],
        )])
    }

    fn fix(&self) -> Option<f64> {
        None // All students are valid, no need to fix
    }

    fn vars(
        _env: &T::Env,
    ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
    {
        let mut vars = BTreeMap::new();
        // Create a variable for each student
        for student in [
            SimpleObject::Student1,
            SimpleObject::Student2,
            SimpleObject::Student3,
            SimpleObject::Student4,
            SimpleObject::Student5,
            SimpleObject::Student6,
            SimpleObject::Student7,
            SimpleObject::Student8,
            SimpleObject::Student9,
        ] {
            vars.insert(
                Var::StudentGroup(student),
                collomatique_ilp::Variable::integer().min(1.).max(3.),
            );
        }
        Ok(vars)
    }
}

impl TryFrom<&ExternVar<SimpleObject>> for Var {
    type Error = VarConversionError;
    fn try_from(value: &ExternVar<SimpleObject>) -> Result<Self, Self::Error> {
        match value.name.as_str() {
            "StudentGroup" => {
                if value.params.len() != 1 {
                    return Err(VarConversionError::WrongParameterCount {
                        name: "StudentGroup".into(),
                        expected: 1,
                        found: value.params.len(),
                    });
                }
                // Extract the student object from parameters
                let student = match &value.params[0] {
                    crate::eval::ExprValue::Object(obj) => SimpleObject::try_from(obj.clone())
                        .map_err(|_| VarConversionError::WrongParameterType {
                            name: "StudentGroup".into(),
                            param: 0,
                            expected: crate::traits::FieldType::Object(std::any::TypeId::of::<
                                SimpleObject,
                            >(
                            )),
                        })?,
                    _ => {
                        return Err(VarConversionError::WrongParameterType {
                            name: "StudentGroup".into(),
                            param: 0,
                            expected: crate::traits::FieldType::Object(std::any::TypeId::of::<
                                SimpleObject,
                            >(
                            )),
                        })
                    }
                };
                Ok(Var::StudentGroup(student))
            }
            _ => Err(VarConversionError::Unknown(value.name.clone())),
        }
    }
}

#[test]
fn global_reified_variables_with_parameters() {
    let env = SimpleEnv {};
    let mut pb_builder = ProblemBuilder::<SimpleObject, Var>::new(&env)
        .expect("SimpleObject and Var should be compatible");

    // Define a reified variable with a Student parameter
    // student_in_group(s, g) is true iff StudentGroup(s) === g
    let warnings = pb_builder
        .add_reified_variables(
            Script {
                name: "reified_with_param".into(),
                content: r#"
                    pub let student_in_group(student: Student, group: Int) -> Constraint = 
                        $StudentGroup(student) === group;
                "#
                .into(),
            },
            vec![("student_in_group".to_string(), "StudentInGroup".to_string())],
        )
        .expect("Should compile reified variables");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Use the reified variable: force student 1 and 2 to be in group 1,
    // and enforce that exactly 2 students are in group 1
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "use_reified_param".into(),
                content: r#"
                    pub let enforce_groups() -> Constraint = 
                        sum s in @[Student] { $StudentInGroup(s, 1) } === 2;
                "#
                .into(),
            },
            vec![("enforce_groups".to_string(), vec![])],
        )
        .expect("Should compile constraints");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // Count how many students are in group 1
    let mut count_in_group_1 = 0;
    for student in [
        SimpleObject::Student1,
        SimpleObject::Student2,
        SimpleObject::Student3,
        SimpleObject::Student4,
        SimpleObject::Student5,
        SimpleObject::Student6,
        SimpleObject::Student7,
        SimpleObject::Student8,
        SimpleObject::Student9,
    ] {
        if let Some(group) = sol.get(ProblemVar::Base(Var::StudentGroup(student))) {
            if (group - 1.0).abs() < 0.01 {
                count_in_group_1 += 1;
            }
        }
    }

    assert_eq!(
        count_in_group_1, 2,
        "Exactly 2 students should be in group 1"
    );
}

#[test]
fn reified_variable_with_multiple_parameter_values() {
    let env = SimpleEnv {};
    let mut pb_builder = ProblemBuilder::<SimpleObject, Var>::new(&env)
        .expect("SimpleObject and Var should be compatible");

    // Define a reified variable with a Student parameter
    // student_in_group(s, g) is true iff StudentGroup(s) === g
    let warnings = pb_builder
        .add_reified_variables(
            Script {
                name: "reified_with_param".into(),
                content: r#"
                    pub let student_in_group(student: Student, group: Int) -> Constraint = 
                        $StudentGroup(student) === group;
                "#
                .into(),
            },
            vec![("student_in_group".to_string(), "StudentInGroup".to_string())],
        )
        .expect("Should compile reified variables");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    // Use the reified variable: force student 1 and 2 to be in group 1,
    // and enforce that exactly 2 students are in group 1
    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "use_reified_param".into(),
                content: r#"
                    pub let enforce_groups(group: Int) -> Constraint = 
                        sum s in @[Student] { $StudentInGroup(s, group) } === 3;
                "#
                .into(),
            },
            vec![
                ("enforce_groups".to_string(), vec![ExprValue::Int(1)]),
                ("enforce_groups".to_string(), vec![ExprValue::Int(2)]),
                ("enforce_groups".to_string(), vec![ExprValue::Int(3)]),
            ],
        )
        .expect("Should compile constraints");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // Count how many students are in group 1
    let mut count_in_group_1 = 0;
    let mut count_in_group_2 = 0;
    let mut count_in_group_3 = 0;
    for student in [
        SimpleObject::Student1,
        SimpleObject::Student2,
        SimpleObject::Student3,
        SimpleObject::Student4,
        SimpleObject::Student5,
        SimpleObject::Student6,
        SimpleObject::Student7,
        SimpleObject::Student8,
        SimpleObject::Student9,
    ] {
        if let Some(group) = sol.get(ProblemVar::Base(Var::StudentGroup(student))) {
            if (group - 1.0).abs() < 0.01 {
                count_in_group_1 += 1;
            }
            if (group - 2.0).abs() < 0.01 {
                count_in_group_2 += 1;
            }
            if (group - 3.0).abs() < 0.01 {
                count_in_group_3 += 1;
            }
        }
    }

    assert_eq!(
        count_in_group_1, 3,
        "Exactly 3 students should be in group 1"
    );
    assert_eq!(
        count_in_group_2, 3,
        "Exactly 3 students should be in group 2"
    );
    assert_eq!(
        count_in_group_3, 3,
        "Exactly 3 students should be in group 3"
    );
}
