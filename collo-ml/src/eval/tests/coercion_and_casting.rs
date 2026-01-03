use super::*;

// ========== Implicit Type Coercion: Int → LinExpr ==========

#[test]
fn coercion_int_to_linexpr_in_addition() {
    let input = "pub let f() -> LinExpr = $V() + 5;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![ExprValue::Int(42)])
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
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => assert!(true),
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Type Conversion with 'into' ==========

#[test]
fn explicit_cast_int_to_linexpr() {
    let input = "pub let f() -> LinExpr = 42 into LinExpr;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f() -> LinExpr = (5 into LinExpr) + (10 into LinExpr);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f(x: Int) -> LinExpr = x into LinExpr;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(100)])
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
    let input = "pub let f() -> [Int] = [] into [Int];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[test]
fn explicit_cast_list_of_linexpr() {
    let input = "pub let f() -> [LinExpr] = [1, 2, 3] into [LinExpr];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f() -> LinExpr = sum x in [1 into LinExpr, 2, 3] { x into LinExpr };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f() -> Constraint = forall x in ([1, 2] into [LinExpr]) { x === (1 into LinExpr) };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f(x: Int) -> LinExpr = ((x + 5) * 2) into LinExpr;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(10)])
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
    let input =
        "pub let f(x: Int) -> LinExpr = if x > 0 { x into LinExpr } else { 0 into LinExpr };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_positive = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result_positive {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(lin_expr, LinExpr::constant(5.));
        }
        _ => panic!("Expected LinExpr"),
    }

    let result_negative = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(-5)])
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
    pub let f() -> LinExpr = helper() into LinExpr;
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    pub let f() -> LinExpr = (helper() + helper()) into LinExpr;
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f(x: Int) -> LinExpr = x into LinExpr;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(123)])
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
    let input = "pub let f() -> LinExpr = (5 into LinExpr) + $V() + 10;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f() -> [LinExpr] = ([1, 2] into [LinExpr]) + ([3, 4] into [LinExpr]);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f() -> LinExpr = ((5 into LinExpr) into LinExpr);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f() -> Bool = (5 into LinExpr) == ($V() into LinExpr);";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f() -> LinExpr = sum x in [1, 2, 3] { x } into LinExpr;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn cast_linexpr_identity() {
    let input = "pub let f() -> LinExpr = ($V() as LinExpr);";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = "pub let f(x: Int) -> Int = x into Int;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn conversion_linexpr_identity() {
    let input = "pub let f() -> LinExpr = $V() into LinExpr;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[test]
fn cast_empty_list_with_special_syntax() {
    let input = "pub let f() -> [LinExpr] = [<LinExpr>];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::List(Vec::new()));
}

#[test]
fn cast_in_nested_list() {
    let input = "pub let f() -> [[Int]] = [[] as [Int], [1, 2] as [Int]];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
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
    let input = r#"pub let f() -> String = 42 into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("42".to_string()));
}

#[test]
fn convert_bool_true_to_string() {
    let input = r#"pub let f() -> String = true into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("true".to_string()));
}

#[test]
fn convert_bool_false_to_string() {
    let input = r#"pub let f() -> String = false into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("false".to_string()));
}

#[test]
fn convert_string_to_string() {
    let input = r#"pub let f() -> String = "hello" into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("hello".to_string()));
}

#[test]
fn convert_none_to_string() {
    let input = r#"pub let f() -> String = none into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("none".to_string()));
}

#[test]
fn convert_int_list_to_string() {
    let input = r#"pub let f() -> String = [1, 2, 3] into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("[1, 2, 3]".to_string()));
}

#[test]
fn convert_bool_list_to_string() {
    let input = r#"pub let f() -> String = [true, false] into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("[true, false]".to_string()));
}

#[test]
fn convert_string_list_to_string() {
    let input = r#"pub let f() -> String = ["a", "b", "c"] into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String(r#"["a", "b", "c"]"#.to_string()));
}

#[test]
fn convert_empty_list_to_string() {
    let input = r#"pub let f() -> String = [<Int>] into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("[]".to_string()));
}

#[test]
fn convert_nested_list_to_string() {
    let input = r#"pub let f() -> String = [[1, 2], [3, 4]] into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("[[1, 2], [3, 4]]".to_string()));
}

#[test]
fn convert_linexpr_to_string() {
    let input = r#"pub let f() -> String = $V() into String;"#;

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("1*$V()".to_string()));
}

#[test]
fn convert_to_string_in_concatenation() {
    let input = r#"pub let f() -> String = "Value: " + (42 into String);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("Value: 42".to_string()));
}

#[test]
fn convert_to_string_with_param() {
    let input = r#"pub let f(x: Int) -> String = x into String;"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(100)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("100".to_string()));
}

#[test]
fn convert_to_string_in_expression() {
    let input = r#"pub let f(x: Int) -> String = "Result: " + ((x + 5) into String);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("Result: 15".to_string()));
}

#[test]
fn convert_to_string_in_if_expression() {
    let input =
        r#"pub let f(x: Bool) -> String = if x { true into String } else { false into String };"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::String("true".to_string()));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::String("false".to_string()));
}

#[test]
fn convert_multiple_types_to_string() {
    let input = r#"pub let f() -> String = (42 into String) + " " + (true into String);"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("42 true".to_string()));
}
