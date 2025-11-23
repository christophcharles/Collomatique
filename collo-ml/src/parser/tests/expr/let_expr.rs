use super::*;

#[test]
fn let_in_accepts_simple_bindings() {
    let cases = vec![
        "let x = 5 { x + 1 }",
        "let doubled = n * 2 { doubled }",
        "let result = a + b { result * 2 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_accepts_nested_bindings() {
    let cases = vec![
        "let x = 1 { let y = 2 { x + y } }",
        "let a = 5 { let b = a * 2 { let c = b + 1 { c } } }",
        "let outer = 10 { let inner = outer * 2 { inner + outer } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_accepts_complex_value_expressions() {
    let cases = vec![
        "let sum_ = a + b * c { sum_ }",
        "let computed = f(x, y) { computed + 1 }",
        "let range = [0..10] { range }",
        "let list = [1, 2, 3] { list }",
        "let cond = x > 5 and y < 10 { cond }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_accepts_complex_body_expressions() {
    let cases = vec![
        "let x = 5 { if x > 3 { 10 } else { 20 } }",
        "let n = 10 { forall i in [0..n] { $V(i) === 1 } }",
        "let items = [1, 2, 3] { sum i in items { i * 2 } }",
        "let base = 5 { |[0..base]| }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_with_membership_operator() {
    let cases = vec![
        "let is_member = x in list { is_member }",
        "let check = 5 in [1, 2, 3, 4, 5] { if check { 1 } else { 0 } }",
        "let found = item in collection { found }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_with_paths_and_field_access() {
    let cases = vec![
        "let age = student.age { age + 1 }",
        "let val = obj.field.subfield { val * 2 }",
        "let active = person.is_active { active }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_with_constraints() {
    let cases = vec![
        "let bound = n * 2 { $V(bound) === 1 }",
        "let limit = 10 { $V(x) <== limit }",
        "let threshold = a + b { $V(y) >== threshold }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_chained_with_different_expressions() {
    let cases = vec![
        "let a = 1 { let b = a + 1 { let c = b * 2 { c + a } } }",
        "let x = [1, 2, 3] { let y = |x| { x union y } }",
        "let n = 5 { let doubled = n * 2 { forall i in [0..doubled] { $V(i) === 1 } } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_with_list_comprehensions() {
    let cases = vec![
        "let bound = 10 { [i * 2 for i in [0..bound]] }",
        "let xs = [1, 2, 3] { [x + 1 for x in xs] }",
        "let n = 5 { [i for i in [0..n] where i > 2] }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_with_boolean_expressions() {
    let cases = vec![
        "let cond = x > 5 { cond and y < 10 }",
        "let check = a == b { not check }",
        "let valid = x >= 0 and x <= 100 { valid or y > 50 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_with_type_annotations() {
    let cases = vec![
        "let x = 5 as Int { x + 1 }",
        "let expr = $V(5) as LinExpr { expr }",
        "let items = [1, 2, 3] as [Int] { items }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_rejects_invalid_syntax() {
    let cases = vec![
        "let x = 5",       // Missing body
        "let x { 5 }",     // Missing = and value
        "let = 5 { x }",   // Missing variable name
        "x = 5 { x }",     // Missing let keyword
        "let x = 5 } x {", // Wrong bracket order
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should NOT parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_in_in_function_definitions() {
    let cases = vec![
        "let f(x: Int) -> Int = let doubled = x * 2 { doubled + 1 };",
        "let g(n: Int) -> Constraint = let bound = n * 2 { $V(bound) === 1 };",
        "let h(a: Int, b: Int) -> Int = let sum_ = a + b { let prod = a * b { sum_ + prod } };",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
