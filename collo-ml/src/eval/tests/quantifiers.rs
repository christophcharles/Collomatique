use super::*;

// ========== SUM with Int Tests ==========

#[test]
fn sum_simple_range() {
    let input = "pub let f() -> Int = sum x in [1..4] { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // 1 + 2 + 3 = 6
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn sum_empty_list() {
    let input = "pub let f() -> Int = sum x in [] as [Int] { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(0));
}

#[test]
fn sum_with_constant_body() {
    let input = "pub let f() -> Int = sum x in [1..5] { 10 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // 4 iterations * 10 = 40
    assert_eq!(result, ExprValue::Int(40));
}

#[test]
fn sum_with_arithmetic_body() {
    let input = "pub let f() -> Int = sum x in [1..5] { x * 2 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // (1*2) + (2*2) + (3*2) + (4*2) = 2 + 4 + 6 + 8 = 20
    assert_eq!(result, ExprValue::Int(20));
}

#[test]
fn sum_with_filter() {
    let input = "pub let f() -> Int = sum x in [1..6] where x % 2 == 0 { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Only even numbers: 2 + 4 = 6
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn sum_with_filter_no_matches() {
    let input = "pub let f() -> Int = sum x in [1..5] where x > 10 { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // No elements pass the filter
    assert_eq!(result, ExprValue::Int(0));
}

#[test]
fn sum_with_complex_filter() {
    let input = "pub let f() -> Int = sum x in [1..10] where x > 3 and x < 7 { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Elements 4, 5, 6: 4 + 5 + 6 = 15
    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn sum_with_param_list() {
    let input = "pub let f(list: [Int]) -> Int = sum x in list { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        Vec::from([ExprValue::Int(10), ExprValue::Int(20), ExprValue::Int(30)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(60));
}

#[test]
fn sum_with_param_in_body() {
    let input = "pub let f(multiplier: Int) -> Int = sum x in [1..4] { x * multiplier };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    // (1*5) + (2*5) + (3*5) = 5 + 10 + 15 = 30
    assert_eq!(result, ExprValue::Int(30));
}

#[test]
fn sum_with_param_in_filter() {
    let input = "pub let f(threshold: Int) -> Int = sum x in [1..10] where x > threshold { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    // Elements > 5: 6 + 7 + 8 + 9 = 30
    assert_eq!(result, ExprValue::Int(30));
}

#[test]
fn sum_nested_arithmetic() {
    let input = "pub let f() -> Int = sum x in [1..3] { sum y in [1..3] { x * y } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // x=1: (1*1)+(1*2) = 3
    // x=2: (2*1)+(2*2) = 6
    // Total: 3 + 6 = 9
    assert_eq!(result, ExprValue::Int(9));
}

#[test]
fn sum_with_explicit_list() {
    let input = "pub let f() -> Int = sum x in [5, 10, 15] { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(30));
}

// ========== SUM with LinExpr Tests ==========

#[test]
fn sum_linexpr_simple() {
    let input = "pub let f() -> LinExpr = sum x in [1..3] { $V(x) };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: $V(1) + $V(2)
            let expected = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            }));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn sum_linexpr_empty_list() {
    let input = "pub let f() -> LinExpr = sum x in [] as [Int] { $V(x) };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be constant 0
            assert_eq!(lin_expr, LinExpr::constant(0.));
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn sum_linexpr_with_coefficient() {
    let input = "pub let f() -> LinExpr = sum x in [1..3] { x * $V() };";

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: 1*$V() + 2*$V() = 3*$V()
            let expected = 3 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![],
            }));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn sum_linexpr_with_constant() {
    let input = "pub let f() -> LinExpr = sum x in [1..4] { $V(x) + 10 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: ($V(1)+10) + ($V(2)+10) + ($V(3)+10) = $V(1) + $V(2) + $V(3) + 30
            let expected = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(3)],
            })) + LinExpr::constant(30.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn sum_linexpr_with_filter() {
    let input = "pub let f() -> LinExpr = sum x in [1..5] where x % 2 == 1 { $V(x) };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: $V(1) + $V(3) (odd numbers only)
            let expected = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(3)],
            }));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn sum_linexpr_multiple_vars() {
    let input = "pub let f() -> LinExpr = sum x in [1..3] { $V1(x) + $V2(x) };";

    let vars = HashMap::from([
        ("V1".to_string(), vec![ExprType::Int]),
        ("V2".to_string(), vec![ExprType::Int]),
    ]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![ExprValue::Int(2)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V2".into(),
                params: vec![ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V2".into(),
                params: vec![ExprValue::Int(2)],
            }));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn sum_linexpr_with_param() {
    let input = "pub let f(coef: Int) -> LinExpr = sum x in [1..3] { coef * $V(x) };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            // Should be: 5*$V(1) + 5*$V(2)
            let expected = 5 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            })) + 5 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            }));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== FORALL with Bool Tests ==========

#[test]
fn forall_bool_all_true() {
    let input = "pub let f() -> Bool = forall x in [1..4] { x > 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn forall_bool_one_false() {
    let input = "pub let f() -> Bool = forall x in [1..5] { x < 3 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Not all elements are < 3
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn forall_bool_empty_list() {
    let input = "pub let f() -> Bool = forall x in [] as [Int] { x > 10 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Vacuously true for empty list
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn forall_bool_with_filter() {
    let input = "pub let f() -> Bool = forall x in [1..10] where x % 2 == 0 { x < 8 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Even numbers: 2, 4, 6, 8
    // Not all are < 8 (8 fails)
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn forall_bool_with_filter_all_pass() {
    let input = "pub let f() -> Bool = forall x in [1..10] where x % 2 == 1 { x < 10 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Odd numbers: 1, 3, 5, 7, 9 - all < 10
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn forall_bool_with_filter_no_matches() {
    let input = "pub let f() -> Bool = forall x in [1..5] where x > 10 { x < 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // No elements pass the filter, vacuously true
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn forall_bool_with_param_list() {
    let input = "pub let f(list: [Int]) -> Bool = forall x in list { x > 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let all_positive = ExprValue::List(
        ExprType::Int,
        Vec::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
    );
    let result_true = checked_ast
        .quick_eval_fn("f", vec![all_positive])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let has_negative = ExprValue::List(
        ExprType::Int,
        Vec::from([ExprValue::Int(1), ExprValue::Int(-2), ExprValue::Int(3)]),
    );
    let result_false = checked_ast
        .quick_eval_fn("f", vec![has_negative])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn forall_bool_with_param_in_body() {
    let input = "pub let f(threshold: Int) -> Bool = forall x in [1..5] { x < threshold };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(3)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn forall_bool_nested() {
    let input = "pub let f() -> Bool = forall x in [1..3] { forall y in [1..3] { x + y > 0 } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn forall_bool_with_complex_condition() {
    let input = "pub let f() -> Bool = forall x in [1..10] { x > 0 and x < 11 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

// ========== FORALL with Constraint Tests ==========

#[test]
fn forall_constraint_simple() {
    let input = "pub let f() -> Constraint = forall x in [1..3] { $V(x) === 1 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Should have 2 constraints (for x=1 and x=2)
            assert_eq!(constraints.len(), 2);

            let constraints = strip_origins(&constraints);

            // Check first constraint: $V(1) === 1
            let constraint1 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            }))
            .eq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint1));

            // Check second constraint: $V(2) === 1
            let constraint2 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            }))
            .eq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint2));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_empty_list() {
    let input = "pub let f() -> Constraint = forall x in [] as [Int] { $V(x) === 1 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Empty list should produce no constraints
            assert_eq!(constraints.len(), 0);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_with_inequality() {
    let input = "pub let f() -> Constraint = forall x in [1..3] { $V(x) <== 10 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
            let constraints = strip_origins(&constraints);

            // Check constraints are <= constraints
            let constraint1 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            }))
            .leq(&LinExpr::constant(10.));
            assert!(constraints.contains(&constraint1));

            let constraint2 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            }))
            .leq(&LinExpr::constant(10.));
            assert!(constraints.contains(&constraint2));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_with_filter() {
    let input = "pub let f() -> Constraint = forall x in [1..6] where x % 2 == 0 { $V(x) === 1 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Only even numbers: 2, 4
            assert_eq!(constraints.len(), 2);
            let constraints = strip_origins(&constraints);

            let constraint1 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            }))
            .eq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint1));

            let constraint2 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(4)],
            }))
            .eq(&LinExpr::constant(1.));
            assert!(constraints.contains(&constraint2));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_with_filter_no_matches() {
    let input = "pub let f() -> Constraint = forall x in [1..5] where x > 10 { $V(x) === 1 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // No elements pass filter, no constraints generated
            assert_eq!(constraints.len(), 0);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_with_arithmetic() {
    let input = "pub let f() -> Constraint = forall x in [1..3] { 2 * $V(x) + 5 === 15 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
            let constraints = strip_origins(&constraints);

            // Check first constraint: 2*$V(1) + 5 === 15
            let constraint1 = (2 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            })) + LinExpr::constant(5.))
            .eq(&LinExpr::constant(15.));
            assert!(constraints.contains(&constraint1));

            let constraint2 = (2 * LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            })) + LinExpr::constant(5.))
            .eq(&LinExpr::constant(15.));
            assert!(constraints.contains(&constraint2));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_multiple_vars() {
    let input = "pub let f() -> Constraint = forall x in [1..3] { $V1(x) + $V2(x) === 10 };";

    let vars = HashMap::from([
        ("V1".to_string(), vec![ExprType::Int]),
        ("V2".to_string(), vec![ExprType::Int]),
    ]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
            let constraints = strip_origins(&constraints);

            let constraint1 = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![ExprValue::Int(1)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V2".into(),
                params: vec![ExprValue::Int(1)],
            })))
            .eq(&LinExpr::constant(10.));
            assert!(constraints.contains(&constraint1));

            let constraint2 = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "V1".into(),
                params: vec![ExprValue::Int(2)],
            })) + LinExpr::var(IlpVar::Base(ExternVar {
                name: "V2".into(),
                params: vec![ExprValue::Int(2)],
            })))
            .eq(&LinExpr::constant(10.));
            assert!(constraints.contains(&constraint2));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_with_param() {
    let input = "pub let f(value: Int) -> Constraint = forall x in [1..3] { $V(x) === value };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 2);
            let constraints = strip_origins(&constraints);

            // All constraints should be === 42
            let constraint1 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(1)],
            }))
            .eq(&LinExpr::constant(42.));
            assert!(constraints.contains(&constraint1));

            let constraint2 = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(2)],
            }))
            .eq(&LinExpr::constant(42.));
            assert!(constraints.contains(&constraint2));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_mixed_types() {
    let input = "pub let f() -> Constraint = forall x in [1..3] { $V(x) === 1 and $V(x) <== 10 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Each iteration produces 2 constraints, 2 iterations = 4 total
            assert_eq!(constraints.len(), 4);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_constraint_nested() {
    let input =
        "pub let f() -> Constraint = forall x in [1..2] { forall y in [1..2] { $V(x, y) === 1 } };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int, ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // 1 iteration * 1 iteration = 1 constraint
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }
}

// ========== Combined Quantifier Tests ==========

#[test]
fn sum_inside_forall() {
    let input =
        "pub let f() -> Constraint = forall x in [1..3] { sum y in [1..3] { $V(x, y) } === 10 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int, ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // 2 constraints (one for each x value)
            assert_eq!(constraints.len(), 2);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn forall_inside_sum() {
    let input = "pub let f() -> Int = sum x in [1..3] { if forall y in [1..3] { y > 0 } { x } else { 0 } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // forall is always true, so sum is 1 + 2 = 3
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn quantifiers_with_collection_ops() {
    let input = "pub let f() -> Int = sum x in ([1, 2, 3] + [4, 5]) { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn forall_with_collection_ops() {
    let input = "pub let f() -> Bool = forall x in ([1..5] + [3..7]) { x >= 3 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    // Intersection is [1..7], 1 and 2 are less than 3
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn quantifiers_with_if() {
    let input = "pub let f(x: Int) -> Int = if x > 0 { sum y in [1..4] { y } } else { 0 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(6));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(-5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Int(0));
}
