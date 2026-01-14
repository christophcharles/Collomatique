use super::*;

// ========== Basic Function Calls ==========

#[test]
fn simple_fn_call() {
    let input = r#"
    let g() -> Int = 42;
    pub let f() -> Int = g();
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn fn_call_with_single_param() {
    let input = r#"
    let double(x: Int) -> Int = x * 2;
    pub let f() -> Int = double(21);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn fn_call_with_multiple_params() {
    let input = r#"
    let add(x: Int, y: Int) -> Int = x + y;
    pub let f() -> Int = add(10, 32);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn fn_call_passing_param_through() {
    let input = r#"
    let identity(x: Int) -> Int = x;
    pub let f(y: Int) -> Int = identity(y);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(100)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(100));
}

#[test]
fn fn_call_should_shadow_params() {
    let input = r#"
    let g(x: Int) -> Int = x;
    pub let f(x: Int) -> Int = g(43);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(42)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(43));
}

#[test]
fn fn_multi_call() {
    let input = r#"
    let g() -> Int = 0;
    let h(x: Int) -> Int = x;
    pub let f() -> [Int] = [g(), h(42)];
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([ExprValue::Int(0), ExprValue::Int(42)]))
    );
}

// ========== Functions with Different Parameter Types ==========

#[test]
fn fn_call_with_bool_param() {
    let input = r#"
    let negate(b: Bool) -> Bool = not b;
    pub let f() -> Bool = negate(true);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(false));
}

#[test]
fn fn_call_with_list_param() {
    let input = r#"
    let sum_list(xs: [Int]) -> Int = sum x in xs { x };
    pub let f() -> Int = sum_list([1, 2, 3]);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn fn_call_with_mixed_param_types() {
    let input = r#"
    let conditional_add(use_it: Bool, x: Int, y: Int) -> Int = if use_it { x + y } else { 0 };
    pub let f() -> Int = conditional_add(true, 10, 32);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn fn_call_with_linexpr_param() {
    let input = r#"
    let scale(expr: LinExpr, factor: Int) -> LinExpr = factor * expr;
    pub let f() -> LinExpr = scale($V(), 5);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected =
                5 * LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Functions with Different Return Types ==========

#[test]
fn fn_returning_bool() {
    let input = r#"
    let is_positive(x: Int) -> Bool = x > 0;
    pub let f() -> Bool = is_positive(5);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn fn_returning_list() {
    let input = r#"
    let make_range(n: Int) -> [Int] = [1..n];
    pub let f() -> [Int] = make_range(5);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(1),
            ExprValue::Int(2),
            ExprValue::Int(3),
            ExprValue::Int(4),
        ]))
    );
}

#[test]
fn fn_returning_linexpr() {
    let input = r#"
    let get_var(x: Int) -> LinExpr = $V(x);
    pub let f() -> LinExpr = get_var(10);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            assert_eq!(
                lin_expr,
                LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                    "V".into(),
                    vec![ExprValue::Int(10)]
                )))
            );
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn fn_returning_constraint() {
    let input = r#"
    let make_constraint(x: Int) -> Constraint = $V(x) === 1;
    pub let f() -> Constraint = make_constraint(5);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            assert_eq!(constraints.len(), 1);
        }
        _ => panic!("Expected Constraint"),
    }
}

#[test]
fn fn_returning_list_of_linexpr() {
    let input = r#"
    let get_vars(xs: [Int]) -> [LinExpr] = [$V(x) for x in xs];
    pub let f() -> [LinExpr] = get_vars([1, 2, 3]);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

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

// ========== Return Type Coercion ==========

#[test]
fn fn_call_int_to_linexpr_conversion() {
    let input = r#"
    let get_int() -> Int = 42;
    pub let f() -> LinExpr = LinExpr(get_int());
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
fn fn_call_int_coerced_in_addition() {
    let input = r#"
    let get_int() -> Int = 10;
    pub let f() -> LinExpr = get_int() + $V();
    "#;

    let vars = HashMap::from([("V".to_string(), vec![])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::constant(10.)
                + LinExpr::var(IlpVar::Base(ExternVar::new_no_env("V".into(), vec![])));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn fn_call_int_coerced_in_constraint() {
    let input = r#"
    let get_value() -> Int = 5;
    pub let f() -> Constraint = get_value() === 10;
    "#;

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

// ========== Nested Function Calls ==========

#[test]
fn nested_fn_call_two_levels() {
    let input = r#"
    let inner(x: Int) -> Int = x * 2;
    let outer(y: Int) -> Int = inner(y) + 1;
    pub let f() -> Int = outer(5);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // outer(5) = inner(5) + 1 = (5*2) + 1 = 11
    assert_eq!(result, ExprValue::Int(11));
}

#[test]
fn nested_fn_call_three_levels() {
    let input = r#"
    let level1(x: Int) -> Int = x + 1;
    let level2(x: Int) -> Int = level1(x) * 2;
    let level3(x: Int) -> Int = level2(x) + 10;
    pub let f() -> Int = level3(5);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // level3(5) = level2(5) + 10 = (level1(5) * 2) + 10 = ((5+1) * 2) + 10 = 22
    assert_eq!(result, ExprValue::Int(22));
}

#[test]
fn fn_call_as_argument() {
    let input = r#"
    let double(x: Int) -> Int = x * 2;
    let add(a: Int, b: Int) -> Int = a + b;
    pub let f() -> Int = add(double(3), double(4));
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // add(double(3), double(4)) = add(6, 8) = 14
    assert_eq!(result, ExprValue::Int(14));
}

#[test]
fn fn_call_nested_in_expression() {
    let input = r#"
    let get_value() -> Int = 10;
    pub let f() -> Int = get_value() * 2 + get_value();
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // 10 * 2 + 10 = 30
    assert_eq!(result, ExprValue::Int(30));
}

// ========== Functions with Complex Bodies ==========

#[test]
fn fn_with_if_expression() {
    let input = r#"
    let abs(x: Int) -> Int = if x >= 0 { x } else { -x };
    pub let f() -> Int = abs(-5);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(5));
}

#[test]
fn fn_with_sum() {
    let input = r#"
    let sum_range(n: Int) -> Int = sum x in [1..n] { x };
    pub let f() -> Int = sum_range(4);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // sum of 1, 2, 3 = 6
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn fn_with_forall() {
    let input = r#"
    let all_positive(xs: [Int]) -> Bool = forall x in xs { x > 0 };
    pub let f() -> Bool = all_positive([1, 2, 3]);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn fn_with_list_comprehension() {
    let input = r#"
    let squares(xs: [Int]) -> [Int] = [x * x for x in xs];
    pub let f() -> [Int] = squares([1, 2, 3]);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(1),
            ExprValue::Int(4),
            ExprValue::Int(9)
        ]))
    );
}

#[test]
fn fn_with_collection_operations() {
    let input = r#"
    let filter_evens(xs: [Int]) -> [Int] = [x for x in xs where x % 2 == 0];
    let combine(xs: [Int], ys: [Int]) -> [Int] = filter_evens(xs) + filter_evens(ys);
    pub let f() -> [Int] = combine([1, 2, 3], [4, 5, 6]);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(2),
            ExprValue::Int(4),
            ExprValue::Int(6)
        ]))
    );
}

// ========== Functions Using Variables ==========

#[test]
fn fn_using_base_var() {
    let input = r#"
    let make_linexpr(x: Int) -> LinExpr = $V(x) + 5;
    pub let f() -> LinExpr = make_linexpr(10);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Base(ExternVar::new_no_env(
                "V".into(),
                vec![ExprValue::Int(10)],
            ))) + LinExpr::constant(5.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn fn_using_reified_var() {
    let input = r#"
    let constraint_gen(x: Int) -> Constraint = $V(x) === 1;
    reify constraint_gen as $MyVar;
    let use_var(x: Int) -> LinExpr = $MyVar(x) + 10;
    pub let f() -> LinExpr = use_var(5);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar".into(),
                None,
                vec![ExprValue::Int(5)],
            ))) + LinExpr::constant(10.);
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

#[test]
fn fn_calling_fn_with_reified_var() {
    let input = r#"
    let constraint_gen(x: Int) -> Constraint = $V(x) === 1;
    reify constraint_gen as $MyVar;
    let helper(x: Int) -> LinExpr = $MyVar(x);
    let user(x: Int) -> LinExpr = helper(x) * 2;
    pub let f() -> LinExpr = user(10);
    "#;

    let vars = HashMap::from([("V".to_string(), vec![ExprType::simple(SimpleType::Int)])]);

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(lin_expr) => {
            let expected = 2 * LinExpr::var(IlpVar::Script(ScriptVar::new_no_env(
                "main".to_string(),
                "MyVar".into(),
                None,
                vec![ExprValue::Int(10)],
            )));
            assert_eq!(lin_expr, expected);
        }
        _ => panic!("Expected LinExpr"),
    }
}

// ========== Helper Function Patterns ==========

#[test]
fn helper_fn_for_repeated_logic() {
    let input = r#"
    let is_even(x: Int) -> Bool = x % 2 == 0;
    let count_evens(xs: [Int]) -> Int = sum x in xs where is_even(x) { 1 };
    pub let f() -> Int = count_evens([1, 2, 3, 4, 5, 6]);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // 2, 4, 6 are even = 3
    assert_eq!(result, ExprValue::Int(3));
}

#[test]
fn helper_fn_for_transformation() {
    let input = r#"
    let transform(x: Int) -> Int = x * 2 + 1;
    let apply_to_list(xs: [Int]) -> [Int] = [transform(x) for x in xs];
    pub let f() -> [Int] = apply_to_list([1, 2, 3]);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(3),
            ExprValue::Int(5),
            ExprValue::Int(7)
        ]))
    );
}

#[test]
fn multiple_helper_fns() {
    let input = r#"
    let square(x: Int) -> Int = x * x;
    let double(x: Int) -> Int = x * 2;
    let process(x: Int) -> Int = square(x) + double(x);
    pub let f() -> Int = process(5);
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // square(5) + double(5) = 25 + 10 = 35
    assert_eq!(result, ExprValue::Int(35));
}

// ========== Private vs Public Functions ==========

#[test]
fn private_fn_cannot_be_called_externally() {
    let input = r#"
    let private_helper() -> Int = 42;
    pub let f() -> Int = private_helper();
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Can call public function f
    let result_f = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result_f, ExprValue::Int(42));

    // Cannot call private function directly
    let result_private = checked_ast.quick_eval_fn("main", "private_helper", vec![]);
    assert!(result_private.is_err());
}

#[test]
fn public_fn_can_be_called() {
    let input = r#"
    pub let g() -> Int = 100;
    pub let f() -> Int = g();
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    // Both public functions can be called
    let result_f = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    assert_eq!(result_f, ExprValue::Int(100));

    let result_g = checked_ast
        .quick_eval_fn("main", "g", vec![])
        .expect("Should evaluate");
    assert_eq!(result_g, ExprValue::Int(100));
}

#[test]
fn private_fn_calls_private_fn() {
    let input = r#"
    let helper1() -> Int = 10;
    let helper2() -> Int = helper1() * 2;
    pub let f() -> Int = helper2() + 5;
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");
    // helper2() + 5 = (helper1() * 2) + 5 = (10 * 2) + 5 = 25
    assert_eq!(result, ExprValue::Int(25));
}

// ========== Functions in Different Contexts ==========

#[test]
fn fn_call_in_if_condition() {
    let input = r#"
    let check(x: Int) -> Bool = x > 10;
    pub let f(x: Int) -> Int = if check(x) { 100 } else { 0 };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(15)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(100));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Int(0));
}

#[test]
fn fn_call_in_quantifier() {
    let input = r#"
    let is_valid(x: Int) -> Bool = x > 0 and x < 10;
    pub let f(xs: [Int]) -> Bool = forall x in xs { is_valid(x) };
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let all_valid = ExprValue::List(Vec::from([
        ExprValue::Int(1),
        ExprValue::Int(5),
        ExprValue::Int(9),
    ]));
    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![all_valid])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Bool(true));

    let has_invalid = ExprValue::List(Vec::from([ExprValue::Int(1), ExprValue::Int(15)]));
    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![has_invalid])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(false));
}

#[test]
fn fn_call_in_list_comprehension() {
    let input = r#"
    let process(x: Int) -> Int = x * x;
    pub let f(xs: [Int]) -> [Int] = [process(x) for x in xs];
    "#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");

    let list = ExprValue::List(Vec::from([
        ExprValue::Int(2),
        ExprValue::Int(3),
        ExprValue::Int(4),
    ]));
    let result = checked_ast
        .quick_eval_fn("main", "f", vec![list])
        .expect("Should evaluate");
    assert_eq!(
        result,
        ExprValue::List(Vec::from([
            ExprValue::Int(4),
            ExprValue::Int(9),
            ExprValue::Int(16)
        ]))
    );
}
