use super::*;

#[test]
fn eval_with_variables_simple_reified_var() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 1;
    reify base as $MyVar;
    pub let f(n: Int) -> Constraint = $MyVar(n) <== 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(&env, "f", vec![ExprValue::<NoObject>::Int(5)])
        .expect("Should evaluate");

    // Check result is a constraint
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar with args [5] was defined
    assert!(var_defs
        .vars
        .contains_key(&("MyVar".to_string(), vec![ExprValue::Int(5)])));

    let my_var_constraints = &var_defs.vars[&("MyVar".to_string(), vec![ExprValue::Int(5)])].0;

    // MyVar(5) should have the constraint from base(5): $V(5) === 1
    assert_eq!(my_var_constraints.len(), 1);

    let expected = LinExpr::var(IlpVar::Base(ExternVar {
        name: "V".into(),
        params: vec![ExprValue::Int(5)],
    }))
    .eq(&LinExpr::constant(1.));

    assert!(my_var_constraints.contains(&expected));
}

#[test]
fn eval_with_variables_multiple_calls_same_var() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 1;
    reify base as $MyVar;
    pub let f() -> Constraint = $MyVar(3) <== 1 and $MyVar(7) <== 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::Int])]);

    let checked_ast = CheckedAST::<NoObject>::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(&env, "f", vec![])
        .expect("Should evaluate");

    // Check result
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar was called with both [3] and [7]
    assert!(var_defs
        .vars
        .contains_key(&("MyVar".to_string(), vec![ExprValue::Int(3)])));
    assert!(var_defs
        .vars
        .contains_key(&("MyVar".to_string(), vec![ExprValue::Int(7)])));

    // Verify constraints for MyVar(3)
    let my_var_3_constraints = &var_defs.vars[&("MyVar".to_string(), vec![ExprValue::Int(3)])].0;
    assert_eq!(my_var_3_constraints.len(), 1);
    let expected_3 = LinExpr::var(IlpVar::Base(ExternVar {
        name: "V".into(),
        params: vec![ExprValue::Int(3)],
    }))
    .eq(&LinExpr::constant(1.));
    assert!(my_var_3_constraints.contains(&expected_3));

    // Verify constraints for MyVar(7)
    let my_var_7_constraints = &var_defs.vars[&("MyVar".to_string(), vec![ExprValue::Int(7)])].0;
    assert_eq!(my_var_7_constraints.len(), 1);
    let expected_7 = LinExpr::var(IlpVar::Base(ExternVar {
        name: "V".into(),
        params: vec![ExprValue::Int(7)],
    }))
    .eq(&LinExpr::constant(1.));
    assert!(my_var_7_constraints.contains(&expected_7));
}

#[test]
fn eval_with_variables_in_forall() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 1;
    reify base as $MyVar;
    pub let f(n: Int) -> Constraint = forall i in [0..n] { $MyVar(i) <== 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(&env, "f", vec![ExprValue::<NoObject>::Int(3)])
        .expect("Should evaluate");

    // Check result has 3 constraints (for i=0,1,2)
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 3);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar was called for i=0,1,2
    assert_eq!(var_defs.vars.len(), 3);
    assert!(var_defs
        .vars
        .contains_key(&("MyVar".to_string(), vec![ExprValue::Int(0)])));
    assert!(var_defs
        .vars
        .contains_key(&("MyVar".to_string(), vec![ExprValue::Int(1)])));
    assert!(var_defs
        .vars
        .contains_key(&("MyVar".to_string(), vec![ExprValue::Int(2)])));

    // Verify each has the correct constraint
    for i in 0..3 {
        let my_var_constraints = &var_defs.vars[&("MyVar".to_string(), vec![ExprValue::Int(i)])].0;
        assert_eq!(my_var_constraints.len(), 1);
        let expected = LinExpr::var(IlpVar::Base(ExternVar {
            name: "V".into(),
            params: vec![ExprValue::Int(i)],
        }))
        .eq(&LinExpr::constant(1.));
        assert!(my_var_constraints.contains(&expected));
    }
}

#[test]
fn eval_with_variables_multiple_vars() {
    let input = r#"
    let base1(x: Int) -> Constraint = $V1(x) === 1;
    let base2(y: Int) -> Constraint = $V2(y) === 0;
    reify base1 as $Var1;
    reify base2 as $Var2;
    pub let f(a: Int, b: Int) -> Constraint = $Var1(a) <== 1 and $Var2(b) <== 1;
    "#;

    let vars = HashMap::from([
        ("V1".to_string(), vec![SimpleType::Int]),
        ("V2".to_string(), vec![SimpleType::Int]),
    ]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            &env,
            "f",
            vec![ExprValue::<NoObject>::Int(5), ExprValue::Int(10)],
        )
        .expect("Should evaluate");

    // Check result
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check both variables were defined
    assert_eq!(var_defs.vars.len(), 2);
    assert!(var_defs
        .vars
        .contains_key(&("Var1".to_string(), vec![ExprValue::Int(5)])));
    assert!(var_defs
        .vars
        .contains_key(&("Var2".to_string(), vec![ExprValue::Int(10)])));

    // Verify Var1 constraint
    let var1_constraints = &var_defs.vars[&("Var1".to_string(), vec![ExprValue::Int(5)])].0;
    let expected1 = LinExpr::var(IlpVar::Base(ExternVar {
        name: "V1".into(),
        params: vec![ExprValue::Int(5)],
    }))
    .eq(&LinExpr::constant(1.));
    assert!(var1_constraints.contains(&expected1));

    // Verify Var2 constraint
    let var2_constraints = &var_defs.vars[&("Var2".to_string(), vec![ExprValue::Int(10)])].0;
    let expected2 = LinExpr::var(IlpVar::Base(ExternVar {
        name: "V2".into(),
        params: vec![ExprValue::Int(10)],
    }))
    .eq(&LinExpr::constant(0.));
    assert!(var2_constraints.contains(&expected2));
}

#[test]
fn eval_with_variables_var_with_multiple_params() {
    let input = r#"
    let base(x: Int, y: Int) -> Constraint = $V(x, y) === 1;
    reify base as $MyVar;
    pub let f(a: Int, b: Int) -> Constraint = $MyVar(a, b) <== 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::Int, SimpleType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            &env,
            "f",
            vec![ExprValue::<NoObject>::Int(3), ExprValue::Int(7)],
        )
        .expect("Should evaluate");

    // Check result
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar(3, 7) was defined
    assert!(var_defs.vars.contains_key(&(
        "MyVar".to_string(),
        vec![ExprValue::Int(3), ExprValue::Int(7)]
    )));

    let my_var_constraints = &var_defs.vars[&(
        "MyVar".to_string(),
        vec![ExprValue::Int(3), ExprValue::Int(7)],
    )]
        .0;
    assert_eq!(my_var_constraints.len(), 1);
    let expected = LinExpr::var(IlpVar::Base(ExternVar {
        name: "V".into(),
        params: vec![ExprValue::Int(3), ExprValue::Int(7)],
    }))
    .eq(&LinExpr::constant(1.));
    assert!(my_var_constraints.contains(&expected));
}

#[test]
fn eval_with_variables_simple_var_list() {
    let input = r#"
    let base(x: Int, y: Int) -> [Constraint] = [$V(x, y) === 1, $V(x, y) <== 10];
    reify base as $[MyVarList];
    pub let f(a: Int, b: Int) -> Constraint = forall v in $[MyVarList](a, b) { v <== 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::Int, SimpleType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            &env,
            "f",
            vec![ExprValue::<NoObject>::Int(3), ExprValue::Int(7)],
        )
        .expect("Should evaluate");

    // Check result has 2 constraints (one for each element in the var list)
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVarList was called with (3, 7)
    assert_eq!(var_defs.var_lists.len(), 1);
    assert!(var_defs.var_lists.contains_key(&(
        "MyVarList".to_string(),
        vec![ExprValue::Int(3), ExprValue::Int(7)]
    )));

    let var_list_constraints = &var_defs.var_lists[&(
        "MyVarList".to_string(),
        vec![ExprValue::Int(3), ExprValue::Int(7)],
    )]
        .0;

    // Should have 2 constraint sets (one for each constraint in base's return list)
    assert_eq!(var_list_constraints.len(), 2);

    // Each constraint set should have 1 constraint
    for constraints in var_list_constraints {
        assert_eq!(constraints.len(), 1);
        let constraint = constraints.iter().next().unwrap();

        // Should be either $V(3, 7) === 1 or $V(3, 7) <== 10
        let c1 = LinExpr::var(IlpVar::Base(ExternVar {
            name: "V".into(),
            params: vec![ExprValue::Int(3), ExprValue::Int(7)],
        }))
        .eq(&LinExpr::constant(1.));

        let c2 = LinExpr::var(IlpVar::Base(ExternVar {
            name: "V".into(),
            params: vec![ExprValue::Int(3), ExprValue::Int(7)],
        }))
        .leq(&LinExpr::constant(10.));

        assert!(*constraint == c1 || *constraint == c2);
    }
}

#[test]
fn eval_with_variables_var_list_in_nested_forall() {
    let input = r#"
    let base(x: Int, y: Int) -> [Constraint] = [$V(x, y) === 1, $V(x, y) <== 10];
    reify base as $[MyVarList];
    pub let f(xs: [Int], ys: [Int]) -> Constraint = 
        forall x in xs {
            forall y in ys {
                forall v in $[MyVarList](x, y) {
                    v <== 1
                }
            }
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::Int, SimpleType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let xs = ExprValue::List(
        SimpleType::Int,
        Vec::from([ExprValue::<NoObject>::Int(1), ExprValue::Int(2)]),
    );
    let ys = ExprValue::List(
        SimpleType::Int,
        Vec::from([ExprValue::<NoObject>::Int(10), ExprValue::Int(20)]),
    );

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(&env, "f", vec![xs, ys])
        .expect("Should evaluate");

    // Check result: 2 xs * 2 ys * 2 constraints per var list = 8 constraints
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 8);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVarList was called with all combinations of (x, y)
    // Should have 4 calls: (1,10), (1,20), (2,10), (2,20)
    assert_eq!(var_defs.var_lists.len(), 4);

    assert!(var_defs.var_lists.contains_key(&(
        "MyVarList".to_string(),
        vec![ExprValue::Int(1), ExprValue::Int(10)]
    )));
    assert!(var_defs.var_lists.contains_key(&(
        "MyVarList".to_string(),
        vec![ExprValue::Int(1), ExprValue::Int(20)]
    )));
    assert!(var_defs.var_lists.contains_key(&(
        "MyVarList".to_string(),
        vec![ExprValue::Int(2), ExprValue::Int(10)]
    )));
    assert!(var_defs.var_lists.contains_key(&(
        "MyVarList".to_string(),
        vec![ExprValue::Int(2), ExprValue::Int(20)]
    )));

    // Verify one of them has the correct structure
    let var_list_1_10 = &var_defs.var_lists[&(
        "MyVarList".to_string(),
        vec![ExprValue::Int(1), ExprValue::Int(10)],
    )]
        .0;

    // Should have 2 constraint sets (one for each constraint in base's return list)
    assert_eq!(var_list_1_10.len(), 2);

    // Each constraint set should have 1 constraint
    for constraint_set in var_list_1_10 {
        assert_eq!(constraint_set.len(), 1);
    }
}

#[test]
fn eval_with_variables_with_let_expr() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 1;
    reify base as $MyVar;
    pub let f(n: Int) -> Constraint = 
        let bound = n * 2 {
            $MyVar(bound) <== 1
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(&env, "f", vec![ExprValue::<NoObject>::Int(5)])
        .expect("Should evaluate");

    // Check result
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar(10) was defined (bound = 5 * 2 = 10)
    assert!(var_defs
        .vars
        .contains_key(&("MyVar".to_string(), vec![ExprValue::Int(10)])));

    let my_var_constraints = &var_defs.vars[&("MyVar".to_string(), vec![ExprValue::Int(10)])].0;
    assert_eq!(my_var_constraints.len(), 1);
    let expected = LinExpr::var(IlpVar::Base(ExternVar {
        name: "V".into(),
        params: vec![ExprValue::Int(10)],
    }))
    .eq(&LinExpr::constant(1.));
    assert!(my_var_constraints.contains(&expected));
}

#[test]
fn eval_with_variables_no_reified_vars() {
    let input = r#"
    pub let f(x: Int) -> Constraint = $V(x) === 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![SimpleType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = NoObjectEnv {};

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(&env, "f", vec![ExprValue::<NoObject>::Int(5)])
        .expect("Should evaluate");

    // Check result
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }

    // No reified variables were used, so var_defs should be empty
    assert!(var_defs.vars.is_empty());
    assert!(var_defs.var_lists.is_empty());
}
