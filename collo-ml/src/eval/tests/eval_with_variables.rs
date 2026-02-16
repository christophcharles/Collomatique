use super::*;
use crate::Hashed;
use std::sync::Arc;

#[tokio::test]
async fn eval_with_variables_simple_reified_var() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 1;
    reify base as $MyVar;
    pub let f(n: Int) -> Constraint = $MyVar(n) <== 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            "main",
            "f",
            vec![ExprValue::<SqliteDatabaseConnection>::Int(5)],
        )
        .await
        .expect("Should evaluate");

    // Check result is a constraint
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar with args [5] was defined
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(5))]
    ))));

    let my_var_constraints = &var_defs.vars[&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(5))],
    ))];

    // MyVar(5) should have the constraint from base(5): $V(5) === 1
    assert_eq!(my_var_constraints.len(), 1);

    let expected = LinExpr::var(IlpVar::Base(ExternVar::new(
        "V".into(),
        vec![Arc::new(ExprValue::Int(5))],
    )))
    .eq(&LinExpr::constant(1.));

    assert!(my_var_constraints.contains(&expected));
}

#[tokio::test]
async fn eval_with_variables_multiple_calls_same_var() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 1;
    reify base as $MyVar;
    pub let f() -> Constraint = $MyVar(3) <== 1 and $MyVar(7) <== 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables("main", "f", vec![])
        .await
        .expect("Should evaluate");

    // Check result
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar was called with both [3] and [7]
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(3))]
    ))));
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(7))]
    ))));

    // Verify constraints for MyVar(3)
    let my_var_3_constraints = &var_defs.vars[&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(3))],
    ))];
    assert_eq!(my_var_3_constraints.len(), 1);
    let expected_3 = LinExpr::var(IlpVar::Base(ExternVar::new(
        "V".into(),
        vec![Arc::new(ExprValue::Int(3))],
    )))
    .eq(&LinExpr::constant(1.));
    assert!(my_var_3_constraints.contains(&expected_3));

    // Verify constraints for MyVar(7)
    let my_var_7_constraints = &var_defs.vars[&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(7))],
    ))];
    assert_eq!(my_var_7_constraints.len(), 1);
    let expected_7 = LinExpr::var(IlpVar::Base(ExternVar::new(
        "V".into(),
        vec![Arc::new(ExprValue::Int(7))],
    )))
    .eq(&LinExpr::constant(1.));
    assert!(my_var_7_constraints.contains(&expected_7));
}

#[tokio::test]
async fn eval_with_variables_in_forall() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 1;
    reify base as $MyVar;
    pub let f(n: Int) -> Constraint = forall i in [0..n] { $MyVar(i) <== 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            "main",
            "f",
            vec![ExprValue::<SqliteDatabaseConnection>::Int(3)],
        )
        .await
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
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(0))]
    ))));
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(1))]
    ))));
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(2))]
    ))));

    // Verify each has the correct constraint
    for i in 0..3 {
        let my_var_constraints = &var_defs.vars[&Hashed::new((
            "main".to_string(),
            "MyVar".to_string(),
            vec![Arc::new(ExprValue::Int(i))],
        ))];
        assert_eq!(my_var_constraints.len(), 1);
        let expected = LinExpr::var(IlpVar::Base(ExternVar::new(
            "V".into(),
            vec![Arc::new(ExprValue::Int(i))],
        )))
        .eq(&LinExpr::constant(1.));
        assert!(my_var_constraints.contains(&expected));
    }
}

#[tokio::test]
async fn eval_with_variables_multiple_vars() {
    let input = r#"
    let base1(x: Int) -> Constraint = $V1(x) === 1;
    let base2(y: Int) -> Constraint = $V2(y) === 0;
    reify base1 as $Var1;
    reify base2 as $Var2;
    pub let f(a: Int, b: Int) -> Constraint = $Var1(a) <== 1 and $Var2(b) <== 1;
    "#;

    let vars = HashMap::from([
        ("V1".to_string(), vec![ExprType::simple(SimpleType::Int)]),
        ("V2".to_string(), vec![ExprType::simple(SimpleType::Int)]),
    ]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            "main",
            "f",
            vec![
                ExprValue::<SqliteDatabaseConnection>::Int(5),
                ExprValue::Int(10),
            ],
        )
        .await
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
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "Var1".to_string(),
        vec![Arc::new(ExprValue::Int(5))]
    ))));
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "Var2".to_string(),
        vec![Arc::new(ExprValue::Int(10))]
    ))));

    // Verify Var1 constraint
    let var1_constraints = &var_defs.vars[&Hashed::new((
        "main".to_string(),
        "Var1".to_string(),
        vec![Arc::new(ExprValue::Int(5))],
    ))];
    let expected1 = LinExpr::var(IlpVar::Base(ExternVar::new(
        "V1".into(),
        vec![Arc::new(ExprValue::Int(5))],
    )))
    .eq(&LinExpr::constant(1.));
    assert!(var1_constraints.contains(&expected1));

    // Verify Var2 constraint
    let var2_constraints = &var_defs.vars[&Hashed::new((
        "main".to_string(),
        "Var2".to_string(),
        vec![Arc::new(ExprValue::Int(10))],
    ))];
    let expected2 = LinExpr::var(IlpVar::Base(ExternVar::new(
        "V2".into(),
        vec![Arc::new(ExprValue::Int(10))],
    )))
    .eq(&LinExpr::constant(0.));
    assert!(var2_constraints.contains(&expected2));
}

#[tokio::test]
async fn eval_with_variables_var_with_multiple_params() {
    let input = r#"
    let base(x: Int, y: Int) -> Constraint = $V(x, y) === 1;
    reify base as $MyVar;
    pub let f(a: Int, b: Int) -> Constraint = $MyVar(a, b) <== 1;
    "#;

    let vars = HashMap::from([(
        "V".to_string(),
        vec![SimpleType::Int.into(), SimpleType::Int.into()],
    )]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            "main",
            "f",
            vec![
                ExprValue::<SqliteDatabaseConnection>::Int(3),
                ExprValue::Int(7),
            ],
        )
        .await
        .expect("Should evaluate");

    // Check result
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar(3, 7) was defined
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(3)), Arc::new(ExprValue::Int(7))]
    ))));

    let my_var_constraints = &var_defs.vars[&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(3)), Arc::new(ExprValue::Int(7))],
    ))];
    assert_eq!(my_var_constraints.len(), 1);
    let expected = LinExpr::var(IlpVar::Base(ExternVar::new(
        "V".into(),
        vec![Arc::new(ExprValue::Int(3)), Arc::new(ExprValue::Int(7))],
    )))
    .eq(&LinExpr::constant(1.));
    assert!(my_var_constraints.contains(&expected));
}

#[tokio::test]
async fn eval_with_variables_with_let_expr() {
    let input = r#"
    let base(x: Int) -> Constraint = $V(x) === 1;
    reify base as $MyVar;
    pub let f(n: Int) -> Constraint = 
        let bound = n * 2 {
            $MyVar(bound) <== 1
        };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            "main",
            "f",
            vec![ExprValue::<SqliteDatabaseConnection>::Int(5)],
        )
        .await
        .expect("Should evaluate");

    // Check result
    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }

    // Check that MyVar(10) was defined (bound = 5 * 2 = 10)
    assert!(var_defs.vars.contains_key(&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(10))]
    ))));

    let my_var_constraints = &var_defs.vars[&Hashed::new((
        "main".to_string(),
        "MyVar".to_string(),
        vec![Arc::new(ExprValue::Int(10))],
    ))];
    assert_eq!(my_var_constraints.len(), 1);
    let expected = LinExpr::var(IlpVar::Base(ExternVar::new(
        "V".into(),
        vec![Arc::new(ExprValue::Int(10))],
    )))
    .eq(&LinExpr::constant(1.));
    assert!(my_var_constraints.contains(&expected));
}

#[tokio::test]
async fn eval_with_variables_no_reified_vars() {
    let input = r#"
    pub let f(x: Int) -> Constraint = $V(x) === 1;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let (result, var_defs) = checked_ast
        .eval_fn_with_variables(
            "main",
            "f",
            vec![ExprValue::<SqliteDatabaseConnection>::Int(5)],
        )
        .await
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
}
