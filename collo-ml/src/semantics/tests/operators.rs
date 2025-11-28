use super::*;

// ========== Arithmetic Operators ==========

#[test]
fn addition_int() {
    let input = "pub let f(x: Int, y: Int) -> Int = x + y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Addition should work: {:?}", errors);
}

#[test]
fn addition_produces_linexpr() {
    let input = "pub let f(x: Int, y: Int) -> LinExpr = x + y;";
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
    let input = "pub let f(x: Int) -> LinExpr = -x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Addition should produce LinExpr: {:?}",
        errors
    );
}

#[test]
fn subtraction() {
    let input = "pub let f(x: Int, y: Int) -> LinExpr = x - y;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Subtraction should work: {:?}", errors);
}

#[test]
fn multiplication() {
    let input = "pub let f(x: Int, y: Int) -> LinExpr = x * y;";
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
    let vars = var_with_args("V", vec![ExprType::Int]);
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
    let input = "pub let f(a: Int, b: Int, c: Int) -> LinExpr = a + b - c * 2;";
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
    let vars = var_with_args("V", vec![ExprType::Int]);
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
    let input = "pub let f(xs: [Int], ys: [Int]) -> [Int] = xs union ys;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Collection union should work: {:?}",
        errors
    );
}

#[test]
fn collection_difference() {
    let input = "pub let f(xs: [Int], ys: [Int]) -> [Int] = xs \\ ys;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Collection difference should work: {:?}",
        errors
    );
}

#[test]
fn collection_ops_must_have_same_element_type() {
    let input = "pub let f(xs: [Int], ys: [Bool]) -> [Int] = xs union ys;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Collection ops require compatible types"
    );
}

#[test]
fn collection_ops_unify_int_linexpr() {
    let input = "pub let f(xs: [Int], ys: [LinExpr]) -> [LinExpr] = xs union ys;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Union should unify [Int] and [LinExpr]: {:?}",
        errors
    );
}

#[test]
fn chained_collection_operations() {
    let input = "pub let f(a: [Int], b: [Int], c: [Int]) -> [Int] = a union b \\ c;";
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
fn in_operator_with_coercion() {
    let input = "pub let f(x: Int, xs: [LinExpr]) -> Bool = x in xs;";
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
