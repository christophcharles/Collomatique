use super::*;

// ========== Narrowing Cast: cast? (fallible) ==========

#[test]
fn cast_fallible_success_int_to_int() {
    let input = "pub let f(x: Int | Bool) -> ?Int = x cast? Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn cast_fallible_failure_bool_to_int() {
    let input = "pub let f(x: Int | Bool) -> ?Int = x cast? Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn cast_fallible_none_value_fails() {
    // When we try to cast None to Int, it fails and returns None
    let input = "pub let f(x: ?Int) -> ?Int = x cast? Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn cast_fallible_from_union_with_none() {
    let input = "pub let f(x: Int | Bool | None) -> ?Bool = x cast? Bool;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Test with Bool value - should succeed
    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Bool(false));

    // Test with Int value - should return none
    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::None);

    // Test with None value - should return none
    let result3 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result3, ExprValue::None);
}

#[test]
fn cast_fallible_list_type() {
    let input = "pub let f(x: [Int] | [Bool]) -> ?[Int] = x cast? [Int];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Test with [Int] list - should succeed
    let result1 = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![ExprValue::Int(1), ExprValue::Int(2)])],
        )
        .expect("Should evaluate");
    assert_eq!(
        result1,
        ExprValue::List(vec![ExprValue::Int(1), ExprValue::Int(2)])
    );

    // Test with [Bool] list - should return none
    let result2 = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![ExprValue::Bool(true)])],
        )
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::None);
}

#[test]
fn cast_fallible_in_if_expression() {
    let input = r#"
    pub let f(x: Int | Bool) -> String =
        if (x cast? Int) != none { "is int" } else { "is bool" };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::String("is int".to_string()));

    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::String("is bool".to_string()));
}

// ========== Narrowing Cast: cast! (panicking) ==========

#[test]
fn cast_panic_success_int_to_int() {
    let input = "pub let f(x: Int | Bool) -> Int = x cast! Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn cast_panic_failure_bool_to_int() {
    let input = "pub let f(x: Int | Bool) -> Int = x cast! Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast.quick_eval_fn("main", "f", vec![ExprValue::Bool(true)]);

    match result {
        Err(EvalError::Panic(_)) => {
            // Expected panic
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn cast_panic_success_bool_to_bool() {
    let input = "pub let f(x: Int | Bool) -> Bool = x cast! Bool;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn cast_panic_failure_int_to_bool() {
    let input = "pub let f(x: Int | Bool) -> Bool = x cast! Bool;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast.quick_eval_fn("main", "f", vec![ExprValue::Int(100)]);

    match result {
        Err(EvalError::Panic(_)) => {
            // Expected panic
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn cast_panic_with_none() {
    let input = "pub let f(x: ?Int) -> Int = x cast! Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Test with Int value - should succeed
    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(5));

    // Test with None value - should panic
    let result2 = checked_ast.quick_eval_fn("main", "f", vec![ExprValue::None]);
    match result2 {
        Err(EvalError::Panic(_)) => {
            // Expected panic
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn cast_panic_in_else_branch_triggers() {
    let input = r#"pub let f(x: Int | Bool) -> Int =
        if x cast? Int != none { x cast! Int } else { x cast! Int };"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Bool value triggers else branch, which panics
    let result = checked_ast.quick_eval_fn("main", "f", vec![ExprValue::Bool(true)]);

    match result {
        Err(EvalError::Panic(_)) => {
            // Expected panic
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn cast_panic_in_else_branch_not_triggered() {
    let input = r#"pub let f(x: Int | Bool) -> Int =
        if x cast? Int != none { x cast! Int } else { 0 };"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Int value takes the then branch, no panic
    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate without panic");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn cast_panic_list_type_success() {
    let input = "pub let f(x: [Int] | [Bool]) -> [Bool] = x cast! [Bool];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::List(vec![ExprValue::Bool(true)])],
        )
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::List(vec![ExprValue::Bool(true)]));
}

#[test]
fn cast_panic_list_type_failure() {
    let input = "pub let f(x: [Int] | [Bool]) -> [Bool] = x cast! [Bool];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result =
        checked_ast.quick_eval_fn("main", "f", vec![ExprValue::List(vec![ExprValue::Int(1)])]);

    match result {
        Err(EvalError::Panic(_)) => {
            // Expected panic
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

// ========== Combined cast? and cast! ==========

#[test]
fn cast_fallible_then_panic() {
    // Use cast? to check, then cast! to narrow (safe pattern)
    let input = r#"
    pub let f(x: Int | Bool | String) -> Int =
        if x cast? Int != none {
            x cast! Int
        } else {
            0
        };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(0));

    let result3 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::String("hello".to_string())])
        .expect("Should evaluate");
    assert_eq!(result3, ExprValue::Int(0));
}

#[test]
fn cast_with_tuple() {
    let input = "pub let f(x: (Int, Bool) | (Bool, Int)) -> ?(Int, Bool) = x cast? (Int, Bool);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Test with (Int, Bool) - should succeed
    let result1 = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(1),
                ExprValue::Bool(true),
            ])],
        )
        .expect("Should evaluate");
    assert_eq!(
        result1,
        ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Bool(true)])
    );

    // Test with (Bool, Int) - should return none
    let result2 = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Bool(true),
                ExprValue::Int(1),
            ])],
        )
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::None);
}

#[test]
fn cast_fallible_to_optional_type() {
    // Target type ?Int already contains None
    let input = "pub let f(x: Int | Bool | None) -> ?Int = x cast? ?Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Int fits in ?Int
    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    // None fits in ?Int
    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::None);

    // Bool does not fit in ?Int, returns none
    let result3 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result3, ExprValue::None);
}

#[test]
fn cast_panic_to_optional_type() {
    let input = "pub let f(x: Int | Bool | None) -> ?Int = x cast! ?Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Int fits in ?Int
    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    // None fits in ?Int
    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::None);

    // Bool does not fit in ?Int, panics
    let result3 = checked_ast.quick_eval_fn("main", "f", vec![ExprValue::Bool(true)]);
    match result3 {
        Err(EvalError::Panic(_)) => {}
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

// ========== Null Coalescing Operator ==========

#[test]
fn null_coalesce_returns_value_when_not_none() {
    let input = "pub let f(x: ?Int) -> Int = x ?? 0;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn null_coalesce_returns_default_when_none() {
    let input = "pub let f(x: ?Int) -> Int = x ?? 0;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(0));
}

#[test]
fn null_coalesce_with_different_types() {
    let input = "pub let f(x: ?Int) -> Int | Bool = x ?? true;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // With Int value
    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    // With None
    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Bool(true));
}

#[test]
fn null_coalesce_with_none_default() {
    let input = "pub let f(x: ?Int) -> ?Int = x ?? none;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // With Int value
    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    // With None (no-op)
    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::None);
}

#[test]
fn null_coalesce_chained() {
    let input = "pub let f(x: ?Int, y: ?Int) -> Int = x ?? y ?? 0;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // First has value
    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(1), ExprValue::Int(2)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(1));

    // First is none, second has value
    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None, ExprValue::Int(2)])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(2));

    // Both are none
    let result3 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None, ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result3, ExprValue::Int(0));
}

#[test]
fn null_coalesce_with_expression_default() {
    let input = "pub let f(x: ?Int, y: Int) -> Int = x ?? (y + 10);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None, ExprValue::Int(5)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn null_coalesce_in_larger_expression() {
    let input = "pub let f(x: ?Int) -> Int = (x ?? 0) + 100;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(105));

    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(100));
}

#[test]
fn null_coalesce_with_string() {
    let input = r#"pub let f(x: ?String) -> String = x ?? "default";"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result1 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::String("hello".to_string())])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::String("hello".to_string()));

    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::String("default".to_string()));
}

#[test]
fn null_coalesce_with_list() {
    let input = "pub let f(x: ?[Int]) -> [Int] = x ?? [1, 2, 3];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(3)
        ])
    );
}

#[test]
fn null_coalesce_with_tuple() {
    let input = "pub let f(x: ?(Int, Bool)) -> (Int, Bool) = x ?? (0, false);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // With value
    let result1 = checked_ast
        .quick_eval_fn(
            "main",
            "f",
            vec![ExprValue::Tuple(vec![
                ExprValue::Int(42),
                ExprValue::Bool(true),
            ])],
        )
        .expect("Should evaluate");
    assert_eq!(
        result1,
        ExprValue::Tuple(vec![ExprValue::Int(42), ExprValue::Bool(true)])
    );

    // With none
    let result2 = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::None])
        .expect("Should evaluate");
    assert_eq!(
        result2,
        ExprValue::Tuple(vec![ExprValue::Int(0), ExprValue::Bool(false)])
    );
}

// ========== Implicit Type Coercion: Int → LinExpr ==========

#[test]
fn coercion_int_to_linexpr_in_addition() {
    let input = "pub let f() -> LinExpr = $V() + 5;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
                + LinExpr::constant(5.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn coercion_int_to_linexpr_in_subtraction() {
    let input = "pub let f() -> LinExpr = $V() - 10;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
                - LinExpr::constant(10.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn coercion_int_to_linexpr_both_sides() {
    let input = "pub let f() -> LinExpr = 5 + $V() - 3;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => assert!(true),
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn coercion_int_to_linexpr_in_constraint() {
    let input = "pub let f() -> Constraint = 5 === 10;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            assert_eq!(
                constraints.iter().next().unwrap().constraint,
                LinExpr::constant(5.).eq(&LinExpr::constant(10.))
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn coercion_int_to_linexpr_constraint_le() {
    let input = "pub let f() -> Constraint = 3 <== 7;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            assert_eq!(
                constraints.iter().next().unwrap().constraint,
                LinExpr::constant(3.).leq(&LinExpr::constant(7.))
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn coercion_int_to_linexpr_constraint_ge() {
    let input = "pub let f() -> Constraint = 10 >== 5;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            assert_eq!(
                constraints.iter().next().unwrap().constraint,
                LinExpr::constant(10.).geq(&LinExpr::constant(5.))
            );
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn coercion_int_to_linexpr_with_var() {
    let input = "pub let f() -> Constraint = $V() + 5 === 10;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let expected = (LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
                + LinExpr::constant(5.))
            .eq(&LinExpr::constant(10.));
            assert_eq!(constraints.iter().next().unwrap().constraint, expected);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn coercion_int_param_to_linexpr() {
    let input = "pub let f(x: Int) -> LinExpr = $V() + x;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
                + LinExpr::constant(42.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Implicit Coercion in Collections ==========

#[test]
fn coercion_in_list_unification() {
    let input = "pub let f() -> [Int | LinExpr] = [$V(), 5];";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 2);
            assert!(list
                .iter()
                .all(|x| matches!(x, ExprValue::LinExpr(_) | ExprValue::Int(_))));
        }
        _ => panic!("Expected List of Int | LinExpr"),
    }
}

#[test]
fn coercion_in_list_comprehension() {
    let input = "pub let f() -> [Int | LinExpr] = [x as Int | LinExpr for x in [1, 2, 3]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 3);
            assert!(list.iter().all(|x| matches!(x, ExprValue::Int(_))));
        }
        _ => panic!("Expected List of Int"),
    }
}

#[test]
fn conversion_in_sum_body() {
    let input = "pub let f() -> LinExpr = sum x in [1, 2, 3] { $V(x) + x };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => assert!(true),
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Type Conversion with 'into' ==========

#[test]
fn explicit_cast_int_to_linexpr() {
    let input = "pub let f() -> LinExpr = LinExpr(42);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(42.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn explicit_cast_in_expression() {
    let input = "pub let f() -> LinExpr = (LinExpr(5)) + (LinExpr(10));";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(15.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn explicit_cast_param() {
    let input = "pub let f(x: Int) -> LinExpr = LinExpr(x);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(100)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(100.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn explicit_cast_list_type() {
    let input = "pub let f() -> [Int] = [Int]([]);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[test]
fn explicit_cast_list_of_linexpr() {
    let input = "pub let f() -> [LinExpr] = [LinExpr]([1, 2, 3]);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 3);
            assert!(list.iter().all(|x| matches!(x, ExprValue::LinExpr(_))));
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn explicit_cast_in_sum() {
    let input = "pub let f() -> LinExpr = sum x in [LinExpr(1), 2, 3] { LinExpr(x) };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be 1 + 2 + 3 = 6 as LinExpr
            assert_eq!(lin_expr, LinExpr::constant(6.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn explicit_cast_in_forall() {
    let input =
        "pub let f() -> Constraint = forall x in ([LinExpr]([1, 2])) { x === (LinExpr(1)) };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn explicit_cast_complex_expression() {
    let input = "pub let f(x: Int) -> LinExpr = LinExpr((x + 5) * 2);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // (10 + 5) * 2 = 30
            assert_eq!(lin_expr, LinExpr::constant(30.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn explicit_cast_in_if_branches() {
    let input = "pub let f(x: Int) -> LinExpr = if x > 0 { LinExpr(x) } else { LinExpr(0) };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_positive = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_positive {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(5.));
        }
        _ => panic!("Expected LinExpr"),
    }

    let result_negative = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");

    match result_negative {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(0.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Coercion in Function Return Types ==========

#[test]
fn conversion_return_type_int_to_linexpr() {
    let input = r#"
    let helper() -> Int = 42;
    pub let f() -> LinExpr = LinExpr(helper());
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(42.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn conversion_return_type_with_arithmetic() {
    let input = r#"
    let helper() -> Int = 10;
    pub let f() -> LinExpr = LinExpr(helper() + helper());
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(20.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn conversion_param_to_return_type() {
    let input = "pub let f(x: Int) -> LinExpr = LinExpr(x);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(123)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(123.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Mixed Coercion and Casting ==========

#[test]
fn mixed_implicit_and_explicit_conversion() {
    let input = "pub let f() -> LinExpr = (LinExpr(5)) + $V() + 10;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::constant(5.)
                + LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
                + LinExpr::constant(10.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn conversion_with_collection_operations() {
    let input = "pub let f() -> [LinExpr] = ([LinExpr]([1, 2])) + ([LinExpr]([3, 4]));";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 4);
            assert!(list.iter().all(|x| matches!(x, ExprValue::LinExpr(_))));
        }
        _ => panic!("Expected List of LinExpr"),
    }
}

#[test]
fn nested_casts() {
    let input = "pub let f() -> LinExpr = LinExpr(LinExpr(5));";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(5.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn conversion_in_comparison() {
    let input = "pub let f() -> Bool = (LinExpr(5)) == (LinExpr($V()));";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    // LinExpr values can be compared for equality
    match result {
        ExprValue::Bool(val) => assert!(!val, "The condition should have failed"),
        _ => panic!("Expected Bool"),
    }
}

// ========== Coercion with Quantifiers ==========

#[test]
fn implicit_conversion_in_forall_body() {
    let input = "pub let f() -> Constraint = forall x in [1, 2, 3] { x === 1 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 3);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn conversion_in_sum_to_linexpr() {
    let input = "pub let f() -> LinExpr = LinExpr(sum x in [1, 2, 3] { x });";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Sum of ints coerced to LinExpr
            assert_eq!(lin_expr, LinExpr::constant(6.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Edge Cases ==========

#[test]
fn cast_identity() {
    let input = "pub let f(x: Int) -> Int = x as Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn cast_linexpr_identity() {
    let input = "pub let f() -> LinExpr = ($V() as LinExpr);";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![],)))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn conversion_identity() {
    let input = "pub let f(x: Int) -> Int = Int(x);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn conversion_linexpr_identity() {
    let input = "pub let f() -> LinExpr = LinExpr($V());";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![],)))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn cast_empty_list_typed() {
    let input = "pub let f() -> [LinExpr] = [] as [LinExpr];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[test]
fn cast_empty_list_with_special_syntax() {
    let input = "pub let f() -> [LinExpr] = [<LinExpr>];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[test]
fn cast_in_nested_list() {
    let input = "pub let f() -> [[Int]] = [[] as [Int], [1, 2] as [Int]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 2);
            assert!(list.iter().all(|inner| {
                match inner {
                    ExprValue::List(inner_list) => {
                        inner_list.iter().all(|x| matches!(x, ExprValue::Int(_)))
                    }
                    _ => false,
                }
            }));
        }
        _ => panic!("Expected List of List of Int"),
    }
}

// ========== Conversion to String with 'into' ==========

#[test]
fn convert_int_to_string() {
    let input = r#"pub let f() -> String = String(42);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("42".to_string()));
}

#[test]
fn convert_bool_true_to_string() {
    let input = r#"pub let f() -> String = String(true);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("true".to_string()));
}

#[test]
fn convert_bool_false_to_string() {
    let input = r#"pub let f() -> String = String(false);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("false".to_string()));
}

#[test]
fn convert_string_to_string() {
    let input = r#"pub let f() -> String = String("hello");"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("hello".to_string()));
}

#[test]
fn convert_none_to_string() {
    let input = r#"pub let f() -> String = String(none);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("none".to_string()));
}

#[test]
fn convert_int_list_to_string() {
    let input = r#"pub let f() -> String = String([1, 2, 3]);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("[1, 2, 3]".to_string()));
}

#[test]
fn convert_bool_list_to_string() {
    let input = r#"pub let f() -> String = String([true, false]);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("[true, false]".to_string()));
}

#[test]
fn convert_string_list_to_string() {
    let input = r#"pub let f() -> String = String(["a", "b", "c"]);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String(r#"["a", "b", "c"]"#.to_string()));
}

#[test]
fn convert_empty_list_to_string() {
    let input = r#"pub let f() -> String = String([<Int>]);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("[]".to_string()));
}

#[test]
fn convert_nested_list_to_string() {
    let input = r#"pub let f() -> String = String([[1, 2], [3, 4]]);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("[[1, 2], [3, 4]]".to_string()));
}

#[test]
fn convert_linexpr_to_string() {
    let input = r#"pub let f() -> String = String($V());"#;

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("1*$V()".to_string()));
}

#[test]
fn convert_to_string_in_concatenation() {
    let input = r#"pub let f() -> String = "Value: " + (String(42));"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("Value: 42".to_string()));
}

#[test]
fn convert_to_string_with_param() {
    let input = r#"pub let f(x: Int) -> String = String(x);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(100)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("100".to_string()));
}

#[test]
fn convert_to_string_in_expression() {
    let input = r#"pub let f(x: Int) -> String = "Result: " + String(x + 5);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("Result: 15".to_string()));
}

#[test]
fn convert_to_string_in_if_expression() {
    let input = r#"pub let f(x: Bool) -> String = if x { String(true) } else { String(false) };"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::String("true".to_string()));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::String("false".to_string()));
}

#[test]
fn convert_multiple_types_to_string() {
    let input = r#"pub let f() -> String = (String(42)) + " " + (String(true));"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("42 true".to_string()));
}
