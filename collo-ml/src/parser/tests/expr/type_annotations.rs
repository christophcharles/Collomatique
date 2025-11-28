use super::*;

// =============================================================================
// TYPE ANNOTATION EXPRESSIONS
// =============================================================================
// Tests for 'as' type cast operator

#[test]
fn type_annotation_simple_types() {
    let cases = vec![
        "x as Int",
        "value as Bool",
        "$V(x) as LinExpr",
        "compute() as Bool",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_list_types() {
    let cases = vec!["[] as [Int]", "[] as [Student]", "collection as [Week]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_nested_list_types() {
    let cases = vec![
        "[] as [[Int]]",
        "[[]] as [[Student]]",
        "nested_list as [[[Bool]]]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_in_expressions() {
    let cases = vec![
        "([] as [Int]) + [1, 2, 3]",
        "[x for x in ([] as [Student])]",
        "|([] as [Int])|",
        "sum x in ([] as [Int]) { x }",
        "if flag { [] as [Int] } else { [1, 2] }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_with_parentheses() {
    let cases = vec![
        "([] as [Int])",
        "(([] as [Student]))",
        "(x as Int) + 5",
        "2 * (value as Int)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_in_function_args() {
    let cases = vec![
        "process([] as [Int])",
        "compute(x as Int, y as Bool)",
        "func(([] as [Student]), value as Int)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_precedence() {
    // 'as' binds tighter than arithmetic
    let cases = vec![
        "x as Int + 5",        // (x as Int) + 5
        "2 * value as Int",    // 2 * (value as Int)
        "a as Int + b as Int", // (a as Int) + (b as Int)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_with_complex_types() {
    let cases = vec![
        "value as LinExpr",
        "constraint as Constraint",
        "flag as Bool",
        "[] as [Int]",
        "[] as [[Int]]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_in_list_literals() {
    let cases = vec!["[([] as [Int]), [1, 2]]", "[x as Int, y as Int, z as Int]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_with_constraints() {
    let cases = vec![
        "$V(x as Int) <== 10",
        "sum x in ([] as [Student]) { $V(x) } >== 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn type_annotation_rejects_missing_type() {
    let cases = vec!["x as", "[] as", "as [Int]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}
