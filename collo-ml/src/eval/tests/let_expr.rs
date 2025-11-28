use super::*;

#[test]
fn let_expr_simple_binding() {
    let input = "pub let f(x: Int) -> Int = let y = 5 { y + x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(3)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(8));
}

#[test]
fn let_expr_arithmetic_value() {
    let input = "pub let f(x: Int) -> Int = let doubled = x * 2 { doubled + 1 };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(21));
}

#[test]
fn let_expr_nested_bindings() {
    let input = "pub let f(x: Int) -> Int = let a = x * 2 { let b = a + 5 { b * 3 } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(4)])
        .expect("Should evaluate");

    // x=4 -> a=8 -> b=13 -> result=39
    assert_eq!(result, ExprValue::Int(39));
}

#[test]
fn let_expr_with_boolean_value() {
    let input = "pub let f(x: Int) -> Bool = let check = x > 5 { check };";

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
fn let_expr_with_list_value() {
    let input = "pub let f() -> [Int] = let items = [1, 2, 3] { items };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn let_expr_with_list_range() {
    let input = "pub let f(n: Int) -> [Int] = let range = [0..n] { range };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([
                ExprValue::Int(0),
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3),
                ExprValue::Int(4)
            ])
        )
    );
}

#[test]
fn let_expr_with_membership_test() {
    let input = "pub let f(x: Int, list: [Int]) -> Bool = let is_member = x in list { is_member };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
    );

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(2), list.clone()])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5), list])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn let_expr_with_if_body() {
    let input = "pub let f(x: Int) -> Int = let bound = 10 { if x > bound { 1 } else { 0 } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(15)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(1));

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Int(0));
}

#[test]
fn let_expr_with_sum_body() {
    let input = "pub let f(n: Int) -> Int = let upper = n { sum i in [0..upper] { i } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    // sum of 0+1+2+3+4 = 10
    assert_eq!(result, ExprValue::Int(10));
}

#[test]
fn let_expr_with_forall_generating_constraints() {
    let input = "pub let f(n: Int) -> Constraint = let bound = n * 2 { forall i in [0..bound] { $V(i) === 1 } };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(3)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // n=3, bound=6, so we should have 6 constraints (i=0..5)
            assert_eq!(constraints.len(), 6);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn let_expr_using_bound_var_multiple_times() {
    let input = "pub let f(x: Int) -> Int = let y = x * 2 { y + y + y };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    // y = 10, y + y + y = 30
    assert_eq!(result, ExprValue::Int(30));
}

#[test]
fn let_expr_with_constraint_value() {
    let input = "pub let f(x: Int) -> Constraint = let c = $V(x) === 1 { c };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(5)],
            }))
            .eq(&LinExpr::constant(1.));

            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn let_expr_with_linexpr_arithmetic() {
    let input = "pub let f(x: Int) -> Constraint = let expr = $V(x) + 5 { expr === 10 };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(3)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
            let constraints = strip_origins(&constraints);

            let constraint = (LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(3)],
            })) + LinExpr::constant(5.))
            .eq(&LinExpr::constant(10.));

            assert!(constraints.contains(&constraint));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn let_expr_with_constraint_combination() {
    let input = "pub let f(x: Int) -> Constraint = let c1 = $V(x) === 1 { let c2 = $V(x) <== 10 { c1 and c2 } };";

    let vars = HashMap::from([("V".to_string(), vec![ExprType::Int])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(7)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Should have 2 constraints: one for === and one for <==
            assert_eq!(constraints.len(), 2);
            let constraints = strip_origins(&constraints);

            let var = LinExpr::var(IlpVar::Base(ExternVar {
                name: "V".into(),
                params: vec![ExprValue::Int(7)],
            }));

            let constraint_eq = var.clone().eq(&LinExpr::constant(1.));
            let constraint_le = var.leq(&LinExpr::constant(10.));

            assert!(constraints.contains(&constraint_eq));
            assert!(constraints.contains(&constraint_le));
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn let_expr_with_list_comprehension() {
    let input = "pub let f(n: Int) -> [Int] = let bound = n { [i * 2 for i in [0..bound]] };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(4)])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([
                ExprValue::Int(0),
                ExprValue::Int(2),
                ExprValue::Int(4),
                ExprValue::Int(6)
            ])
        )
    );
}

#[test]
fn let_expr_with_cardinality() {
    let input = "pub let f(items: [Int]) -> Int = let list = items { |list| };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn let_expr_with_collection_union() {
    let input = "pub let f(a: [Int], b: [Int]) -> [Int] = let combined = a + b { combined };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list_a = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2)]),
    );
    let list_b = ExprValue::List(
        ExprType::Int,
        BTreeSet::from([ExprValue::Int(2), ExprValue::Int(3)]),
    );

    let result = checked_ast
        .quick_eval_fn("f", vec![list_a, list_b])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );
}

#[test]
fn let_expr_shadowing_parameter() {
    let input = "pub let f(x: Int) -> Int = let x = 10 { x };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(999)])
        .expect("Should evaluate");

    // Should use the let-bound value, not the parameter
    assert_eq!(result, ExprValue::Int(10));
}

#[test]
fn let_expr_shadowing_outer_let() {
    let input = "pub let f() -> Int = let x = 5 { let x = 10 { x } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    // Should use the inner let-bound value
    assert_eq!(result, ExprValue::Int(10));
}

#[test]
fn let_expr_complex_nested_computation() {
    let input = r#"
        pub let f(x: Int) -> Int = 
            let a = x * 2 {
                let b = a + 5 {
                    let c = b * 3 {
                        if c > 100 {
                            a + b
                        } else {
                            c
                        }
                    }
                }
            };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // x=10 -> a=20 -> b=25 -> c=75 -> c <= 100 so return c
    let result1 = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(75));

    // x=20 -> a=40 -> b=45 -> c=135 -> c > 100 so return a+b=85
    let result2 = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(20)])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(85));
}

#[test]
fn let_expr_with_function_call() {
    let input = r#"
        let helper(x: Int) -> Int = x * 3;
        pub let f(n: Int) -> Int = let result = helper(n) { result + 1 };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(4)])
        .expect("Should evaluate");

    // helper(4) = 12, result + 1 = 13
    assert_eq!(result, ExprValue::Int(13));
}

#[test]
fn let_expr_returning_list_from_if() {
    let input = "pub let f(x: Int) -> [Int] = let threshold = 5 { if x > threshold { [1, 2, 3] } else { [4, 5] } };";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(
        result_true,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)])
        )
    );

    let result_false = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(3)])
        .expect("Should evaluate");
    assert_eq!(
        result_false,
        ExprValue::List(
            ExprType::Int,
            BTreeSet::from([ExprValue::Int(4), ExprValue::Int(5)])
        )
    );
}
