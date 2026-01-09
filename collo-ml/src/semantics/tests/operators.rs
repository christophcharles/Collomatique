use super::*;

// ========== Narrowing Cast Operators: cast? and cast! ==========

#[test]
fn cast_fallible_valid_narrowing() {
    let input = "pub let f(x: Int | Bool) -> ?Int = x cast? Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast? with valid subtype should work: {:?}",
        errors
    );
}

#[test]
fn cast_fallible_returns_optional_type() {
    let input = "pub let f(x: Int | Bool) -> Int = x cast? Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    // Return type should be ?Int, not Int
    assert!(!errors.is_empty(), "cast? should return optional type");
}

#[test]
fn cast_fallible_invalid_not_subtype() {
    // Trying to cast Int to String should fail because String is not a subtype of Int
    let input = "pub let f(x: Int) -> ?String = x cast? String;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "cast? should not allow casting to non-subtype"
    );
}

#[test]
fn cast_fallible_same_type() {
    let input = "pub let f(x: Int) -> ?Int = x cast? Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast? with same type should work: {:?}",
        errors
    );
}

#[test]
fn cast_fallible_with_none() {
    let input = "pub let f(x: ?Int) -> ?Int = x cast? Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast? from optional type should work: {:?}",
        errors
    );
}

#[test]
fn cast_fallible_from_optional() {
    // Casting an optional type to get the underlying value
    let input = "pub let f(x: ?Int) -> ?Int = x cast? Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast? from optional should work: {:?}",
        errors
    );
}

#[test]
fn cast_fallible_unrelated_types() {
    // String is not a subtype of Int | Bool
    let input = "pub let f(x: Int | Bool) -> ?String = x cast? String;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "cast? to unrelated type should fail");
}

#[test]
fn cast_panic_valid_narrowing() {
    let input = "pub let f(x: Int | Bool) -> Int = x cast! Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast! with valid subtype should work: {:?}",
        errors
    );
}

#[test]
fn cast_panic_returns_exact_type() {
    // cast! returns the target type directly (not optional)
    let input = "pub let f(x: Int | Bool) -> ?Int = x cast! Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    // This should work since Int is subtype of ?Int
    assert!(
        errors.is_empty(),
        "cast! result should fit in optional: {:?}",
        errors
    );
}

#[test]
fn cast_panic_invalid_not_subtype() {
    // Trying to cast Int to String should fail because String is not a subtype of Int
    let input = "pub let f(x: Int) -> String = x cast! String;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "cast! should not allow casting to non-subtype"
    );
}

#[test]
fn cast_panic_same_type() {
    let input = "pub let f(x: Int) -> Int = x cast! Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast! with same type should work: {:?}",
        errors
    );
}

#[test]
fn cast_panic_with_none() {
    let input = "pub let f(x: ?Int) -> Int = x cast! Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast! from optional type should work: {:?}",
        errors
    );
}

#[test]
fn cast_panic_unrelated_types() {
    let input = "pub let f(x: Int | Bool) -> String = x cast! String;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "cast! to unrelated type should fail");
}

#[test]
fn cast_fallible_list_type() {
    let input = "pub let f(x: [Int] | [Bool]) -> ?[Int] = x cast? [Int];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast? with list type should work: {:?}",
        errors
    );
}

#[test]
fn cast_panic_list_type() {
    let input = "pub let f(x: [Int] | [Bool]) -> [Bool] = x cast! [Bool];";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast! with list type should work: {:?}",
        errors
    );
}

#[test]
fn cast_fallible_tuple_type() {
    let input = "pub let f(x: (Int, Bool) | (Bool, Int)) -> ?(Int, Bool) = x cast? (Int, Bool);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast? with tuple type should work: {:?}",
        errors
    );
}

#[test]
fn cast_panic_tuple_type() {
    let input = "pub let f(x: (Int, Bool) | (Bool, Int)) -> (Bool, Int) = x cast! (Bool, Int);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast! with tuple type should work: {:?}",
        errors
    );
}

#[test]
fn cast_in_expression_context() {
    let input = "pub let f(x: Int | Bool) -> Int = (x cast! Int) + 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast! in expression should work: {:?}",
        errors
    );
}

#[test]
fn cast_chained_with_as() {
    // Cast to narrow, then widen with as
    let input = "pub let f(x: Int | Bool | String) -> ?Int = (x cast? Int) as ?Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast? followed by as should work: {:?}",
        errors
    );
}

#[test]
fn cast_fallible_to_optional_type() {
    // Target type ?Int already contains None, result is still ?Int
    let input = "pub let f(x: Int | Bool | None) -> ?Int = x cast? ?Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast? to optional type should work: {:?}",
        errors
    );
}

#[test]
fn cast_panic_to_optional_type() {
    let input = "pub let f(x: Int | Bool | None) -> ?Int = x cast! ?Int;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "cast! to optional type should work: {:?}",
        errors
    );
}

// ========== Null Coalescing Operator ==========

#[test]
fn null_coalesce_basic() {
    let input = "pub let f(x: ?Int) -> Int = x ?? 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "?? with maybe type should work: {:?}",
        errors
    );
}

#[test]
fn null_coalesce_union_type() {
    let input = "pub let f(x: Int | Bool | None) -> Int | Bool = x ?? 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "?? with union containing None should work: {:?}",
        errors
    );
}

#[test]
fn null_coalesce_on_non_maybe_fails() {
    let input = "pub let f(x: Int) -> Int = x ?? 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "?? on non-maybe type should fail");
}

#[test]
fn null_coalesce_result_type_removes_none() {
    // Result should be Int, not ?Int
    let input = "pub let f(x: ?Int) -> ?Int = x ?? 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    // Int fits in ?Int, so this should work
    assert!(
        errors.is_empty(),
        "?? result should fit in optional: {:?}",
        errors
    );
}

#[test]
fn null_coalesce_result_type_unifies() {
    // x: ?Int, default: Bool -> result is Int | Bool
    let input = "pub let f(x: ?Int) -> Int | Bool = x ?? true;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "?? should unify lhs-None with rhs: {:?}",
        errors
    );
}

#[test]
fn null_coalesce_with_none_default() {
    // x ?? none is a no-op, result is still ?Int
    let input = "pub let f(x: ?Int) -> ?Int = x ?? none;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "?? with none default should work: {:?}",
        errors
    );
}

#[test]
fn null_coalesce_chained() {
    let input = "pub let f(x: ?Int, y: ?Int) -> Int = x ?? y ?? 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "chained ?? should work: {:?}", errors);
}

#[test]
fn null_coalesce_with_expression() {
    let input = "pub let f(x: ?Int) -> Int = (x ?? 0) + 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "?? in expression context should work: {:?}",
        errors
    );
}

#[test]
fn null_coalesce_precedence_with_or() {
    // ?? has lower precedence than or, so this parses as (x or y) ?? z
    let input = "pub let f(x: Bool, y: Bool, z: Bool) -> Bool = (x or y) as ?Bool ?? z;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "?? precedence with or should work: {:?}",
        errors
    );
}

// ========== Arithmetic Operators ==========

#[test]
fn addition_int() {
    let input = "pub let f(x: Int, y: Int) -> Int = x + y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Addition should work: {:?}", errors);
}

#[test]
fn addition_produces_linexpr() {
    let input = "pub let f(x: LinExpr, y: LinExpr) -> LinExpr = x + y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Addition should produce LinExpr: {:?}",
        errors
    );
}

#[test]
fn negation_int() {
    let input = "pub let f(x: Int) -> Int = -x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Addition should work: {:?}", errors);
}

#[test]
fn negation_produces_linexpr() {
    let input = "pub let f(x: LinExpr) -> LinExpr = -x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Addition should produce LinExpr: {:?}",
        errors
    );
}

#[test]
fn subtraction() {
    let input = "pub let f(x: Int, y: Int) -> Int = x - y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Subtraction should work: {:?}", errors);
}

#[test]
fn multiplication() {
    let input = "pub let f(x: Int, y: Int) -> Int = x * y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Multiplication should work: {:?}",
        errors
    );
}

#[test]
fn integer_division() {
    let input = "pub let f(x: Int, y: Int) -> Int = x // y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Integer division should work: {:?}",
        errors
    );
}

#[test]
fn modulo() {
    let input = "pub let f(x: Int, y: Int) -> Int = x % y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Modulo should work: {:?}", errors);
}

#[test]
fn arithmetic_with_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = $V(x) + 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Arithmetic with LinExpr should work: {:?}",
        errors
    );
}

#[test]
fn arithmetic_with_bool_should_fail() {
    let input = "pub let f(x: Bool) -> Int = x + 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Arithmetic with Bool should fail");
}

#[test]
fn chained_arithmetic() {
    let input = "pub let f(a: Int, b: Int, c: Int) -> Int = a + b - c * 2;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Chained arithmetic should work: {:?}",
        errors
    );
}

// ========== Comparison Operators ==========

#[test]
fn equality_comparison() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x == y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Equality comparison should work: {:?}",
        errors
    );
}

#[test]
fn inequality_comparison() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x != y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Inequality comparison should work: {:?}",
        errors
    );
}

#[test]
fn less_than() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x < y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Less than should work: {:?}", errors);
}

#[test]
fn less_equal() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x <= y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Less or equal should work: {:?}", errors);
}

#[test]
fn greater_than() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x > y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Greater than should work: {:?}", errors);
}

#[test]
fn greater_equal() {
    let input = "pub let f(x: Int, y: Int) -> Bool = x >= y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Greater or equal should work: {:?}",
        errors
    );
}

// ========== Constraint Operators ==========

#[test]
fn constraint_equality() {
    let input = "pub let f(x: Int) -> Constraint = x === 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Constraint equality should work: {:?}",
        errors
    );
}

#[test]
fn constraint_less_equal() {
    let input = "pub let f(x: Int) -> Constraint = x <== 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Constraint <== should work: {:?}",
        errors
    );
}

#[test]
fn constraint_greater_equal() {
    let input = "pub let f(x: Int) -> Constraint = x >== 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Constraint >== should work: {:?}",
        errors
    );
}

#[test]
fn constraint_with_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> Constraint = $V(x) === 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Constraint with LinExpr should work: {:?}",
        errors
    );
}

#[test]
fn constraint_with_arithmetic() {
    let input = "pub let f(x: Int, y: Int) -> Constraint = x + y === 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Constraint with arithmetic should work: {:?}",
        errors
    );
}

// ========== Logical Operators ==========

#[test]
fn logical_and_bool() {
    let input = "pub let f(a: Bool, b: Bool) -> Bool = a and b;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Logical AND with Bool should work: {:?}",
        errors
    );
}

#[test]
fn logical_and_constraint() {
    let input = "pub let f(x: Int) -> Constraint = (x === 0) and (x <== 10);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Logical AND with Constraint should work: {:?}",
        errors
    );
}

#[test]
fn logical_and_mixed_types_fails() {
    let input = "pub let f(x: Int) -> Constraint = true and (x === 0);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Cannot mix Bool and Constraint in AND");
}

#[test]
fn logical_or_bool() {
    let input = "pub let f(a: Bool, b: Bool) -> Bool = a or b;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Logical OR with Bool should work: {:?}",
        errors
    );
}

#[test]
fn logical_or_constraint() {
    let input = "pub let f(x: Int) -> Constraint = (x === 0) or (x === 10);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Logical OR with Constraint should work: {:?}",
        errors
    );
}

#[test]
fn logical_not_bool() {
    let input = "pub let f(a: Bool) -> Bool = not a;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Logical NOT with Bool should work: {:?}",
        errors
    );
}

#[test]
fn logical_not_constraint() {
    let input = "pub let f(x: Int) -> Constraint = not (x === 0);";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Logical NOT with Constraint should not work: {:?}",
        errors
    );
}

#[test]
fn chained_logical_operations() {
    let input = "pub let f(a: Bool, b: Bool, c: Bool) -> Bool = a and b or c;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Chained logical operations should work: {:?}",
        errors
    );
}

// ========== Collection Operators ==========

#[test]
fn collection_union() {
    let input = "pub let f(xs: [Int], ys: [Int]) -> [Int] = xs + ys;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Collection + should work: {:?}", errors);
}

#[test]
fn collection_difference() {
    let input = "pub let f(xs: [Int], ys: [Int]) -> [Int] = xs - ys;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Collection difference should work: {:?}",
        errors
    );
}

#[test]
fn collection_ops_must_have_same_element_type() {
    let input = "pub let f(xs: [Int], ys: [Bool]) -> [Int] = xs + ys;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Collection ops require compatible types"
    );
}

#[test]
fn collection_ops_unify_int_linexpr() {
    let input = "pub let f(xs: [Int], ys: [LinExpr]) -> [Int | LinExpr] = xs + ys;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Union should unify [Int] and [LinExpr]: {:?}",
        errors
    );
}

#[test]
fn chained_collection_operations() {
    let input = "pub let f(a: [Int], b: [Int], c: [Int]) -> [Int] = a + b - c;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Chained collection ops should work: {:?}",
        errors
    );
}

// ========== Membership Operator ==========

#[test]
fn in_operator_element_in_list() {
    let input = "pub let f(x: Int, xs: [Int]) -> Bool = x in xs;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "'in' operator should work: {:?}", errors);
}

#[test]
fn in_operator_type_mismatch() {
    let input = "pub let f(x: Bool, xs: [Int]) -> Bool = x in xs;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "'in' with wrong type should fail");
}

#[test]
fn in_operator_with_conversion() {
    let input = "pub let f(x: Int, xs: [LinExpr]) -> Bool = (LinExpr(x)) in xs;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "'in' should allow coercion: {:?}",
        errors
    );
}

// ========== Cardinality Operator ==========

#[test]
fn cardinality_of_list() {
    let input = "pub let f(xs: [Int]) -> Int = |xs|;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Cardinality should work: {:?}", errors);
}

#[test]
fn cardinality_of_non_list_fails() {
    let input = "pub let f(x: Int) -> Int = |x|;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Cardinality of non-list should fail");
}

#[test]
fn cardinality_in_expression() {
    let input = "pub let f(xs: [Int]) -> Int = |xs| + 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Cardinality in expression should work: {:?}",
        errors
    );
}

// ========== Operator Precedence and Associativity ==========

#[test]
fn arithmetic_precedence() {
    let input = "pub let f(a: Int, b: Int, c: Int) -> Int = a + b * c;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Arithmetic precedence should work: {:?}",
        errors
    );
}

#[test]
fn comparison_vs_logical() {
    let input = "pub let f(a: Int, b: Int, c: Bool) -> Bool = a > b and c;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Comparison vs logical precedence should work: {:?}",
        errors
    );
}

#[test]
fn parentheses_override_precedence() {
    let input = "pub let f(a: Int, b: Int, c: Int) -> Int = (a + b) * c;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Parentheses should work: {:?}", errors);
}

// ========== List Indexing: [i]? and [i]! ==========

#[test]
fn list_index_fallible_basic() {
    let input = "pub let f(xs: [Int]) -> ?Int = xs[0]?;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List indexing with [i]? should work: {:?}",
        errors
    );
}

#[test]
fn list_index_panic_basic() {
    let input = "pub let f(xs: [Int]) -> Int = xs[0]!;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List indexing with [i]! should work: {:?}",
        errors
    );
}

#[test]
fn list_index_fallible_returns_optional() {
    // [i]? should return optional type, not the element type
    let input = "pub let f(xs: [Int]) -> Int = xs[0]?;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "[i]? should return optional type, not Int"
    );
}

#[test]
fn list_index_panic_returns_non_optional_type() {
    // [i]! returns Int directly, which is a subtype of ?Int, so this should work
    let input = "pub let f(xs: [Int]) -> ?Int = xs[0]!;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "[i]! returning Int should be assignable to ?Int: {:?}",
        errors
    );
}

#[test]
fn list_index_panic_not_optional() {
    // [i]! returns Int, not ?Int - so it can be used directly without unwrapping
    let input = "pub let f(xs: [Int]) -> Int = xs[0]! + 1;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "[i]! should return non-optional type: {:?}",
        errors
    );
}

#[test]
fn list_index_on_non_list_fails() {
    let input = "pub let f(x: Int) -> ?Int = x[0]?;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "List indexing on non-list should fail");
}

#[test]
fn list_index_non_int_index_fails() {
    let input = "pub let f(xs: [Int]) -> ?Int = xs[true]?;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "List indexing with non-Int index should fail"
    );
}

#[test]
fn list_index_union_of_lists() {
    // ([Int] | [Bool])[i]! should return Int | Bool
    let input = "pub let f(xs: [Int] | [Bool]) -> Int | Bool = xs[0]!;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List indexing on union of lists should work: {:?}",
        errors
    );
}

#[test]
fn list_index_union_of_lists_fallible() {
    // ([Int] | [Bool])[i]? should return Int | Bool | None
    let input = "pub let f(xs: [Int] | [Bool]) -> Int | Bool | None = xs[0]?;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Fallible list indexing on union of lists should work: {:?}",
        errors
    );
}

#[test]
fn list_index_chained() {
    let input = "pub let f(matrix: [[Int]]) -> Int = matrix[0]![1]!;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Chained list indexing should work: {:?}",
        errors
    );
}

#[test]
fn list_index_with_field_access() {
    let types = HashMap::from([(
        "Student".to_string(),
        HashMap::from([(
            "scores".to_string(),
            ExprType::simple(SimpleType::List(SimpleType::Int.into())),
        )]),
    )]);
    let input = "pub let f(s: Student) -> ?Int = s.scores[0]?;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "List indexing after field access should work: {:?}",
        errors
    );
}

#[test]
fn list_index_with_expression() {
    let input = "pub let f(xs: [Int], i: Int) -> ?Int = xs[i + 1]?;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List indexing with expression index should work: {:?}",
        errors
    );
}

#[test]
fn list_index_nested_list_element_type() {
    // [[Int]][i]? should return ?[Int]
    let input = "pub let f(xs: [[Int]]) -> ?[Int] = xs[0]?;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List indexing on nested list should return correct type: {:?}",
        errors
    );
}

#[test]
fn list_index_on_literal() {
    let input = "pub let f() -> ?Int = [1, 2, 3][0]?;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "List indexing on literal should work: {:?}",
        errors
    );
}
