use super::*;

// =============================================================================
// COMPLEX INTEGRATION TESTS
// =============================================================================
// Tests that combine multiple expression features

#[test]
fn complex_realistic_linexpr() {
    let cases = vec![
        // Variable calls with arithmetic
        "2 * $V1(x) + 3 * $V2(y)",
        "$V1(x) + 2 * $V2(y) - $V3(z)",
        "10 + 2 * $V1(x) - 3 * $V2(y) + 5",
        // With cardinality
        "|@[Week]| * $V(s)",
        "(|@[Student]|) * $Var(x)",
        // With aggregations
        "sum s in @[Student] { s.weight * $Assigned(s) }",
        // With conditionals
        "if s.priority > 5 { 10 * $V(s) } else { $V(s) }",
        // With function calls
        "compute_base(s) + 2 * $Extra(s)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn complex_realistic_constraints() {
    let cases = vec![
        // Forall with sum constraint
        "forall w in @[Week] { sum s in @[Student] { $Assigned(s, w) } <== 10 }",

        // Conjunction of constraints
        "$V1(s) >== 0 and $V1(s) <== s.max_value",

        // Nested forall
        "forall s in @[Student] { forall w in @[Week] { $InWeek(s, w) <== 1 } }",

        // Conditional constraint
        "if r.available { sum s in @[Student] { $InRoom(s, r) } <== r.capacity } else { sum s in @[Student] { $InRoom(s, r) } === 0 }",

        // Complex nested structure
        "forall student in @[Student] { forall week in @[Week] { (sum slot in subject.slots { $StudentInSlot(student, slot, week) }) <== 1 and $HasSubject(subject, student, week) === 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn complex_deeply_nested_expressions() {
    let cases = vec![
        // Nested parentheses
        "((($Var(x))))",
        "(($V1(x) <== 10) and ($V2(y) >== 0))",

        // Nested aggregations
        "sum x in @[X] { sum y in @[Y] { $Var(x, y) } }",
        "forall x in @[X] { forall y in @[Y] { $V(x, y) <== 1 } }",

        // Nested collections
        "((a + b) - c) + d",
        "@[Student] - (excluded - suspended)",

        // Nested if expressions
        "if x { if y { 1 } else { 2 } } else { 3 }",

        // Complex combination
        "forall s in (@[Student] - excluded) where s.active { sum w in @[Week] where w.number > 10 { if s.priority > 5 { 2 * $V(s, w) } else { $V(s, w) } } <== s.max_load }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn complex_mixed_operators() {
    let cases = vec![
        // Arithmetic, comparison, logical
        "(a + b) * 2 > 10 and x or y",
        "x > 0 and y > 0 or z > 0",
        "not (x > 0 and y > 0)",
        // Collection, membership, logical
        "a in set1 and not (b in set2)",
        "student in @[Student] and student.age > 18",
        // Constraint and logical
        "$V1(x) === 0 and $V2(y) >== 1",
        "($V(x) === 1) and ($V(y) === 1)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn complex_function_and_variable_calls() {
    let cases = vec![
        // Function calls with complex arguments
        "compute($V1(x) + $V2(y))",
        "check($V(x) <== 10)",
        "process(@[Student] + @[Teacher])",
        "func([x for x in @[S]])",
        "nested(outer(inner(x)))",
        // Variable calls with complex arguments
        "$StudentInSlot(student, slot, week.number)",
        "$Assigned(student, |@[Week]|)",
        "$Value(if x { 1 } else { 0 }, y)",
        "$V(x + 5, y * 2)",
        // Mixed
        "enforce_rule1(x) and enforce_rule2(y)",
        "$V(x) <== 10 and apply_constraint(x, y)",
        "forall x in @[X] { check_rule(x) and $V(x) >== 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn complex_with_all_features() {
    let cases = vec![
        // Kitchen sink expression
        "if flag { forall x in (@[X] - excluded) where x > 0 { sum y in @[Y] { (2 * $V1(x, y) + compute(x)) as LinExpr } <== |@[Y]| } } else { sum x in @[X] { $V2(x) } >= 1 }",

        // Another complex one
        "forall s in @[Student] + @[Teacher] where s.active { (if s.type == 1 { 2 } else { 1 }) * (sum w in @[Week] { $Assigned(s, w) }) === |@[Week]| and s.age >= 18 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn complex_precedence_checks() {
    let cases = vec![
        // Multiplication before addition
        "1 + 2 * 3",
        // Left associativity of subtraction
        "10 - 5 - 2",
        // And before or
        "x and y or z",
        // Not before and
        "not x and y",
        // Comparison before and
        "a < b and c > d",
        // As binds tighter than arithmetic
        "x as Int + 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn complex_with_parentheses() {
    let cases = vec![
        // Override precedence
        "(x > 5) and (y > 5)",
        "(5 + 3) * 2",
        "(x > 0 and y > 0) or z > 0",
        // Nested parentheses
        "(((a + b) * c))",
        // With aggregations
        "(sum x in @[X] { $V(x) })",
        "(forall x in @[X] { $V(x) >= 0 })",
        // With collections
        "(@[Subject] - pairing)",
        "(group1 + group2)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
