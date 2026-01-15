use super::*;

// ========== Addition Tests ==========

#[test]
fn add_two_ints() {
    let input = "pub let f() -> Int = 5 + 3;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(8));
}

#[test]
fn add_negative_ints() {
    let input = "pub let f() -> Int = -10 + 7;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(-3));
}

#[test]
fn add_int_params() {
    let input = "pub let f(x: Int, y: Int) -> Int = x + y;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(15), ExprValue::Int(27)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn negate_int_params() {
    let input = "pub let f(x: Int) -> Int = -x;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(15)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(-15));
}

#[test]
fn add_linexpr_with_int_coercion() {
    let input = "pub let f() -> LinExpr = $V() + 5;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: 1 * $V() + 5
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![]))) + 5.
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn add_int_to_linexpr_coercion() {
    let input = "pub let f() -> LinExpr = 10 + $V();";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: 1 * $V() + 10
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![]))) + 10.
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn add_two_linexprs() {
    let input = "pub let f() -> LinExpr = $V1() + $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: $V1() + $V2()
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V1".into(), vec![])))
                    + LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V2".into(), vec![])))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn negate_linexpr_params() {
    let input = "pub let f(x: LinExpr) -> LinExpr = -x;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::LinExpr(LinExpr::constant(5.))])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::LinExpr(LinExpr::constant(-5.)));
}

#[test]
fn add_chain() {
    let input = "pub let f() -> Int = 1 + 2 + 3 + 4;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(10));
}

// ========== Subtraction Tests ==========

#[test]
fn sub_two_ints() {
    let input = "pub let f() -> Int = 10 - 3;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(7));
}

#[test]
fn sub_negative_result() {
    let input = "pub let f() -> Int = 5 - 12;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(-7));
}

#[test]
fn sub_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Int = x - y;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(50), ExprValue::Int(8)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn sub_linexpr_with_int() {
    let input = "pub let f() -> LinExpr = $V() - 3;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![]))) - 3
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn sub_two_linexprs() {
    let input = "pub let f() -> LinExpr = $V1() - $V2();";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V1".into(), vec![])))
                    - LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V2".into(), vec![])))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Multiplication Tests ==========

#[test]
fn mul_two_ints() {
    let input = "pub let f() -> Int = 6 * 7;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn mul_with_zero() {
    let input = "pub let f() -> Int = 42 * 0;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(0));
}

#[test]
fn mul_with_negative() {
    let input = "pub let f() -> Int = -5 * 3;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(-15));
}

#[test]
fn mul_int_with_linexpr() {
    let input = "pub let f() -> LinExpr = 5 * $V();";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                5 * LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn mul_linexpr_with_int() {
    let input = "pub let f() -> LinExpr = $V() * 3;";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                3 * LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn mul_with_param() {
    let input = "pub let f(x: Int) -> LinExpr = x * $V();";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                10 * LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn mul_chain() {
    let input = "pub let f() -> Int = 2 * 3 * 7;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

// ========== Division Tests ==========

#[test]
fn div_two_ints() {
    let input = "pub let f() -> Int = 10 / 2;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(5));
}

#[test]
fn div_integer_division() {
    let input = "pub let f() -> Int = 7 / 2;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn div_exact() {
    let input = "pub let f() -> Int = 42 / 6;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(7));
}

#[test]
fn div_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Int = x / y;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(100), ExprValue::Int(4)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(25));
}

#[test]
fn div_negative_numerator() {
    let input = "pub let f() -> Int = -10 / 3;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(-3));
}

// ========== Modulo Tests ==========

#[test]
fn mod_two_ints() {
    let input = "pub let f() -> Int = 10 % 3;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(1));
}

#[test]
fn mod_exact_division() {
    let input = "pub let f() -> Int = 12 % 4;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(0));
}

#[test]
fn mod_larger_than_divisor() {
    let input = "pub let f() -> Int = 5 % 10;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(5));
}

#[test]
fn mod_with_params() {
    let input = "pub let f(x: Int, y: Int) -> Int = x % y;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(17), ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(2));
}

#[test]
fn mod_check_even() {
    let input = "pub let f(x: Int) -> Bool = x % 2 == 0;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result_even = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(4)])
        .expect("Should evaluate");
    assert_eq!(result_even, ExprValue::Bool(true));

    let result_odd = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_odd, ExprValue::Bool(false));
}

// ========== Mixed Operations Tests ==========

#[test]
fn mixed_add_mul_precedence() {
    let input = "pub let f() -> Int = 2 + 3 * 4;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Should be 2 + 12 = 14, not (2 + 3) * 4 = 20
    assert_eq!(result, ExprValue::Int(14));
}

#[test]
fn mixed_sub_div_precedence() {
    let input = "pub let f() -> Int = 20 - 10 / 2;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // Should be 20 - 5 = 15, not (20 - 10) // 2 = 5
    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn mixed_operations_with_parentheses() {
    let input = "pub let f() -> Int = (5 + 3) * 2;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(16));
}

#[test]
fn complex_arithmetic_expression() {
    let input = "pub let f() -> Int = (10 + 5) * 2 - 8 / 4 + 3 % 2;";

    let vars = HashMap::new();

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // (15) * 2 - 2 + 1 = 30 - 2 + 1 = 29
    assert_eq!(result, ExprValue::Int(29));
}

#[test]
fn linexpr_arithmetic_combination() {
    let input = "pub let f() -> LinExpr = 2 * $V1() + 3 * $V2() - 5;";

    let vars = HashMap::from([("V1".to_string(), vec![]), ("V2".to_string(), vec![])]);

    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => assert!(true),
        _ => panic!("Expected LinExpr"),
    }
}
