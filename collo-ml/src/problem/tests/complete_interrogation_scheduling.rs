use crate::eval::{NoObject, NoObjectEnv};

use super::*;

#[test]
fn complete_interrogations_scheduling() {
    // Colles scheduling problem:
    // - 11 students
    // - 3 subjects (each with 4 teachers, so 12 teachers total)
    // - 3 weeks
    // Constraints:
    // - Each student has exactly one subject per week
    // - Each student has each subject exactly once over the 3 weeks
    // - Each teacher interrogates at most 1 student per week

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Var {
        StudentWithTeacher {
            student: i32, // 0..11
            teacher: i32, // 0..12 (teachers 0-3: subject 0, 4-7: subject 1, 8-11: subject 2)
            week: i32,    // 0..3
        },
    }

    impl<T: EvalObject> EvalVar<T> for Var {
        fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
            HashMap::from([(
                "StudentWithTeacher".to_string(),
                vec![
                    crate::traits::SimpleFieldType::Int.into(),
                    crate::traits::SimpleFieldType::Int.into(),
                    crate::traits::SimpleFieldType::Int.into(),
                ],
            )])
        }

        fn fix(&self, _env: &T::Env) -> Option<f64> {
            match self {
                Var::StudentWithTeacher {
                    student,
                    teacher,
                    week,
                } => {
                    // Fix to 0 if any parameter is out of bounds
                    if *student < 0 || *student >= 11 {
                        return Some(0.0);
                    }
                    if *teacher < 0 || *teacher >= 12 {
                        return Some(0.0);
                    }
                    if *week < 0 || *week >= 3 {
                        return Some(0.0);
                    }
                    None
                }
            }
        }

        fn vars(
            _env: &T::Env,
        ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
        {
            let mut vars = BTreeMap::new();
            // Only create variables for valid combinations
            for student in 0..11 {
                for teacher in 0..12 {
                    for week in 0..3 {
                        vars.insert(
                            Var::StudentWithTeacher {
                                student,
                                teacher,
                                week,
                            },
                            collomatique_ilp::Variable::binary(),
                        );
                    }
                }
            }
            Ok(vars)
        }
    }

    impl<T: EvalObject> TryFrom<&ExternVar<T>> for Var {
        type Error = VarConversionError;
        fn try_from(value: &ExternVar<T>) -> Result<Self, Self::Error> {
            match value.name.as_str() {
                "StudentWithTeacher" => {
                    if value.params.len() != 3 {
                        return Err(VarConversionError::WrongParameterCount {
                            name: "StudentWithTeacher".into(),
                            expected: 3,
                            found: value.params.len(),
                        });
                    }
                    let student = match &value.params[0] {
                        crate::eval::ExprValue::Int(i) => *i,
                        _ => {
                            return Err(VarConversionError::WrongParameterType {
                                name: "StudentWithTeacher".into(),
                                param: 0,
                                expected: crate::traits::SimpleFieldType::Int.into(),
                            })
                        }
                    };
                    let teacher = match &value.params[1] {
                        crate::eval::ExprValue::Int(i) => *i,
                        _ => {
                            return Err(VarConversionError::WrongParameterType {
                                name: "StudentWithTeacher".into(),
                                param: 1,
                                expected: crate::traits::SimpleFieldType::Int.into(),
                            })
                        }
                    };
                    let week = match &value.params[2] {
                        crate::eval::ExprValue::Int(i) => *i,
                        _ => {
                            return Err(VarConversionError::WrongParameterType {
                                name: "StudentWithTeacher".into(),
                                param: 2,
                                expected: crate::traits::SimpleFieldType::Int.into(),
                            })
                        }
                    };
                    Ok(Var::StudentWithTeacher {
                        student,
                        teacher,
                        week,
                    })
                }
                _ => Err(VarConversionError::Unknown(value.name.clone())),
            }
        }
    }

    let env = NoObjectEnv {};
    let mut pb_builder =
        ProblemBuilder::<NoObject, Var>::new(&env).expect("NoObject and Var should be compatible");

    let warnings = pb_builder
        .add_constraints(
            Script {
                name: "colles_constraints".into(),
                content: r#"
                    ## Each student has exactly one teacher per week
                    pub let one_teacher_per_week() -> [Constraint] = [
                        sum t in [0..12] { $StudentWithTeacher(s, t, w) } === 1
                        for s in [0..11]
                        for w in [0..3]
                    ];
                    
                    ## Each student has each subject exactly once over the 3 weeks
                    ## Subject 0: teachers 0-3, Subject 1: teachers 4-7, Subject 2: teachers 8-11
                    pub let each_subject_once() -> [Constraint] = [
                        sum t in [0..4] { sum w in [0..3] { $StudentWithTeacher(s, t, w) } } === 1
                        for s in [0..11]
                    ] + [
                        sum t in [4..8] { sum w in [0..3] { $StudentWithTeacher(s, t, w) } } === 1
                        for s in [0..11]
                    ] + [
                        sum t in [8..12] { sum w in [0..3] { $StudentWithTeacher(s, t, w) } } === 1
                        for s in [0..11]
                    ];
                    
                    ## Each teacher interrogates at most 1 students per week
                    pub let max_students_per_teacher() -> [Constraint] = [
                        sum s in [0..11] { $StudentWithTeacher(s, t, w) } <== 1
                        for t in [0..12]
                        for w in [0..3]
                    ];
                "#
                .into(),
            },
            vec![
                ("one_teacher_per_week".to_string(), vec![]),
                ("each_subject_once".to_string(), vec![]),
                ("max_students_per_teacher".to_string(), vec![]),
            ],
        )
        .expect("Should compile");

    assert!(warnings.is_empty(), "Unexpected warnings: {:?}", warnings);

    let problem = pb_builder.build();

    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
    use collomatique_ilp::solvers::Solver;
    let sol_opt = solver.solve(problem.get_inner_problem());

    let sol = sol_opt.expect("There should be a solution");

    // Verify the solution satisfies our constraints

    // 1. Each student has exactly one teacher per week
    for student in 0..11 {
        for week in 0..3 {
            let mut count = 0;
            for teacher in 0..12 {
                if let Some(val) = sol.get(ProblemVar::Base(Var::StudentWithTeacher {
                    student,
                    teacher,
                    week,
                })) {
                    if val >= 0.99 {
                        count += 1;
                    }
                }
            }
            assert_eq!(
                count, 1,
                "Student {} should have exactly 1 teacher in week {}, got {}",
                student, week, count
            );
        }
    }

    // 2. Each student has each subject exactly once
    for student in 0..11 {
        // Subject 0 (teachers 0-3)
        let mut subject0_count = 0;
        for teacher in 0..4 {
            for week in 0..3 {
                if let Some(val) = sol.get(ProblemVar::Base(Var::StudentWithTeacher {
                    student,
                    teacher,
                    week,
                })) {
                    if val >= 0.99 {
                        subject0_count += 1;
                    }
                }
            }
        }
        assert_eq!(
            subject0_count, 1,
            "Student {} should have subject 0 exactly once, got {}",
            student, subject0_count
        );

        // Subject 1 (teachers 4-7)
        let mut subject1_count = 0;
        for teacher in 4..8 {
            for week in 0..3 {
                if let Some(val) = sol.get(ProblemVar::Base(Var::StudentWithTeacher {
                    student,
                    teacher,
                    week,
                })) {
                    if val >= 0.99 {
                        subject1_count += 1;
                    }
                }
            }
        }
        assert_eq!(
            subject1_count, 1,
            "Student {} should have subject 1 exactly once, got {}",
            student, subject1_count
        );

        // Subject 2 (teachers 8-11)
        let mut subject2_count = 0;
        for teacher in 8..12 {
            for week in 0..3 {
                if let Some(val) = sol.get(ProblemVar::Base(Var::StudentWithTeacher {
                    student,
                    teacher,
                    week,
                })) {
                    if val >= 0.99 {
                        subject2_count += 1;
                    }
                }
            }
        }
        assert_eq!(
            subject2_count, 1,
            "Student {} should have subject 2 exactly once, got {}",
            student, subject2_count
        );
    }

    // 3. Each teacher has at most 1 students per week
    for teacher in 0..12 {
        for week in 0..3 {
            let mut count = 0;
            for student in 0..11 {
                if let Some(val) = sol.get(ProblemVar::Base(Var::StudentWithTeacher {
                    student,
                    teacher,
                    week,
                })) {
                    if val >= 0.99 {
                        count += 1;
                    }
                }
            }
            assert!(
                count <= 1,
                "Teacher {} should have at most 1 students in week {}, got {}",
                teacher,
                week,
                count
            );
        }
    }

    println!("Colles scheduling solution found and verified!");
}
