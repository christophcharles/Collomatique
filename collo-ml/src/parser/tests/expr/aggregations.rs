use super::*;

// =============================================================================
// AGGREGATION EXPRESSIONS
// =============================================================================
// Tests for: sum, forall (with optional where clause)

#[test]
fn aggregation_accepts_simple_sum() {
    let cases = vec![
        "sum x in students { $Var(x) }",
        "sum x in [1, 2, 3] { x }",
        "sum s in collection { 2 * $V(s) }",
        "sum x in set { $V1(x) + $V2(x) }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_accepts_sum_with_where() {
    let cases = vec![
        "sum x in students where x.is_active { $Var(x) }",
        "sum slot in slots where slot.hour > 8 { $StudentInSlot(s, slot, w) }",
        "sum x in collection where x.value > 0 and x.flag { $V(x) }",
        "sum x in [1, 2, 3] where x > 1 { x * 2 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_accepts_nested_sums() {
    let cases = vec![
        "sum x in x_list { sum y in y_list { $Var(x, y) } }",
        "sum student in students { sum week in weeks { $HasSubject(math, student, week) } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_accepts_sum_in_larger_expression() {
    let cases = vec![
        "(sum x in x_list { $V(x) }) + 10",
        "2 * (sum x in x_list { $V(x) })",
        "(sum x in x_list { $V(x) }) - (sum y in y_list { $V(y) })",
        "sum x in x_list { $V(x) + 5 }", // 5 inside sum
        "sum x in x_list { $V(x) } + 5", // 5 outside sum
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_accepts_simple_forall() {
    let cases = vec![
        "forall x in students { $Assigned(x) === 1 }",
        "forall week in weeks { sum slot in slots { $Used(slot, week) } <== 10 }",
        "forall s in collection { $V(s) >== 0 }",
        "forall x in [1, 2, 3] { $V(x) >== 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_accepts_forall_with_where() {
    let cases = vec![
        "forall x in students where x.is_active { $Assigned(x) === 1 }",
        "forall week in weeks where week.number > 10 { $V(week) <== 5 }",
        "forall s in collection where s.valid and s.flag { $V(s) >== 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_accepts_nested_forall() {
    let cases = vec![
        "forall x in x_list { forall y in y_list { $V(x, y) <== 1 } }",
        "forall student in students { forall week in weeks { sum slot in slots { $Assigned(student, slot, week) } <== 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_accepts_forall_with_and() {
    let cases = vec![
        "forall x in x_list { $V1(x) >= 0 and $V2(x) <= 10 }",
        "forall s in students { $Assigned(s) == 1 and sum w in weeks { $HasColle(s, w) } >== 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_with_collections() {
    let cases = vec![
        "forall s in students + teachers { $V(s) >== 0 }",
        "sum s in group_a + group_b { $V(s) }",
        "forall x in (group_a - excluded) { $V(x) === 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_rejects_incomplete_expressions() {
    let cases = vec![
        "sum x in x_list {",              // missing closing brace
        "sum x in x_list $V(x)",          // missing braces
        "forall x in x_list {",           // missing closing brace
        "forall x in x_list $V(x) <== 1", // missing braces
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn aggregation_rejects_exists() {
    // exists is not implemented
    let cases = vec!["exists x in x_list { $V(x) === 1 }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}
