use super::*;

// =============================================================================
// SEMANTICALLY INVALID BUT GRAMMATICALLY VALID EXPRESSIONS
// =============================================================================
// These tests explicitly validate that the parser accepts expressions that
// are syntactically correct but semantically wrong. Type checking and
// semantic analysis should catch these errors later.
//
// Note: Int automatically coerces to LinExpr in this language.

#[test]
fn semantic_invalid_type_mismatches() {
    // Parser accepts these; semantics will reject them
    let cases = vec![
        // Operations on incompatible types (no coercion)
        "true + false",     // Bool + Bool
        "[1, 2] * [3, 4]",  // List * List
        "true union false", // Bool union Bool (not sets)
        // Wrong types in aggregations
        "sum x in 5 { x }",       // 5 is not a collection
        "forall x in true { x }", // true is not a collection
        // Type mismatches with variables
        "$V() union [1, 2]", // LinExpr union List
        "[1, 2] === [3, 4]", // List === List (not LinExpr)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (grammatically valid, semantically wrong): {:?}",
            case,
            result
        );
    }
}

#[test]
fn semantic_invalid_but_coercion_makes_valid() {
    // These look wrong but are actually CORRECT due to Int -> LinExpr coercion
    let cases = vec![
        "5 === 10",        // Int coerced to LinExpr
        "$V() + 5",        // 5 coerced to LinExpr
        "2 * $V() === 10", // 10 coerced to LinExpr
        "$V() == 5",       // 5 coerced to LinExpr (returns Bool)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (actually semantically valid with coercion): {:?}",
            case,
            result
        );
    }
}

#[test]
fn semantic_invalid_undefined_references() {
    // Parser doesn't check if identifiers are defined
    let cases = vec![
        "undefined_variable",
        "student.nonexistent_field",
        "undefined_function()",
        "$UndefinedVar(x)",
        "x in undefined_collection",
        "sum x in undefined { x }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (undefined reference): {:?}",
            case,
            result
        );
    }
}

#[test]
fn semantic_invalid_scope_violations() {
    // Parser doesn't check variable scope
    let cases = vec![
        "x + y",                  // x and y not in scope
        "$V(x)",                  // x not in scope
        "sum x in list { y }",    // y not in scope (x is)
        "forall x in @[X] { y }", // y not in scope (x is)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (scope violation): {:?}",
            case,
            result
        );
    }
}

#[test]
fn semantic_invalid_wrong_arity() {
    // Parser doesn't check function arity
    let cases = vec![
        "func()",              // Maybe expects arguments
        "func(a, b, c, d, e)", // Maybe expects fewer
        "$V()",                // Maybe expects arguments
        "$V(a, b, c)",         // Maybe expects fewer
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (wrong arity): {:?}",
            case,
            result
        );
    }
}

#[test]
fn semantic_invalid_non_linear() {
    // Non-linear expressions parse but are semantically invalid for ILP
    let cases = vec![
        "$V1() * $V2()",        // Variable * Variable
        "$V() * $V()",          // Variable * itself
        "compute($V()) * $W()", // Function of variable * variable
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (non-linear): {:?}",
            case,
            result
        );
    }
}
