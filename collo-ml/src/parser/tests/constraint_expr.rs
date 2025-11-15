use super::*;

#[test]
fn constraint_accepts_simple_comparisons() {
    let cases = vec![
        "$Var(x) <= 10",
        "$Var(x) >= 0",
        "$Var(x) == 1",
        "$V1(x) + $V2(y) <= 5",
        "sum x in @[X]: $V(x) >= 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_complex_linear_expressions() {
    let cases = vec![
        "2 * $V1(x) + 3 * $V2(y) <= 10",
        "$V1(x) - $V2(y) + 5 >= 0",
        "(sum x in @[X]: $V(x)) == |@[X]|",
        "(if flag { $V1(x) } else { $V2(x) }) <= 10",
        "$V(x) + compute_value(y) >= 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_and_operator() {
    let cases = vec![
        "$V1(x) <= 10 and $V2(y) >= 0",
        "$V1(x) <= 10 && $V2(y) >= 0",
        "$V(x) == 1 and $V(y) == 1 and $V(z) == 1",
        "sum x in @[X]: $V(x) >= 1 and sum y in @[Y]: $V(y) <= 10",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_forall() {
    let cases = vec![
        "forall x in @[Student]: $Assigned(x) == 1",
        "forall week in @[Week]: sum slot in slots: $Used(slot, week) <= 10",
        "forall s in collection: $V(s) >= 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_forall_with_where() {
    let cases = vec![
        "forall x in @[Student] where x.is_active: $Assigned(x) == 1",
        "forall week in @[Week] where week.number > 10: $V(week) <= 5",
        "forall s in collection where s.valid and s.flag: $V(s) >= 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_nested_forall() {
    let cases = vec![
        "forall x in @[X]: forall y in @[Y]: $V(x, y) <= 1",
        "forall student in @[Student]: forall week in @[Week]: sum slot in slots: $Assigned(student, slot, week) <= 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_forall_with_and() {
    let cases = vec![
        "forall x in @[X]: $V1(x) >= 0 and $V2(x) <= 10",
        "forall s in @[Student]: $Assigned(s) == 1 and sum w in @[Week]: $HasColle(s, w) >= 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_if_expressions() {
    let cases = vec![
        "if flag { $V1(x) <= 10 } else { $V2(x) <= 10 }",
        "if x > 5 { $V(x) == 1 } else { $V(x) == 0 }",
        "if condition { forall y in @[Y]: $V(y) >= 0 } else { $V(z) == 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_function_calls_returning_constraints() {
    let cases = vec![
        "enforce_rule(student, week)",
        "check_capacity(room, slot)",
        "apply_constraint(x, y, z)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_function_calls_with_and() {
    let cases = vec![
        "enforce_rule1(x) and enforce_rule2(y)",
        "$V(x) <= 10 and apply_constraint(x, y)",
        "forall x in @[X]: check_rule(x) and $V(x) >= 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_parentheses() {
    let cases = vec![
        "($V(x) <= 10)",
        "(forall x in @[X]: $V(x) >= 0)",
        "($V1(x) <= 10) and ($V2(y) >= 0)",
        "((($V(x) == 1)))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_accepts_complex_combinations() {
    let cases = vec![
        "forall student in @[Student]: forall week in @[Week]: (sum slot in subject.slots: $StudentInSlot(student, slot, week)) <= 1 and $HasSubject(subject, student, week) == 1",
        "(forall x in @[X]: $V(x) >= 0) and (forall y in @[Y]: $V(y) <= 10)",
        "if flag { forall x in @[X]: $V(x) == 1 } else { sum x in @[X]: $V(x) >= 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn constraint_rejects_or_operator() {
    let cases = vec!["$V1(x) <= 10 or $V2(y) >= 0", "$V(x) == 1 || $V(y) == 1"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (or not allowed): {:?}",
            case,
            result
        );
    }
}

#[test]
fn constraint_rejects_not_operator() {
    let cases = vec!["not $V(x) <= 10", "!($V(x) == 1)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (not not allowed): {:?}",
            case,
            result
        );
    }
}

#[test]
fn constraint_rejects_exists() {
    let cases = vec!["exists x in @[X]: $V(x) == 1"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (exists not allowed): {:?}",
            case,
            result
        );
    }
}

#[test]
fn constraint_rejects_forbidden_comparisons() {
    let cases = vec![
        "$V(x) < 10", // only <=, >=, == allowed
        "$V(x) > 0",
        "$V(x) != 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (forbidden comparison): {:?}",
            case,
            result
        );
    }
}

#[test]
fn constraint_rejects_arithmetic_without_comparison() {
    let cases = vec![
        "$V(x) + $V(y)", // missing comparison
        "2 * $V(x)",
        "sum x in @[X]: $V(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing comparison): {:?}",
            case,
            result
        );
    }
}

#[test]
fn constraint_rejects_incomplete_expressions() {
    let cases = vec![
        "$V(x) <=",
        "forall x in @[X]:",
        "forall x in @[X] $V(x) <= 1", // missing colon
        "$V(x) <= 10 and",
        "and $V(x) >= 0",
        "($V(x) <= 10",
        "$V(x) <= 10)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::constraint_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}
