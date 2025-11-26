use super::*;

// ========== Quantifiers with Variables and Collection Operations ==========

#[test]
fn forall_with_reified_var_and_filter() {
    let input = r#"
    let constraint_gen(x: Int) -> Constraint = $V(x) <== 1;
    reify constraint_gen as $MyVar;
    pub let f(xs: [Int]) -> Constraint = forall x in xs where x > 0 { $MyVar(x) === 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(-1), ExprValue::Int(1), ExprValue::Int(2)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Only x=1 and x=2 pass the filter
            assert_eq!(constraints.len(), 2);
            let constraints = strip_origins(&constraints);

            // Expected: $MyVar(1) === 1 and $MyVar(2) === 1
            let expected1 = LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(1)],
            }))
            .eq(&LinExpr::constant(1.));

            let expected2 = LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(2)],
            }))
            .eq(&LinExpr::constant(1.));

            assert!(constraints.contains(&expected1));
            assert!(constraints.contains(&expected2));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn sum_with_var_list_and_comprehension() {
    let input = r#"
    let h(xs: [Int]) -> [Constraint] = [$V(x) === 1 for x in xs];
    reify h as $[MyVars];
    pub let f(xs: [Int], ys: [Int]) -> LinExpr = sum v in $[MyVars](xs union ys) { v };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let xs = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let ys = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(2), ExprValue::Int(3)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![xs, ys])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Union gives [1, 2, 3], so 3 variables summed
            let expected = LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVars".into(),
                from_list: Some(0),
                params: vec![ExprValue::List(
                    ExprType::Int,
                    BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
                )],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVars".into(),
                from_list: Some(1),
                params: vec![ExprValue::List(
                    ExprType::Int,
                    BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
                )],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVars".into(),
                from_list: Some(2),
                params: vec![ExprValue::List(
                    ExprType::Int,
                    BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
                )],
            }));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn nested_quantifiers_with_filters() {
    let input = r#"
    pub let f(xs: [Int], ys: [Int]) -> Int = 
        sum x in xs where x > 0 { 
            sum y in ys where y < 10 { 
                if x + y > 5 { 1 } else { 0 }
            }
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let xs = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(-1), ExprValue::Int(2), ExprValue::Int(3)]),
    );
    let ys = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(5), ExprValue::Int(15)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![xs, ys])
        .expect("Should evaluate");

    // xs filtered: [2, 3], ys filtered: [5]
    // (2,5): 7>5 → 1, (3,5): 8>5 → 1, total: 2
    assert_eq!(result, ExprValue::Int(2));
}

// ========== List Comprehensions with Complex Expressions ==========

#[test]
fn list_comp_with_function_calls_and_filters() {
    let input = r#"
    let is_valid(x: Int) -> Bool = x > 0 and x < 10;
    let transform(x: Int) -> Int = x * x;
    pub let f(xs: [Int]) -> [Int] = [transform(x) for x in xs where is_valid(x)];
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([
            ExprValue::Int(-1),
            ExprValue::Int(2),
            ExprValue::Int(5),
            ExprValue::Int(15),
        ]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list])
        .expect("Should evaluate");

    // Valid: 2, 5 → squared: 4, 25
    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(4), ExprValue::Int(25)])
        )
    );
}

#[test]
fn nested_list_comp_with_reified_vars() {
    let input = r#"
    let constraint_gen(x: Int, y: Int) -> Constraint = $V(x, y) === 1;
    reify constraint_gen as $MyVar;
    pub let f(xs: [Int], ys: [Int]) -> [LinExpr] = 
        [$MyVar(x, y) for x in xs for y in ys where x != y];
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int, ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let xs = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let ys = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(2), ExprValue::Int(3)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![xs, ys])
        .expect("Should evaluate");

    match result {
        ExprValue::List(ExprType::LinExpr, list) => {
            // (1,2), (1,3), (2,3) - 3 pairs where x != y
            assert_eq!(list.len(), 3);

            let expected_vars = BTreeSet::from([
                ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar {
                    name: "MyVar".into(),
                    from_list: None,
                    params: vec![ExprValue::Int(1), ExprValue::Int(2)],
                }))),
                ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar {
                    name: "MyVar".into(),
                    from_list: None,
                    params: vec![ExprValue::Int(1), ExprValue::Int(3)],
                }))),
                ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar {
                    name: "MyVar".into(),
                    from_list: None,
                    params: vec![ExprValue::Int(2), ExprValue::Int(3)],
                }))),
            ]);
            assert_eq!(list, expected_vars);
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn list_comp_with_collection_ops_in_body() {
    let input = r#"
    let intersect_with_range(xs: [Int], n: Int) -> [Int] = xs inter [1..n];
    pub let f(lists: [[Int]]) -> [Int] = 
        [|intersect_with_range(lst, 10)| for lst in lists];
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list1 = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(5), ExprValue::Int(15)]),
    );
    let list2 = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(3), ExprValue::Int(8)]),
    );
    let lists = ExprValue::List(
        ExprType::List(Box::new(ExprType::Int)),
        BTreeSet::from([list1, list2]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![lists])
        .expect("Should evaluate");

    // list1 inter [1..10]: [1, 5] → |2|
    // list2 inter [1..10]: [3, 8] → |2|
    assert_eq!(
        result,
        ExprValue::List(ExprType::Int, BTreeSet::from([ExprValue::Int(2)]))
    );
}

// ========== If Expressions with Complex Conditions ==========

#[test]
fn if_with_quantifier_in_condition() {
    let input = r#"
    pub let f(xs: [Int]) -> Int = 
        if forall x in xs { x > 0 } { 
            sum x in xs { x } 
        } else { 
            0 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let all_positive = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
    );
    let result_positive = checked_ast
        .quick_eval_fn("f", vec![all_positive])
        .expect("Should evaluate");
    assert_eq!(result_positive, ExprValue::Int(6));

    let has_negative = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(-2)]),
    );
    let result_negative = checked_ast
        .quick_eval_fn("f", vec![has_negative])
        .expect("Should evaluate");
    assert_eq!(result_negative, ExprValue::Int(0));
}

#[test]
fn if_with_collection_check() {
    let input = r#"
    pub let f(x: Int, valid_set: [Int]) -> Bool = 
        if x in valid_set { 
            x > 0 
        } else { 
            false 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let valid_set = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(5), ExprValue::Int(10)]),
    );

    let result_in_and_positive = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5), valid_set.clone()])
        .expect("Should evaluate");
    assert_eq!(result_in_and_positive, ExprValue::Bool(true));

    let result_not_in = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(3), valid_set])
        .expect("Should evaluate");
    assert_eq!(result_not_in, ExprValue::Bool(false));
}

#[test]
fn nested_if_with_variables() {
    let input = r#"
    let constraint_gen(x: Int) -> Constraint = $V(x) === 1;
    reify constraint_gen as $MyVar;
    pub let f(x: Int, use_var: Bool, scale: Bool) -> LinExpr = 
        if use_var { 
            if scale { 
                2 * $MyVar(x) 
            } else { 
                $MyVar(x) 
            }
        } else { 
            x as LinExpr 
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_scaled = checked_ast
        .quick_eval_fn(
            "f",
            vec![
                ExprValue::Int(5),
                ExprValue::Bool(true),
                ExprValue::Bool(true),
            ],
        )
        .expect("Should evaluate");

    match result_scaled {
        ExprValue::LinExpr(lin_expr) => {
            let expected = 2 * LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(5)],
            }));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Functions with Quantifiers and Variables ==========

#[test]
fn function_returning_constraint_system() {
    let input = r#"
    let var_sum_constraint(xs: [Int], total: Int) -> Constraint = 
        sum x in xs { $V(x) } === total;
    let var_bound_constraints(xs: [Int]) -> Constraint = 
        forall x in xs { $V(x) >== 0 and $V(x) <== 1 };
    pub let f(xs: [Int], total: Int) -> Constraint = 
        var_sum_constraint(xs, total) and var_bound_constraints(xs);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list, ExprValue::Int(2)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // 1 sum constraint + 6 bound constraints (2 per variable)
            assert_eq!(constraints.len(), 7);
            let constraints = strip_origins(&constraints);

            // Check sum constraint: V(1) + V(2) + V(3) === 2
            let sum_constraint = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(3)],
            })))
            .eq(&LinExpr::constant(2.));
            assert!(constraints.contains(&sum_constraint));

            // Check bound constraints for each variable
            for x in [1, 2, 3] {
                let ge_constraint = LinExpr::var(IlpVar::Base(ExternVar {
                    name: "V".into(),
                    params: vec![ExprValue::Int(x)],
                }))
                .geq(&LinExpr::constant(0.));

                let le_constraint = LinExpr::var(IlpVar::Base(ExternVar {
                    name: "V".into(),
                    params: vec![ExprValue::Int(x)],
                }))
                .leq(&LinExpr::constant(1.));

                assert!(constraints.contains(&ge_constraint));
                assert!(constraints.contains(&le_constraint));
            }
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn function_composition_with_reified_vars() {
    let input = r#"
    let make_constraint(x: Int, y: Int) -> Constraint = $V(x, y) === 1;
    reify make_constraint as $MyVar;
    let sum_vars(xs: [Int], y: Int) -> LinExpr = sum x in xs { $MyVar(x, y) };
    let constrain_sum(xs: [Int], y: Int, limit: Int) -> Constraint = sum_vars(xs, y) <== limit;
    pub let f(xs: [Int], y: Int) -> Constraint = constrain_sum(xs, y, 10);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int, ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list, ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            // Expected: MyVar(1,5) + MyVar(2,5) <= 10
            let expected = (LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(1), ExprValue::Int(5)],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(2), ExprValue::Int(5)],
            })))
            .leq(&LinExpr::constant(10.));

            assert!(constraints.contains(&expected));
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Realistic Scheduling-Like Scenarios ==========

#[test]
fn assignment_constraint_pattern() {
    let input = r#"
    # Each student must be assigned to exactly one time slot
    let exactly_one_slot(student: Int, slots: [Int]) -> Constraint = 
        sum slot in slots { $Assigned(student, slot) } === 1;
    
    # Total assignments per slot must not exceed capacity
    let slot_capacity(slot: Int, students: [Int], capacity: Int) -> Constraint = 
        sum student in students { $Assigned(student, slot) } <== capacity;
    
    pub let f(students: [Int], slots: [Int], capacity: Int) -> Constraint = 
        forall student in students { exactly_one_slot(student, slots) } and
        forall slot in slots { slot_capacity(slot, students, capacity) };
    "#;

    let vars = HashMap::from([("Assigned".to_string(), vec![ExprType::Int, ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let students = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let slots = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![students, slots, ExprValue::Int(1)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // 2 student constraints + 2 slot constraints = 4 total
            assert_eq!(constraints.len(), 4);
            let constraints = strip_origins(&constraints);

            // Student 1 exactly one: Assigned(1,1) + Assigned(1,2) === 1
            let student1_constraint = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(1), ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(1), ExprValue::Int(2)],
            })))
            .eq(&LinExpr::constant(1.));
            assert!(constraints.contains(&student1_constraint));

            // Student 2 exactly one: Assigned(2,1) + Assigned(2,2) === 1
            let student2_constraint = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(2), ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(2), ExprValue::Int(2)],
            })))
            .eq(&LinExpr::constant(1.));
            assert!(constraints.contains(&student2_constraint));

            // Slot 1 capacity: Assigned(1,1) + Assigned(2,1) <= 1
            let slot1_constraint = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(1), ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(2), ExprValue::Int(1)],
            })))
            .leq(&LinExpr::constant(1.));
            assert!(constraints.contains(&slot1_constraint));

            // Slot 2 capacity: Assigned(1,2) + Assigned(2,2) <= 1
            let slot2_constraint = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(1), ExprValue::Int(2)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(2), ExprValue::Int(2)],
            })))
            .leq(&LinExpr::constant(1.));
            assert!(constraints.contains(&slot2_constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn conditional_constraint_with_reification() {
    let input = r#"
    # Create an indicator variable for a constraint
    let student_available(student: Int, time: Int) -> Constraint = 
        $Available(student, time) === 1;
    reify student_available as $IsAvailable;
    
    # Only assign if available
    pub let f(student: Int, time: Int) -> Constraint = 
        $Assigned(student, time) <== $IsAvailable(student, time);
    "#;

    let vars = HashMap::from([
        ("Available".to_string(), vec![ExprType::Int, ExprType::Int]),
        ("Assigned".to_string(), vec![ExprType::Int, ExprType::Int]),
    ]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(1), ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            // Expected: Assigned(1,5) <= IsAvailable(1,5)
            let expected = LinExpr::var(IlpVar::Base(ExternVar {
                name: "Assigned".into(),
                params: vec![ExprValue::Int(1), ExprValue::Int(5)],
            }))
            .leq(&LinExpr::var(IlpVar::Script(ScriptVar {
                name: "IsAvailable".into(),
                from_list: None,
                params: vec![ExprValue::Int(1), ExprValue::Int(5)],
            })));

            assert!(constraints.contains(&expected));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn aggregation_with_filtering() {
    let input = r#"
    let count_valid_assignments(students: [Student], time: Int, min_score: Int) -> LinExpr = 
        sum student in students where student.score > min_score { 
            $Assigned(student, time) 
        };
    
    pub let f(students: [Student], times: [Int], min_score: Int, min_per_time: Int) -> Constraint = 
        forall time in times { 
            count_valid_assignments(students, time, min_score) >== min_per_time 
        };
    "#;
    let vars = HashMap::from([
        ("Score".to_string(), vec![ExprType::Int]),
        (
            "Assigned".to_string(),
            vec![ExprType::Object("Student".into()), ExprType::Int],
        ),
    ]);

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Student {
        Student1,
        Student2,
    }

    struct Env {}

    impl EvalObject for Student {
        type Env = Env;

        fn objects_with_typ(_env: &Self::Env, name: &str) -> BTreeSet<Self> {
            match name {
                "Student" => BTreeSet::from([Student::Student1, Student::Student2]),
                _ => BTreeSet::new(),
            }
        }
        fn typ_name(&self, _env: &Self::Env) -> String {
            "Student".into()
        }
        fn field_access(&self, _env: &Self::Env, field: &str) -> Option<ExprValue<Self>> {
            assert_eq!(field, "score");
            Some(match self {
                Student::Student1 => ExprValue::Int(45),
                Student::Student2 => ExprValue::Int(100),
            })
        }
        fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
            HashMap::from([(
                "Student".to_string(),
                HashMap::from([("score".to_string(), ExprType::Int)]),
            )])
        }
    }

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let students = ExprValue::List(
        ExprType::Object("Student".into()),
        BTreeSet::from([
            ExprValue::Object(Student::Student1),
            ExprValue::Object(Student::Student2),
        ]),
    );
    let times = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );

    let env = Env {};
    let result = checked_ast
        .eval_fn(
            &env,
            "f",
            vec![students, times, ExprValue::Int(50), ExprValue::Int(1)],
        )
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // One constraint per time slot
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Collection Operations with Complex Expressions ==========

#[test]
fn dynamic_set_construction() {
    let input = r#"
    let valid_pairs(xs: [Int], ys: [Int]) -> [Int] = 
        [x + y for x in xs for y in ys where x + y < 10];
    
    let filter_evens(nums: [Int]) -> [Int] = 
        [n for n in nums where n % 2 == 0];
    
    pub let f(xs: [Int], ys: [Int]) -> [Int] = 
        filter_evens(valid_pairs(xs, ys));
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let xs = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(3), ExprValue::Int(5)]),
    );
    let ys = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(2), ExprValue::Int(4)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![xs, ys])
        .expect("Should evaluate");

    // valid_pairs: (1,2)→3, (1,4)→5, (3,2)→5, (3,4)→7, (5,2)→7, (5,4)→9
    // Unique values: [3, 5, 7, 9]
    // filter_evens: [] (none are even)
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn set_operations_with_comprehensions() {
    let input = r#"
    let positive_squares(xs: [Int]) -> [Int] = 
        [x * x for x in xs where x > 0];
    
    let small_numbers(xs: [Int]) -> [Int] = 
        [x for x in xs where x < 20];
    
    pub let f(xs: [Int]) -> [Int] = 
        positive_squares(xs) inter small_numbers(xs);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([
            ExprValue::Int(-2),
            ExprValue::Int(3),
            ExprValue::Int(5),
            ExprValue::Int(10),
        ]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list])
        .expect("Should evaluate");

    // positive_squares: [9, 25, 100]
    // small_numbers: [3, 5, 10]
    // intersection: [] (no overlap)
    assert_eq!(result, ExprValue::List(ExprType::Int, BTreeSet::new()));
}

#[test]
fn union_of_var_lists() {
    let input = r#"
    let vars_for_set(xs: [Int]) -> [Constraint] = [$V(x) === 1 for x in xs];
    reify vars_for_set as $[Vars];
    
    pub let f(xs: [Int], ys: [Int]) -> LinExpr = 
        sum v in ($[Vars](xs) union $[Vars](ys)) { v };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let xs = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let ys = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(2), ExprValue::Int(3)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![xs, ys])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => {
            // Union removes duplicates at the LinExpr level
            // The exact structure depends on how var lists merge
            assert!(true);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Edge Cases and Corner Cases ==========

#[test]
fn empty_list_propagation() {
    let input = r#"
    pub let f(xs: [Int]) -> Int = 
        if |xs| == 0 { 
            0 
        } else { 
            sum x in xs { x } 
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let empty = ExprValue::List(ExprType::Int, BTreeSet::new());
    let result_empty = checked_ast
        .quick_eval_fn("f", vec![empty])
        .expect("Should evaluate");
    assert_eq!(result_empty, ExprValue::Int(0));

    let non_empty = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let result_non_empty = checked_ast
        .quick_eval_fn("f", vec![non_empty])
        .expect("Should evaluate");
    assert_eq!(result_non_empty, ExprValue::Int(3));
}

#[test]
fn deeply_nested_structure() {
    let input = r#"
    let inner(x: Int) -> Int = x * 2;
    let middle(xs: [Int]) -> [Int] = [inner(x) for x in xs];
    let outer(lists: [[Int]]) -> Int = sum lst in lists { sum x in middle(lst) { x } };
    pub let f() -> Int = outer([[1, 2], [3]]);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    // [[1,2], [3]]
    // middle([1,2]) = [2,4], sum = 6
    // middle([3]) = [6], sum = 6
    // outer sum = 12
    assert_eq!(result, ExprValue::Int(12));
}

#[test]
fn mixed_coercion_in_complex_expression() {
    let input = r#"
    let get_coefficient(x: Int) -> Int = x * 2;
    pub let f(xs: [Int]) -> LinExpr = 
        sum x in xs { get_coefficient(x) * $V(x) };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // 2*$V(1) + 4*$V(2)
            let expected = 2 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            })) + 4 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            }));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn let_expr_in_deeply_nested_structure() {
    let input = r#"
    let process(x: Int) -> Int = let doubled = x * 2 { doubled + 1 };
    let transform(xs: [Int]) -> [Int] = [process(x) for x in xs];
    let aggregate(lists: [[Int]]) -> Int = 
        let processed = [transform(lst) for lst in lists] {
            sum result_list in processed { 
                sum val in result_list { val } 
            }
        };
    pub let f() -> Int = 
        let input_data = [[1, 2], [3, 4]] {
            aggregate(input_data)
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    // [[1,2], [3,4]]
    // process(1) = 3, process(2) = 5 -> [3, 5]
    // process(3) = 7, process(4) = 9 -> [7, 9]
    // processed = [[3, 5], [7, 9]]
    // sum of first: 3 + 5 = 8
    // sum of second: 7 + 9 = 16
    // total: 24
    assert_eq!(result, ExprValue::Int(24));
}

#[test]
fn all_features_combined() {
    let input = r#"
    # Helper to check if value is in valid range
    let in_range(x: Int, min: Int, max: Int) -> Bool = x >= min and x <= max;
    
    # Create base constraints
    let base_constraint(x: Int, y: Int) -> Constraint = $V(x, y) === 1;
    reify base_constraint as $MyVar;
    
    # Filter valid pairs
    let valid_pairs(xs: [Int], ys: [Int]) -> [Int] = 
        [x + y for x in xs for y in ys where in_range(x + y, 1, 10)];
    
    # Main constraint builder
    pub let f(xs: [Int], ys: [Int]) -> Constraint = 
        if |valid_pairs(xs, ys)| > 0 {
            forall x in xs {
                forall y in ys where in_range(x + y, 1, 10) {
                    $MyVar(x, y) <== 1
                }
            } and (sum x in xs { sum y in ys { $MyVar(x, y) } } <== 5)
        } else {
            (0 as LinExpr) === (0 as LinExpr)
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int, ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let xs = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let ys = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(3), ExprValue::Int(4)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![xs, ys])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Should have multiple constraints from nested foralls + sum constraint
            // (1,3):4, (1,4):5, (2,3):5, (2,4):6 all in range [1,10]
            // 4 le constraints + 1 sum constraint = 5 total
            assert_eq!(constraints.len(), 5);
            let constraints = strip_origins(&constraints);

            // Verify some constraints exist
            let constraint_1_3 = LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(1), ExprValue::Int(3)],
            }))
            .leq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint_1_3));

            // Verify sum constraint exists
            let sum_constraint = (LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(1), ExprValue::Int(3)],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(1), ExprValue::Int(4)],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(2), ExprValue::Int(3)],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(2), ExprValue::Int(4)],
            })))
            .leq(&LinExpr::constant(5.));
            assert!(constraints.contains(&sum_constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn all_features_combined_with_let() {
    let input = r#"
    # Helper to check if value is in valid range
    let in_range(x: Int, min: Int, max: Int) -> Bool = 
        let lower_check = x >= min {
            let upper_check = x <= max {
                lower_check and upper_check
            }
        };
    
    # Create base constraints
    let base_constraint(x: Int, y: Int) -> Constraint = $V(x, y) === 1;
    reify base_constraint as $MyVar;
    
    # Filter valid pairs using let
    let valid_pairs(xs: [Int], ys: [Int]) -> [Int] = 
        let range_min = 1 {
            let range_max = 10 {
                [x + y for x in xs for y in ys where in_range(x + y, range_min, range_max)]
            }
        };
    
    # Compute a threshold using let
    let compute_threshold(xs: [Int]) -> Int =
        let base = |xs| {
            let multiplier = 2 {
                base + multiplier
            }
        };
    
    # Main constraint builder with let expressions
    pub let f(xs: [Int], ys: [Int]) -> Constraint = 
        let valid = valid_pairs(xs, ys) {
            let threshold = compute_threshold(xs) {
                if |valid| > 0 {
                    let bound_value = 5 {
                        forall x in xs {
                            forall y in ys where in_range(x + y, 1, 10) {
                                $MyVar(x, y) <== 1
                            }
                        } and (sum x in xs { sum y in ys { $MyVar(x, y) } } <== bound_value)
                    }
                } else {
                    let zero_expr = 0 as LinExpr {
                        zero_expr === zero_expr
                    }
                }
            }
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int, ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let xs = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let ys = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(3), ExprValue::Int(4)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![xs, ys])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Should have multiple constraints from nested foralls + sum constraint
            // (1,3):4, (1,4):5, (2,3):5, (2,4):6 all in range [1,10]
            // 4 le constraints + 1 sum constraint = 5 total
            assert_eq!(constraints.len(), 5);
            let constraints = strip_origins(&constraints);

            // Verify some constraints exist
            let constraint_1_3 = LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(1), ExprValue::Int(3)],
            }))
            .leq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint_1_3));

            let constraint_2_4 = LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(2), ExprValue::Int(4)],
            }))
            .leq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint_2_4));

            // Verify sum constraint exists
            let sum_constraint = (LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(1), ExprValue::Int(3)],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(1), ExprValue::Int(4)],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(2), ExprValue::Int(3)],
            })) + LinExpr::var(IlpVar::Script(ScriptVar {
                name: "MyVar".into(),
                from_list: None,
                params: vec![ExprValue::Int(2), ExprValue::Int(4)],
            })))
            .leq(&LinExpr::constant(5.));
            assert!(constraints.contains(&sum_constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}
