use super::*;

// ========== External/Base Variable Calls ==========

#[test]
fn base_var_simple() {
    let input = "pub let f() -> LinExpr = $V();";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
            "V".into(),
            vec![]
        ))))
    );
}

#[test]
fn base_var_with_int_param() {
    let input = "pub let f() -> LinExpr = $V(42);";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
            "V".into(),
            vec![ExprValue::Int(42)]
        ))))
    );
}

#[test]
fn base_var_with_bool_param() {
    let input = "pub let f() -> LinExpr = $V(true);";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Bool)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
            "V".into(),
            vec![ExprValue::Bool(true)]
        ))))
    );
}

#[test]
fn base_var_with_multiple_params() {
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
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
            "V".into(),
            vec![ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]
        ))))
    );
}

#[test]
fn base_var_with_function_param() {
    let input = "pub let f(x: Int) -> LinExpr = $V(x);";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
            "V".into(),
            vec![ExprValue::Int(42)]
        ))))
    );
}

#[test]
fn base_var_with_expression_param() {
    let input = "pub let f(x: Int) -> LinExpr = $V(x + 5);";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::LinExpr(LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
            "V".into(),
            vec![ExprValue::Int(15)]
        ))))
    );
}

#[test]
fn base_var_in_constraint() {
    let input = "pub let f() -> Constraint = $V() === 1;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            assert_eq!(
                constraints.iter().next().unwrap().constraint,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
                    .eq(&LinExpr::constant(1.))
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn base_var_in_arithmetic() {
    let input = "pub let f() -> LinExpr = 3 * $V() + 5;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected =
                3 * LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
                    + LinExpr::constant(5.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn multiple_base_vars() {
    let input = "pub let f() -> LinExpr = $V1() + $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V1".into(), vec![])))
                + LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V2".into(), vec![])));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Script Variables (Reified from single Constraint) ==========

#[test]
fn script_var_simple_reify() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int) -> LinExpr = $MyVar(x);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVar".into(),
                    None,
                    vec![ExprValue::Int(5)],
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn script_var_in_constraint() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int) -> Constraint = $MyVar(x) === 0;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            assert_eq!(
                constraints.iter().next().unwrap().constraint,
                LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVar".into(),
                    None,
                    vec![ExprValue::Int(10)],
                )))
                .eq(&LinExpr::constant(0.))
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn script_var_with_sum() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) <== 1;
    reify f as $MyVar;
    pub let g(xs: [Int]) -> LinExpr = sum x in xs { $MyVar(x) };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([
        ExprValue::Int(1),
        ExprValue::Int(2),
        ExprValue::Int(3),
    ]));

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![list])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar".into(),
                None,
                vec![ExprValue::Int(1)],
            ))) + LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar".into(),
                None,
                vec![ExprValue::Int(2)],
            ))) + LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar".into(),
                None,
                vec![ExprValue::Int(3)],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn script_var_with_forall() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(xs: [Int]) -> Constraint = forall x in xs { $MyVar(x) <== 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([ExprValue::Int(1), ExprValue::Int(2)]));

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![list])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn script_var_multiple_params() {
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
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![ExprValue::Int(3), ExprValue::Int(7)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVar".into(),
                    None,
                    vec![ExprValue::Int(3), ExprValue::Int(7)],
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn script_var_no_params() {
    let input = r#"
    let f() -> Constraint = $V() === 1;
    reify f as $MyVar;
    pub let g() -> LinExpr = $MyVar();
    "#;

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVar".into(),
                    None,
                    vec![],
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn script_var_with_arithmetic() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int) -> LinExpr = 2 * $MyVar(x) + 5;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = 2 * LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar".into(),
                None,
                vec![ExprValue::Int(10)],
            ))) + LinExpr::constant(5.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn multiple_script_vars() {
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
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar1".into(),
                None,
                vec![ExprValue::Int(5)],
            ))) + LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar2".into(),
                None,
                vec![ExprValue::Int(5)],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn script_var_and_base_var_mixed() {
    let input = r#"
    let f(x: Int) -> Constraint = $BaseV(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int) -> LinExpr = $MyVar(x) + $BaseV(x);
    "#;

    let vars = HashMap::from([("BaseV".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar".into(),
                None,
                vec![ExprValue::Int(10)],
            ))) + LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "BaseV".into(),
                vec![ExprValue::Int(10)],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Variable List Calls (Reified from [Constraint]) ==========

#[test]
fn var_list_simple_reify() {
    let input = r#"
    let h(xs: [Int]) -> [Constraint] = [$V(x) <== 1 for x in xs];
    reify h as $[MyVars];
    pub let i(xs: [Int]) -> [LinExpr] = $[MyVars](xs);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([
        ExprValue::Int(1),
        ExprValue::Int(2),
        ExprValue::Int(3),
    ]));

    let result = checked_ast
        .quick_eval_fn("main", "i", vec![list.clone()])
        .expect("Should evaluate");

    match result {
        ExprValue::List(linexprs) => {
            assert_eq!(linexprs.len(), 3);

            // Verify the LinExprs are script vars with from_list set
            for (idx, linexpr) in linexprs.iter().enumerate() {
                match linexpr {
                    ExprValue::LinExpr(le) => {
                        assert_eq!(
                            le,
                            &LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                                "main".to_string(),
                                "MyVars".into(),
                                Some(idx),
                                vec![list.clone()],
                            )))
                        );
                    }
                    _ => panic!("Expected LinExpr in list"),
                }
            }
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn var_list_in_sum() {
    let input = r#"
    let f(xs: [Int]) -> [Constraint] = [$V(x) <== 1 for x in xs];
    reify f as $[MyVars];
    pub let g(xs: [Int]) -> LinExpr = sum v in $[MyVars](xs) { v };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([ExprValue::Int(1), ExprValue::Int(2)]));

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![list.clone()])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVars".into(),
                    Some(0),
                    vec![list.clone()]
                ))) + LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVars".into(),
                    Some(1),
                    vec![list.clone()]
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn var_list_in_constraint() {
    let input = r#"
    let f(xs: [Int]) -> [Constraint] = [$V(x) === 1 for x in xs];
    reify f as $[MyVars];
    pub let g(xs: [Int]) -> Constraint = sum v in $[MyVars](xs) { v } <== 10;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([
        ExprValue::Int(1),
        ExprValue::Int(2),
        ExprValue::Int(3),
    ]));

    let result = checked_ast
        .quick_eval_fn("main", "g", vec![list.clone()])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraint = constraints.iter().next().unwrap().constraint.clone();
            assert_eq!(
                constraint,
                (LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVars".into(),
                    Some(0),
                    vec![list.clone()]
                ))) + LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVars".into(),
                    Some(1),
                    vec![list.clone()]
                ))) + LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVars".into(),
                    Some(2),
                    vec![list.clone()]
                ))))
                .leq(&LinExpr::constant(10.))
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn var_list_with_forall() {
    let input = r#"
    let h(xs: [Int]) -> [Constraint] = [$V(x) === 1 for x in xs];
    reify h as $[MyVars];
    pub let i(xs: [Int]) -> Constraint = forall v in $[MyVars](xs) { v <== 1 };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([ExprValue::Int(1), ExprValue::Int(2)]));

    let result = checked_ast
        .quick_eval_fn("main", "i", vec![list.clone()])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVars".into(),
                Some(0),
                vec![list.clone()],
            )))
            .leq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint));

            let constraint = LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVars".into(),
                Some(1),
                vec![list.clone()],
            )))
            .leq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn var_list_cardinality() {
    let input = r#"
    let h(xs: [Int]) -> [Constraint] = [$V(x) === 1 for x in xs];
    reify h as $[MyVars];
    pub let i(xs: [Int]) -> Int = |$[MyVars](xs)|;
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([
        ExprValue::Int(1),
        ExprValue::Int(2),
        ExprValue::Int(3),
    ]));

    let result = checked_ast
        .quick_eval_fn("main", "i", vec![list])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn var_list_with_multiple_params() {
    let input = r#"
    let h(xs: [Int], y: Int) -> [Constraint] = [$V(x, y) === 1 for x in xs];
    reify h as $[MyVars];
    pub let i(xs: [Int], y: Int) -> [LinExpr] = $[MyVars](xs, y);
    "#;
    let vars = HashMap::from([(
        "V".to_string(),
        vec![SimpleType::Int.into(), SimpleType::Int.into()],
    )]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([ExprValue::Int(1), ExprValue::Int(2)]));

    let result = checked_ast
        .quick_eval_fn("main", "i", vec![list.clone(), ExprValue::Int(10)])
        .expect("Should evaluate");

    match result {
        ExprValue::List(linexprs) => {
            assert_eq!(linexprs.len(), 2);
            let lin_expr1 =
                ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVars".into(),
                    Some(0),
                    vec![list.clone(), ExprValue::Int(10)],
                ))));
            assert!(linexprs.contains(&lin_expr1));
            let lin_expr2 =
                ExprValue::LinExpr(LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVars".into(),
                    Some(1),
                    vec![list.clone(), ExprValue::Int(10)],
                ))));
            assert!(linexprs.contains(&lin_expr2));
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn var_list_empty_input() {
    let input = r#"
    let h(xs: [Int]) -> [Constraint] = [$V(x) === 1 for x in xs];
    reify h as $[MyVars];
    pub let i(xs: [Int]) -> [LinExpr] = $[MyVars](xs);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let empty_list = ExprValue::List(Vec::new());

    let result = checked_ast
        .quick_eval_fn("main", "i", vec![empty_list])
        .expect("Should evaluate");

    match result {
        ExprValue::List(linexprs) => {
            assert_eq!(linexprs.len(), 0);
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn var_list_in_list_comprehension() {
    let input = r#"
    let h(xs: [Int]) -> [Constraint] = [$V(x) === 1 for x in xs];
    reify h as $[MyVars];
    pub let i(xs: [Int]) -> [LinExpr] = [v * 2 for v in $[MyVars](xs)];
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([ExprValue::Int(1), ExprValue::Int(2)]));

    let result = checked_ast
        .quick_eval_fn("main", "i", vec![list])
        .expect("Should evaluate");

    match result {
        ExprValue::List(linexprs) => {
            assert_eq!(linexprs.len(), 2);
            assert!(linexprs.iter().all(|x| matches!(x, ExprValue::LinExpr(_))));
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn var_list_with_collection_ops() {
    let input = r#"
    let h(xs: [Int]) -> [Constraint] = [$V(x) === 1 for x in xs];
    reify h as $[MyVars];
    pub let i(xs: [Int], ys: [Int]) -> [LinExpr] = $[MyVars](xs) + $[MyVars](ys);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list1 = ExprValue::List(Vec::from([ExprValue::Int(1)]));
    let list2 = ExprValue::List(Vec::from([ExprValue::Int(2)]));

    let result = checked_ast
        .quick_eval_fn("main", "i", vec![list1, list2])
        .expect("Should evaluate");

    match result {
        ExprValue::List(linexprs) => {
            // Union of two var lists
            assert!(linexprs.len() >= 2);
            assert!(linexprs.iter().all(|x| matches!(x, ExprValue::LinExpr(_))));
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

// ========== Complex Variable Usage ==========

#[test]
fn nested_reification_usage() {
    let input = r#"
    let helper(x: Int) -> Constraint = $V(x) === 1;
    reify helper as $H;
    let outer(xs: [Int]) -> Constraint = forall x in xs { $H(x) <== 1 };
    reify outer as $O;
    pub let final(xs: [Int]) -> LinExpr = $O(xs);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([ExprValue::Int(1)]));

    let result = checked_ast
        .quick_eval_fn("main", "final", vec![list])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => assert!(true),
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn var_in_if_expression() {
    let input = r#"
    let f(x: Int) -> Constraint = $V(x) === 1;
    reify f as $MyVar;
    pub let g(x: Int, use_var: Bool) -> LinExpr = if use_var { $MyVar(x) } else { LinExpr(0) };
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "g", vec![ExprValue::Int(5), ExprValue::Bool(true)])
        .expect("Should evaluate");

    match result_true {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                    "main".to_string(),
                    "MyVar".into(),
                    None,
                    vec![ExprValue::Int(5)]
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }

    let result_false = checked_ast
        .quick_eval_fn("main", "g", vec![ExprValue::Int(5), ExprValue::Bool(false)])
        .expect("Should evaluate");

    match result_false {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(0.));
        }
        _ => panic!("Expected LinExpr"),
    }
}
