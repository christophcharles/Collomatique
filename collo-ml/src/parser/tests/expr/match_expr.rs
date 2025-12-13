use super::*;

// =============================================================================
// MATCH EXPRESSIONS
// =============================================================================
// Tests for match expressions with type patterns

#[test]
fn match_accepts_simple_type_patterns() {
    let cases = vec![
        "match x { Int { 10 } }",
        "match value { Bool { true } }",
        "match student { Student { 1 } }",
        "match item { None { 0 } Int { 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_else_clause() {
    let cases = vec![
        "match x { Int { 10 } else { 0 } }",
        "match value { Bool { 1 } else { 0 } }",
        "match item { Student { x } else { 0 } }",
        "match x { else { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_into_type_conversion() {
    let cases = vec![
        "match x { Int into Bool { true } }",
        "match value { Student into Int { 1 } }",
        "match item { Int into LinExpr { x } else { 0 } }",
        "match x { Bool into Constraint { $V(x) === 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_where_filters() {
    let cases = vec![
        "match x { Int where x > 5 { 10 } }",
        "match value { Student where value.age > 18 { 1 } }",
        "match item { Int where item > 0 { item } else { 0 } }",
        "match x { Bool where x { 1 } else { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_combined_into_and_where() {
    let cases = vec![
        "match x { Int into Bool where x > 5 { true } }",
        "match value { Student into Int where value.age > 18 { 1 } }",
        "match item { Int into LinExpr where item > 0 { $V(item) } else { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_union_types() {
    let cases = vec![
        "match x { Int | Bool { 1 } }",
        "match value { Student | Teacher { x } }",
        "match item { Int | Bool | None { 0 } else { 1 } }",
        "match x { ?Int | Bool { 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_maybe_types() {
    let cases = vec![
        "match x { ?Int { 10 } }",
        "match value { ??Bool { true } }",
        "match item { ?Student { 1 } else { 0 } }",
        "match x { ?Int | ??Bool { 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_list_types() {
    let cases = vec![
        "match x { [Int] { 10 } }",
        "match value { [Student] { |value| } }",
        "match item { [[Int]] { 0 } }",
        "match x { [Int] | Int { 1 } else { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_multiple_branches() {
    let cases = vec![
        "match x { Int { 1 } Bool { 2 } else { 3 } }",
        "match value { Student { a } Teacher { b } None { 0 } }",
        "match item { Int where x > 0 { 1 } Int where x < 0 { -1 } else { 0 } }",
        "match x { Int into Bool { true } Bool into Int { 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_complex_bodies() {
    let cases = vec![
        "match x { Int { x + 5 } }",
        "match value { Student { value.age * 2 } }",
        "match item { Int { if item > 0 { item } else { 0 } } }",
        "match x { Bool { sum y in @[Y] { $V(y) } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_paths_and_field_access() {
    let cases = vec![
        "match student { Student { student.age } }",
        "match item { Student { item.group.name } }",
        "match x { Int { obj.field } else { 0 } }",
        "match value { Student { value.is_active } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_arithmetic_bodies() {
    let cases = vec![
        "match x { Int { x * 2 } else { 0 } }",
        "match value { Student { value.age + 10 } }",
        "match item { Int { item // 2 } Bool { 1 } }",
        "match x { Int { (x + 5) * 2 } else { x - 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_variable_calls() {
    let cases = vec![
        "match x { Int { $V(x) } }",
        "match value { Student { $V1(value) } else { $V2(value) } }",
        "match item { Int { $V(item) === 1 } }",
        "match x { Bool { $V(x) <= 10 } else { $W(x) >== 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_constraints() {
    let cases = vec![
        "match x { Int { $V(x) === 1 } }",
        "match value { Student { $V(value.age) >== 18 } }",
        "match item { Int { $V(item) === 0 } else { $V(item) === 1 } }",
        "match x { Bool { forall y in @[Y] { $V(y) === x } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_boolean_expressions() {
    let cases = vec![
        "match x { Int { x > 5 } }",
        "match value { Student { value.age > 18 and value.is_active } }",
        "match item { Int { item in collection } else { false } }",
        "match x { Bool { x or not x } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_collections() {
    let cases = vec![
        "match x { [Int] { |x| } }",
        "match value { [Student] { sum s in value { s.age } } }",
        "match item { [Int] { [y for y in item] } }",
        "match x { Int { x } else { |@[Int]| } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn nested_match_expressions() {
    let cases = vec![
        "match x { Int { match x { Int { 1 } else { 0 } } } }",
        "match a { Student { match a.group { Group { 1 } else { 0 } } } }",
        "match x { Int { 1 } else { match y { Bool { 2 } else { 3 } } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_in_arithmetic_context() {
    let cases = vec![
        "(match x { Int { 10 } else { 0 } }) + 5",
        "2 * (match value { Student { value.age } else { 0 } })",
        "match x { Int { x } else { 0 } } + match y { Int { y } else { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_let_expressions() {
    let cases = vec![
        "match x { Int { let y = x { y + 1 } } }",
        "match value { Student { let age = value.age { age * 2 } } }",
        "let temp = match x { Int { x } else { 0 } } { temp + 5 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_quantifiers() {
    let cases = vec![
        "match x { [Int] { sum i in x { i } } }",
        "match students { [Student] { forall s in students { s.age > 0 } } }",
        "match items { [Int] { fold i in items with acc = 0 { acc + i } } }",
        "match x { Int { sum y in @[Y] where y > x { $V(y) } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_rejects_missing_braces() {
    let cases = vec![
        "match x { Int 10 }",
        "match x { Int { 10 }",
        "match x Int { 10 } }",
        "match x { Int { 10 ",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn match_rejects_empty_bodies() {
    let cases = vec![
        "match x { Int { } }",
        "match x { Int { } else { 0 } }",
        "match x { Int { 1 } else { } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn match_rejects_missing_type_pattern() {
    let cases = vec![
        "match x { { 10 } }",
        "match x { where x > 5 { 10 } }",
        "match x { into Bool { true } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn match_rejects_missing_expression() {
    let cases = vec!["match { Int { 10 } }", "match { else { 0 } }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_no_branches() {
    let cases = vec!["match x { }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should accept '{}': {:?}", case, result);
    }
}

#[test]
fn match_rejects_malformed_patterns() {
    let cases = vec![
        "match x { Int where { 10 } }",
        "match x { Int into { 10 } }",
        "match x { into Bool where x { true } }",
        "match x { where x into Bool { true } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn match_complex_real_world_examples() {
    let cases = vec![
        // Type-based dispatch
        "match value { Int { $IntVar(value) } Bool { if value { 1 } else { 0 } } else { 0 } }",

        // Optional handling with filtering
        "match student { ?Student where student.age > 18 { student.age } else { 0 } }",

        // List processing
        "match items { [Int] { sum i in items where i > 0 { i } } else { 0 } }",

        // Nested type patterns with conversion
        "match data { Int into LinExpr { $V(data) } [Int] into Constraint { sum x in data { $V(x) } === 10 } }",

        // Complex filtering
        "match students { [Student] where |students| > 0 { forall s in students { s.age >== 18 } } else { true } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
