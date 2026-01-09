use super::*;

// =============================================================================
// MATCH EXPRESSIONS
// =============================================================================
// Tests for match expressions with type patterns

#[test]
fn match_accepts_simple_type_patterns() {
    let cases = vec![
        "match x { y as Int { 10 } }",
        "match value { v as Bool { true } }",
        "match student { s as Student { 1 } }",
        "match item { i as None { 0 } j as Int { 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_else_clause() {
    let cases = vec![
        "match x { y as Int { 10 } z { 0 } }",
        "match value { v as Bool { 1 } w { 0 } }",
        "match item { s as Student { x } other { 0 } }",
        "match x { y { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_type_conversion_in_body() {
    // Type conversions are now done with C-like syntax in the body
    let cases = vec![
        "match x { y as Int { Bool(true) } }",
        "match value { v as Student { Int(1) } }",
        "match item { i as Int { LinExpr(x) } other { LinExpr(0) } }",
        "match x { y as Bool { $V(x) === 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_where_filters() {
    let cases = vec![
        "match x { y as Int where x > 5 { 10 } }",
        "match value { v as Student where value.age > 18 { 1 } }",
        "match item { i as Int where item > 0 { item } other { 0 } }",
        "match x { y as Bool where x { 1 } z { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_where_with_conversion_in_body() {
    // Type conversions are now done in the body, not with 'into'
    let cases = vec![
        "match x { y as Int where x > 5 { Bool(true) } }",
        "match value { v as Student where value.age > 18 { Int(1) } }",
        "match item { i as Int where item > 0 { LinExpr($V(item)) } other { LinExpr(0) } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_union_types() {
    let cases = vec![
        "match x { y as Int | Bool { 1 } }",
        "match value { v as Student | Teacher { x } }",
        "match item { i as Int | Bool | None { 0 } other { 1 } }",
        "match x { y as ?Int | Bool { 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_maybe_types() {
    let cases = vec![
        "match x { y as ?Int { 10 } }",
        "match value { v as ??Bool { true } }",
        "match item { i as ?Student { 1 } other { 0 } }",
        "match x { y as ?Int | ??Bool { 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_list_types() {
    let cases = vec![
        "match x { y as [Int] { 10 } }",
        "match value { v as [Student] { |value| } }",
        "match item { i as [[Int]] { 0 } }",
        "match x { y as [Int] | Int { 1 } other { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_accepts_multiple_branches() {
    let cases = vec![
        "match x { i as Int { 1 } b as Bool { 2 } other { 3 } }",
        "match value { s as Student { a } t as Teacher { b } n as None { 0 } }",
        "match item { i as Int where x > 0 { 1 } j as Int where x < 0 { -1 } other { 0 } }",
        "match x { i as Int { Bool(true) } b as Bool { Int(1) } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_complex_bodies() {
    let cases = vec![
        "match x { y as Int { x + 5 } }",
        "match value { v as Student { value.age * 2 } }",
        "match item { i as Int { if item > 0 { item } else { 0 } } }",
        "match x { y as Bool { sum y in @[Y] { $V(y) } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_paths_and_field_access() {
    let cases = vec![
        "match student { s as Student { student.age } }",
        "match item { i as Student { item.group.name } }",
        "match x { i as Int { obj.field } other { 0 } }",
        "match value { v as Student { value.is_active } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_arithmetic_bodies() {
    let cases = vec![
        "match x { i as Int { x * 2 } other { 0 } }",
        "match value { v as Student { value.age + 10 } }",
        "match item { i as Int { item / 2 } b as Bool { 1 } }",
        "match x { i as Int { (x + 5) * 2 } other { x - 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_variable_calls() {
    let cases = vec![
        "match x { i as Int { $V(x) } }",
        "match value { v as Student { $V1(value) } other { $V2(value) } }",
        "match item { i as Int { $V(item) === 1 } }",
        "match x { b as Bool { $V(x) <= 10 } other { $W(x) >== 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_constraints() {
    let cases = vec![
        "match x { i as Int { $V(x) === 1 } }",
        "match value { v as Student { $V(value.age) >== 18 } }",
        "match item { i as Int { $V(item) === 0 } other { $V(item) === 1 } }",
        "match x { b as Bool { forall y in @[Y] { $V(y) === x } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_boolean_expressions() {
    let cases = vec![
        "match x { i as Int { x > 5 } }",
        "match value { v as Student { value.age > 18 and value.is_active } }",
        "match item { i as Int { item in collection } other { false } }",
        "match x { b as Bool { x or not x } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_collections() {
    let cases = vec![
        "match x { lst as [Int] { |x| } }",
        "match value { v as [Student] { sum s in value { s.age } } }",
        "match item { i as [Int] { [y for y in item] } }",
        "match x { i as Int { x } other { |@[Int]| } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn nested_match_expressions() {
    let cases = vec![
        "match x { i as Int { match x { j as Int { 1 } other { 0 } } } }",
        "match a { s as Student { match a.group { g as Group { 1 } other { 0 } } } }",
        "match x { i as Int { 1 } other { match y { b as Bool { 2 } z { 3 } } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_in_arithmetic_context() {
    let cases = vec![
        "(match x { i as Int { 10 } other { 0 } }) + 5",
        "2 * (match value { v as Student { value.age } other { 0 } })",
        "match x { i as Int { x } other { 0 } } + match y { j as Int { y } z { 0 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_let_expressions() {
    let cases = vec![
        "match x { i as Int { let y = x { y + 1 } } }",
        "match value { v as Student { let age = value.age { age * 2 } } }",
        "let temp = match x { i as Int { x } other { 0 } } { temp + 5 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_with_quantifiers() {
    let cases = vec![
        "match x { lst as [Int] { sum i in x { i } } }",
        "match students { s as [Student] { forall s in students { s.age > 0 } } }",
        "match items { i as [Int] { fold i in items with acc = 0 { acc + i } } }",
        "match x { i as Int { sum y in @[Y] where y > x { $V(y) } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn match_rejects_missing_braces() {
    let cases = vec![
        "match x { y as Int 10 }",
        "match x { y as Int { 10 }",
        "match x y as Int { 10 } }",
        "match x { y as Int { 10 ",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn match_rejects_empty_bodies() {
    let cases = vec![
        "match x { y as Int { } }",
        "match x { y as Int { } other { 0 } }",
        "match x { y as Int { 1 } other { } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn match_rejects_missing_identifier() {
    let cases = vec![
        "match x { as Int { 10 } }",
        "match x { where x > 5 { 10 } }",
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
fn match_requires_at_least_one_branch() {
    // Empty match is ambiguous with struct type cast syntax: `match x { }` could be `x { }` (empty struct cast)
    // This is a grammar ambiguity, so empty match expressions don't parse
    let cases = vec!["match x { }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject empty match '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn match_rejects_malformed_patterns() {
    let cases = vec![
        "match x { y as Int where { 10 } }",
        "match x { y as { 10 } }",
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
        "match value { i as Int { $IntVar(value) } b as Bool { if value { 1 } else { 0 } } other { 0 } }",

        // Optional handling with filtering
        "match student { s as ?Student where student.age > 18 { student.age } other { 0 } }",

        // List processing
        "match items { lst as [Int] { sum i in items where i > 0 { i } } other { 0 } }",

        // Type-based dispatch with conversion in body
        "match data { i as Int { LinExpr($V(data)) } lst as [Int] { (sum x in data { $V(x) }) === 10 } }",

        // Complex filtering
        "match students { s as [Student] where |students| > 0 { forall s in students { s.age >== 18 } } other { true } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
