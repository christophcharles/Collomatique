use super::*;

// =============================================================================
// FUNCTIONS WITH OPTION TYPE PARAMETERS
// =============================================================================

#[test]
fn function_with_option_param_receives_value() {
    let input = r#"
        let process(x: ?Int) -> Int = 99;
        pub let f() -> Int = process(42);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

#[test]
fn function_with_option_param_receives_none() {
    let input = r#"
        let process(x: ?Int) -> Int = 99;
        pub let f() -> Int = process(none);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

#[test]
fn function_with_option_param_passes_through_value() {
    let input = r#"
        let identity(x: ?Int) -> ?Int = x;
        pub let f() -> ?Int = identity(42);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn function_with_option_param_passes_through_none() {
    let input = r#"
        let identity(x: ?Int) -> ?Int = x;
        pub let f() -> ?Int = identity(none);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn function_with_multiple_option_params() {
    let input = r#"
        let choose_first(x: ?Int, y: ?Int) -> ?Int = x;
        pub let f() -> ?Int = choose_first(42, none);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn function_option_list_param() {
    let input = r#"
        let get_length(xs: ?[Int]) -> Int = 10;
        pub let f() -> Int = get_length([1, 2, 3]);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

#[test]
fn function_option_list_param_receives_none() {
    let input = r#"
        let get_length(xs: ?[Int]) -> Int = 10;
        pub let f() -> Int = get_length(none);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

// =============================================================================
// FUNCTIONS WITH SUM TYPE PARAMETERS
// =============================================================================

#[test]
fn function_with_sum_param_receives_first_variant() {
    let input = r#"
        let process(x: Int | Bool) -> Int = 99;
        pub let f() -> Int = process(42);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

#[test]
fn function_with_sum_param_receives_second_variant() {
    let input = r#"
        let process(x: Int | Bool) -> Int = 99;
        pub let f() -> Int = process(true);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

#[test]
fn function_with_sum_param_passes_through_first() {
    let input = r#"
        let identity(x: Int | Bool) -> Int | Bool = x;
        pub let f() -> Int | Bool = identity(42);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn function_with_sum_param_passes_through_second() {
    let input = r#"
        let identity(x: Int | Bool) -> Int | Bool = x;
        pub let f() -> Int | Bool = identity(true);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn function_with_three_variant_sum() {
    let input = r#"
        let process(x: Int | Bool | LinExpr) -> Int = 99;
        pub let f() -> Int = process(true);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

#[test]
fn function_with_sum_including_none() {
    let input = r#"
        let process(x: None | Int | Bool) -> Int = 99;
        pub let f() -> Int = process(none);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

#[test]
fn function_list_of_sum_param() {
    let input = r#"
        let process(xs: [Int | Bool]) -> Int = 99;
        pub let f() -> Int = process([1 as Int | Bool, true as Int | Bool]);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

#[test]
fn function_sum_of_list_types_param() {
    let input = r#"
        let process(xs: [Int] | [Bool]) -> Int = 99;
        pub let f() -> Int = process([1, 2]);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(99));
}

// =============================================================================
// FUNCTIONS WITH OPTION RETURN TYPES
// =============================================================================

#[test]
fn function_option_return_returns_value() {
    let input = r#"
        let maybe_value(flag: Bool) -> ?Int = if flag { 42 as ?Int } else { none };
        pub let f() -> ?Int = maybe_value(true);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn function_option_return_returns_none() {
    let input = r#"
        let maybe_value(flag: Bool) -> ?Int = if flag { 42 as ?Int } else { none };
        pub let f() -> ?Int = maybe_value(false);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn function_option_list_return() {
    let input = r#"
        let maybe_list(flag: Bool) -> ?[Int] = if flag { [1, 2, 3] as ?[Int] } else { none };
        pub let f() -> ?[Int] = maybe_list(true);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 3);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn function_option_list_return_none() {
    let input = r#"
        let maybe_list(flag: Bool) -> ?[Int] = if flag { [1, 2, 3] as ?[Int] } else { none };
        pub let f() -> ?[Int] = maybe_list(false);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

// =============================================================================
// FUNCTIONS WITH SUM RETURN TYPES
// =============================================================================

#[test]
fn function_sum_return_returns_first() {
    let input = r#"
        let choose(flag: Bool) -> Int | Bool = if flag { 42 as Int | Bool } else { true };
        pub let f() -> Int | Bool = choose(true);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn function_sum_return_returns_second() {
    let input = r#"
        let choose(flag: Bool) -> Int | Bool = if flag { 42 as Int | Bool } else { true };
        pub let f() -> Int | Bool = choose(false);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn function_sum_return_with_none() {
    let input = r#"
        let get_value(x: Int) -> None | Int = if x == 0 { none as ?Int } else { x };
        pub let f() -> None | Int = get_value(0);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn function_sum_return_list_types() {
    let input = r#"
        let get_list(flag: Bool) -> [Int] | [Bool] = if flag { [1, 2] as [Int] | [Bool] } else { [true, false] };
        pub let f() -> [Int] | [Bool] = get_list(true);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 2);
            assert_eq!(list[0], ExprValue::Int(1));
            assert_eq!(list[1], ExprValue::Int(2));
        }
        _ => panic!("Expected List"),
    }
}

// =============================================================================
// CHAINED FUNCTION CALLS WITH SUM/OPTION TYPES
// =============================================================================

#[test]
fn chained_calls_with_option_types() {
    let input = r#"
        let first(x: Int) -> ?Int = if x > 0 { x as ?Int } else { none };
        let second(x: ?Int) -> ?Int = x;
        pub let f(x: Int) -> ?Int = second(first(x));
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(5));
}

#[test]
fn chained_calls_with_option_types_returns_none() {
    let input = r#"
        let first(x: Int) -> ?Int = if x > 0 { x as ?Int } else { none };
        let second(x: ?Int) -> ?Int = x;
        pub let f(x: Int) -> ?Int = second(first(x));
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(0)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn chained_calls_with_sum_types() {
    let input = r#"
        let first(x: Int) -> Int | Bool = if x > 0 { x as Int | Bool } else { true };
        let second(x: Int | Bool) -> Int | Bool = x;
        pub let f(x: Int) -> Int | Bool = second(first(x));
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(5));
}

// =============================================================================
// HIGHER-ORDER PATTERNS WITH SUM/OPTION TYPES
// =============================================================================

#[test]
fn function_returning_function_result_with_option() {
    let input = r#"
        let inner(x: Int) -> ?Int = if x > 0 { x as ?Int } else { none };
        let outer(x: Int) -> ?Int = inner(x);
        pub let f() -> ?Int = outer(5);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(5));
}

#[test]
fn function_with_mixed_option_and_regular_params() {
    let input = r#"
        let combine(x: ?Int, y: Int) -> Int = y;
        pub let f() -> Int = combine(42, 10);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

#[test]
fn function_with_mixed_sum_and_regular_params() {
    let input = r#"
        let combine(x: Int | Bool, y: Int) -> Int = y;
        pub let f() -> Int = combine(42, 10);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(10));
}

// =============================================================================
// COMPLEX NESTED FUNCTION SCENARIOS
// =============================================================================

#[test]
fn nested_option_list_through_functions() {
    let input = r#"
        let make_list() -> [?Int] = [1, none, 3];
        let wrap(xs: [?Int]) -> ?[?Int] = xs;
        pub let f() -> ?[?Int] = wrap(make_list());
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(outer_list) => {
            assert_eq!(outer_list.len(), 3);
            assert_eq!(outer_list[0], ExprValue::Int(1));
            assert_eq!(outer_list[1], ExprValue::None);
            assert_eq!(outer_list[2], ExprValue::Int(3));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn function_sum_type_with_list_variants() {
    let input = r#"
        let get_ints() -> [Int] = [1, 2, 3];
        let get_bools() -> [Bool] = [true, false];
        let choose(flag: Bool) -> [Int] | [Bool] = if flag { get_ints() as [Int] | [Bool] } else { get_bools() };
        pub let f() -> [Int] | [Bool] = choose(true);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 3);
            assert_eq!(list[0], ExprValue::Int(1));
        }
        _ => panic!("Expected List"),
    }
}

// =============================================================================
// REALISTIC USE CASES
// =============================================================================

#[test]
fn realistic_optional_database_lookup() {
    let input = r#"
        let find_by_id(id: Int) -> ?Int = if id == 1 { 100 } else { if id == 2 { 200 as ?Int } else { none } };
        let get_value(id: Int) -> ?Int = find_by_id(id);
        pub let f() -> ?Int = get_value(1);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(100));
}

#[test]
fn realistic_optional_database_lookup_not_found() {
    let input = r#"
        let find_by_id(id: Int) -> ?Int = if id == 1 { 100 } else { if id == 2 { 200 as ?Int } else { none } };
        let get_value(id: Int) -> ?Int = find_by_id(id);
        pub let f() -> ?Int = get_value(99);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn realistic_polymorphic_response() {
    let input = r#"
        let process(x: Int) -> Int | Bool = if x == 0 { false } else { if x == 1 { true as Int | Bool } else { x } };
        pub let f() -> Int | Bool = process(5);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(5));
}
