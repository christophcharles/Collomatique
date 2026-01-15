use super::*;

// =============================================================================
// OPTION TYPE EVALUATION - ?Type
// =============================================================================

#[test]
fn option_type_with_value() {
    let input = "pub let f() -> ?Int = 42;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn option_type_with_none() {
    let input = "pub let f() -> ?Int = none;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn option_bool_with_value() {
    let input = "pub let f() -> ?Bool = true;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn option_list_with_value() {
    let input = "pub let f() -> ?[Int] = [1, 2, 3];";
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
            assert_eq!(list[1], ExprValue::Int(2));
            assert_eq!(list[2], ExprValue::Int(3));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn option_list_with_none() {
    let input = "pub let f() -> ?[Int] = none;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn list_of_option_values() {
    // Must explicitly cast since Int and None don't unify
    let input = "pub let f() -> [?Int] = [1 as ?Int, none, 3 as ?Int];";
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
            assert_eq!(list[1], ExprValue::None);
            assert_eq!(list[2], ExprValue::Int(3));
        }
        _ => panic!("Expected List"),
    }
}

// =============================================================================
// SUM TYPE EVALUATION - Type1 | Type2
// =============================================================================

#[test]
fn sum_type_returns_first_variant() {
    let input = "pub let f() -> Int | Bool = 42;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn sum_type_returns_second_variant() {
    let input = "pub let f() -> Int | Bool = true;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn sum_type_with_none_returns_none() {
    let input = "pub let f() -> None | Int | Bool = none;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn sum_type_with_none_returns_int() {
    let input = "pub let f() -> None | Int | Bool = 42;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn sum_type_three_variants_returns_middle() {
    let input = "pub let f() -> Int | Bool | LinExpr = true;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn list_of_sum_type_homogeneous() {
    // All elements cast to same type in sum
    let input = "pub let f() -> [Int | Bool] = [1 as Int | Bool, 2 as Int | Bool];";
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

#[test]
fn list_of_sum_type_mixed() {
    // Elements with different types in the sum
    let input =
        "pub let f() -> [Int | Bool] = [1 as Int | Bool, true as Int | Bool, 2 as Int | Bool];";
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
            assert_eq!(list[1], ExprValue::Bool(true));
            assert_eq!(list[2], ExprValue::Int(2));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn sum_of_list_types_returns_first() {
    let input = "pub let f() -> [Int] | [Bool] = [1, 2];";
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

#[test]
fn sum_of_list_types_returns_second() {
    let input = "pub let f() -> [Int] | [Bool] = [true, false];";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 2);
            assert_eq!(list[0], ExprValue::Bool(true));
            assert_eq!(list[1], ExprValue::Bool(false));
        }
        _ => panic!("Expected List"),
    }
}

// =============================================================================
// EXPLICIT TYPE CASTS WITH SUM TYPES
// =============================================================================

#[test]
fn explicit_cast_to_option_type() {
    let input = "pub let f() -> ?Int = 42 as ?Int;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn explicit_cast_to_sum_type() {
    let input = "pub let f() -> Int | Bool = 42 as Int | Bool;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn conversion_int_to_linexpr_in_sum() {
    let input = "pub let f() -> LinExpr | Bool = LinExpr(5);";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

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
fn explicit_cast_none_to_option() {
    let input = "pub let f() -> ?Int = none as ?Int;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn explicit_cast_in_list_of_sum() {
    let input = "pub let f() -> [Int | Bool] = [1 as Int | Bool, true as Int | Bool];";
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
            assert_eq!(list[1], ExprValue::Bool(true));
        }
        _ => panic!("Expected List"),
    }
}

// =============================================================================
// COERCION WITH SUM TYPES
// =============================================================================

#[test]
fn implicit_coercion_to_sum_type() {
    // Int coerces to Int | Bool since Int appears in the sum
    let input = "pub let f() -> Int | Bool = 42;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn implicit_coercion_to_option_type() {
    // Int coerces to ?Int (None | Int) since Int appears
    let input = "pub let f() -> ?Int = 42;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn implicit_coercion_none_to_option() {
    // None coerces to ?Int (None | Int) since None appears
    let input = "pub let f() -> ?Int = none;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}

#[test]
fn implicit_coercion_empty_list_to_option_list() {
    // [] coerces to ?[Int] when only one list type
    let input = "pub let f() -> ?[Int] = [];";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 0);
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn implicit_coercion_empty_list_to_sum_with_one_list() {
    // [] coerces to [Int] | Bool when only one list type in sum
    let input = "pub let f() -> [Int] | Bool = [];";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(list) => {
            assert_eq!(list.len(), 0);
        }
        _ => panic!("Expected List"),
    }
}

// =============================================================================
// OPTION AND SUM TYPES WITH VARIABLES
// =============================================================================

#[test]
fn option_linexpr_from_variable() {
    let input = "pub let f() -> ?LinExpr = $V();";
    let vars = HashMap::from([("V".to_string(), vec![])]);
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

#[test]
fn sum_type_with_linexpr_from_variable() {
    let input = "pub let f() -> LinExpr | Int = $V();";
    let vars = HashMap::from([("V".to_string(), vec![])]);
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

#[test]
fn option_constraint_from_comparison() {
    let input = "pub let f() -> ?Constraint = 5 === 10;";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(_) => assert!(true),
        _ => panic!("Expected Constraint"),
    }
}

// =============================================================================
// COMPLEX NESTED TYPES
// =============================================================================

#[test]
fn option_of_list_of_sum_evaluation() {
    let input = "pub let f() -> ?[Int | Bool] = [1 as Int | Bool, true as Int | Bool];";
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
            assert_eq!(list[1], ExprValue::Bool(true));
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn nested_list_with_sum_types() {
    let input = "pub let f() -> [[Int | Bool]] = [[1 as Int | Bool, true as Int | Bool]];";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::List(outer_list) => {
            assert_eq!(outer_list.len(), 1);
            match &outer_list[0] {
                ExprValue::List(inner_list) => {
                    assert_eq!(inner_list.len(), 2);
                    assert_eq!(inner_list[0], ExprValue::Int(1));
                    assert_eq!(inner_list[1], ExprValue::Bool(true));
                }
                _ => panic!("Expected inner List"),
            }
        }
        _ => panic!("Expected outer List"),
    }
}

// =============================================================================
// CONDITIONAL EXPRESSIONS WITH SUM TYPES
// =============================================================================

#[test]
fn if_expr_returns_different_sum_variants() {
    let input = "pub let f(flag: Bool) -> Int | Bool = if flag { 42 as Int | Bool } else { true };";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(42));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::Bool(true));
}

#[test]
fn if_expr_returns_value_or_none() {
    let input = "pub let f(flag: Bool) -> ?Int = if flag { 42 as ?Int } else { none };";
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(result_true, ExprValue::Int(42));

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(result_false, ExprValue::None);
}

// =============================================================================
// REALISTIC EXAMPLES
// =============================================================================

#[test]
fn realistic_optional_lookup() {
    let input = r#"
        let lookup(id: Int) -> ?Int = if id == 1 { 100 as ?Int } else { none };
        pub let f() -> ?Int = lookup(1);
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
fn realistic_optional_lookup_returns_none() {
    let input = r#"
        let lookup(id: Int) -> ?Int = if id == 1 { 100 as ?Int } else { none };
        pub let f() -> ?Int = lookup(2);
    "#;
    let vars = HashMap::new();
    let checked_ast =
        CheckedAST::new(&BTreeMap::from([("main", input)]), vars).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::None);
}
