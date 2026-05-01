use super::*;
use std::sync::Arc;

// ========== External/Base Variable Calls ==========

#[tokio::test]
async fn base_var_simple() {
    let input = "pub let f() -> LinExpr = $V();";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(IntLinExpr::var(IlpVar::Base(ExternVar::new(
            "V".into(),
            vec![]
        ))))
    );
}

#[tokio::test]
async fn base_var_with_int_param() {
    let input = "pub let f() -> LinExpr = $V(42);";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(IntLinExpr::var(IlpVar::Base(ExternVar::new(
            "V".into(),
            vec![Arc::new(ExprValue::Int(42))]
        ))))
    );
}

#[tokio::test]
async fn base_var_with_bool_param() {
    let input = "pub let f() -> LinExpr = $V(true);";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Bool)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(IntLinExpr::var(IlpVar::Base(ExternVar::new(
            "V".into(),
            vec![Arc::new(ExprValue::Bool(true))]
        ))))
    );
}

#[tokio::test]
async fn base_var_with_multiple_params() {
    let input = "pub let f() -> LinExpr = $V(1, 2, 3);";

    let vars = HashMap::from([(
        "V".to_string(),
        vec![
            ExprType::simple(SimpleType::Int),
            ExprType::simple(SimpleType::Int),
            ExprType::simple(SimpleType::Int),
        ],
    )]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(IntLinExpr::var(IlpVar::Base(ExternVar::new(
            "V".into(),
            vec![
                Arc::new(ExprValue::Int(1)),
                Arc::new(ExprValue::Int(2)),
                Arc::new(ExprValue::Int(3))
            ]
        ))))
    );
}

#[tokio::test]
async fn base_var_with_function_param() {
    let input = "pub let f(x: Int) -> LinExpr = $V(x);";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Int(42)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(IntLinExpr::var(IlpVar::Base(ExternVar::new(
            "V".into(),
            vec![Arc::new(ExprValue::Int(42))]
        ))))
    );
}

#[tokio::test]
async fn base_var_with_expression_param() {
    let input = "pub let f(x: Int) -> LinExpr = $V(x + 5);";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![ExprValue::Int(10)])
        .await
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(IntLinExpr::var(IlpVar::Base(ExternVar::new(
            "V".into(),
            vec![Arc::new(ExprValue::Int(15))]
        ))))
    );
}

#[tokio::test]
async fn base_var_in_constraint() {
    let input = "pub let f() -> Constraint = $V() === 1;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            assert_eq!(
                constraints.iter().next().unwrap().constraint,
                IntLinExpr::var(IlpVar::Base(ExternVar::new("V".into(), vec![])))
                    .eq(&IntLinExpr::constant(1))
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn base_var_in_arithmetic() {
    let input = "pub let f() -> LinExpr = 3 * $V() + 5;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = 3 * IntLinExpr::var(IlpVar::Base(ExternVar::new("V".into(), vec![])))
                + IntLinExpr::constant(5);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[tokio::test]
async fn multiple_base_vars() {
    let input = "pub let f() -> LinExpr = $V1() + $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "f", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = IntLinExpr::var(IlpVar::Base(ExternVar::new("V1".into(), vec![])))
                + IntLinExpr::var(IlpVar::Base(ExternVar::new("V2".into(), vec![])));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Script Variables (Reified from single Constraint) ==========

#[tokio::test]
async fn script_var_simple_reify() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int) -> LinExpr = $MyVar(x);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "g", vec![ExprValue::Int(5)])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                    "main".to_string(),
                    "MyVar".into(),
                    vec![Arc::new(ExprValue::Int(5))],
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[tokio::test]
async fn script_var_in_constraint() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int) -> Constraint = $MyVar(x) === 0;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "g", vec![ExprValue::Int(10)])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            assert_eq!(
                constraints.iter().next().unwrap().constraint,
                IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                    "main".to_string(),
                    "MyVar".into(),
                    vec![Arc::new(ExprValue::Int(10))],
                )))
                .eq(&IntLinExpr::constant(0))
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn script_var_with_sum() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) <== 1;
    reify f as $MyVar;
    pub let g(xs: [Int]) -> LinExpr = sum x in xs { $MyVar(x) };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let list = ExprValue::List(Vec::from([
        Arc::new(ExprValue::Int(1)),
        Arc::new(ExprValue::Int(2)),
        Arc::new(ExprValue::Int(3)),
    ]));

    let result = checked_ast
        .eval_fn("main", "g", vec![list])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                "main".to_string(),
                "MyVar".into(),
                vec![Arc::new(ExprValue::Int(1))],
            ))) + IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                "main".to_string(),
                "MyVar".into(),
                vec![Arc::new(ExprValue::Int(2))],
            ))) + IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                "main".to_string(),
                "MyVar".into(),
                vec![Arc::new(ExprValue::Int(3))],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[tokio::test]
async fn script_var_with_forall() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(xs: [Int]) -> Constraint = forall x in xs { $MyVar(x) <== 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let list = ExprValue::List(Vec::from([
        Arc::new(ExprValue::Int(1)),
        Arc::new(ExprValue::Int(2)),
    ]));

    let result = checked_ast
        .eval_fn("main", "g", vec![list])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[tokio::test]
async fn script_var_multiple_params() {
    let input = r#"
    let f(x: Int, y: Int) -> Constraint = $V(x, y) === 1;
    reify f as $MyVar;
    pub let g(a: Int, b: Int) -> LinExpr = $MyVar(a, b);
    "#;

    let vars = HashMap::from([(
        "V".to_string(),
        vec![SimpleType::Int.into(), SimpleType::Int.into()],
    )]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "g", vec![ExprValue::Int(3), ExprValue::Int(7)])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                    "main".to_string(),
                    "MyVar".into(),
                    vec![Arc::new(ExprValue::Int(3)), Arc::new(ExprValue::Int(7))],
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[tokio::test]
async fn script_var_no_params() {
    let input = r#"
    let f() -> Constraint = $V() === 1;
    reify f as $MyVar;
    pub let g() -> LinExpr = $MyVar();
    "#;

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "g", vec![])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                    "main".to_string(),
                    "MyVar".into(),
                    vec![],
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[tokio::test]
async fn script_var_with_arithmetic() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int) -> LinExpr = 2 * $MyVar(x) + 5;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "g", vec![ExprValue::Int(10)])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = 2 * IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                "main".to_string(),
                "MyVar".into(),
                vec![Arc::new(ExprValue::Int(10))],
            ))) + IntLinExpr::constant(5);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[tokio::test]
async fn multiple_script_vars() {
    let input = r#"
    let f1(x: Int) -> Constraint = $V1(x) === 1;
    let f2(x: Int) -> Constraint = $V2(x) === 2;
    reify f1 as $MyVar1;
    reify f2 as $MyVar2;
    pub let g(x: Int) -> LinExpr = $MyVar1(x) + $MyVar2(x);
    "#;

    let vars = HashMap::from([
        ("V1".to_string(), vec![ExprType::simple(SimpleType::Int)]),
        ("V2".to_string(), vec![ExprType::simple(SimpleType::Int)]),
    ]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "g", vec![ExprValue::Int(5)])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                "main".to_string(),
                "MyVar1".into(),
                vec![Arc::new(ExprValue::Int(5))],
            ))) + IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                "main".to_string(),
                "MyVar2".into(),
                vec![Arc::new(ExprValue::Int(5))],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[tokio::test]
async fn script_var_and_base_var_mixed() {
    let input = r#"
    let f(x: Int) -> Constraint = $BaseV(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int) -> LinExpr = $MyVar(x) + $BaseV(x);
    "#;

    let vars = HashMap::from([("BaseV".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result = checked_ast
        .eval_fn("main", "g", vec![ExprValue::Int(10)])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                "main".to_string(),
                "MyVar".into(),
                vec![Arc::new(ExprValue::Int(10))],
            ))) + IntLinExpr::var(IlpVar::Base(ExternVar::new(
                "BaseV".into(),
                vec![Arc::new(ExprValue::Int(10))],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Complex Variable Usage ==========

#[tokio::test]
async fn nested_reification_usage() {
    let input = r#"
    let helper(x: Int) -> Constraint = $V(x) === 1;
    reify helper as $H;
    let outer(xs: [Int]) -> Constraint = forall x in xs { $H(x) <== 1 };
    reify outer as $O;
    pub let final(xs: [Int]) -> LinExpr = $O(xs);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let list = ExprValue::List(Vec::from([Arc::new(ExprValue::Int(1))]));

    let result = checked_ast
        .eval_fn("main", "final", vec![list])
        .await
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => assert!(true),
        _ => panic!("Expected LinExpr"),
    }
}

#[tokio::test]
async fn var_in_if_expression() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int, use_var: Bool) -> LinExpr = if use_var { $MyVar(x) } else { LinExpr(0) };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::<SqliteDatabaseDriver>::new(&BTreeMap::from([("main", input)]), vars)
            .await
            .expect("Should compile");

    let result_true = checked_ast
        .eval_fn("main", "g", vec![ExprValue::Int(5), ExprValue::Bool(true)])
        .await
        .expect("Should evaluate");

    match result_true {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                IntLinExpr::var(IlpVar::Script(ScriptVar::new(
                    "main".to_string(),
                    "MyVar".into(),
                    vec![Arc::new(ExprValue::Int(5))]
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }

    let result_false = checked_ast
        .eval_fn("main", "g", vec![ExprValue::Int(5), ExprValue::Bool(false)])
        .await
        .expect("Should evaluate");

    match result_false {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, IntLinExpr::constant(0));
        }
        _ => panic!("Expected LinExpr"),
    }
}
