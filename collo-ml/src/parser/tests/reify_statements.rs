use super::*;

#[test]
fn reify_accepts_simple_reification() {
    let cases = vec![
        "reify my_constraint as $MyVar;",
        "reify student_has_subject as $HasSubject;",
        "reify is_valid as $Valid;",
        "reify check123 as $Check123;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_accepts_with_docstrings() {
    let cases = vec![
        "## This reifies a constraint\nreify my_constraint as $MyVar;",
        "## First line\n## Second line\nreify student_has_subject as $HasSubject;",
        "## Documentation\n## More docs\n## Even more\nreify check as $Check;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_accepts_identifiers_with_underscores() {
    let cases = vec![
        "reify has_any_slot as $HasAnySlot;",
        "reify student_in_slot_check as $StudentInSlotCheck;",
        "reify my_long_constraint_name as $MyLongVar;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_accepts_variable_names_with_underscores() {
    let cases = vec![
        "reify constraint as $My_Var;",
        "reify check as $Student_Has_Subject;",
        "reify rule as $Is_Valid_123;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_rejects_missing_dollar_sign() {
    let cases = vec!["reify my_constraint as MyVar;", "reify check as Valid;"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing $): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_rejects_constraint_expression() {
    let cases = vec![
        "reify ($V(x) <= 10) as $MyVar;", // Can't reify inline expression
        "reify (forall x in @[X]: $V(x) >= 0) as $Check;",
        "reify $V(x) == 1 as $IsOne;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (inline expression): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_rejects_function_call() {
    let cases = vec![
        "reify compute_constraint(x) as $Result;",
        "reify my_func(a, b) as $MyVar;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (function call): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_rejects_missing_as_keyword() {
    let cases = vec!["reify my_constraint $MyVar;", "reify check = $Valid;"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing 'as'): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_rejects_missing_semicolon() {
    let cases = vec!["reify my_constraint as $MyVar", "reify check as $Valid\n"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing semicolon): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_rejects_invalid_identifier() {
    let cases = vec![
        "reify 123constraint as $MyVar;", // starts with digit
        "reify my-constraint as $MyVar;", // hyphen not allowed
        "reify my constraint as $MyVar;", // space not allowed
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (invalid identifier): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_rejects_invalid_variable_name() {
    let cases = vec![
        "reify constraint as $123Var;", // starts with digit
        "reify check as $My-Var;",      // hyphen not allowed
        "reify rule as $My Var;",       // space not allowed
        "reify test as $$MyVar;",       // double dollar
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (invalid variable name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_rejects_reserved_keywords_as_identifiers() {
    let cases = vec![
        "reify let as $MyVar;",
        "reify forall as $Check;",
        "reify sum as $Total;",
        "reify if as $Condition;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (reserved keyword): {:?}",
            case,
            result
        );
    }
}

#[test]
fn reify_allows_keyword_prefixes() {
    let cases = vec![
        "reify sum_constraint as $SumConstraint;",
        "reify forall_check as $ForallCheck;",
        "reify if_valid as $IfValid;",
        "reify let_binding as $LetBinding;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::reify_statement, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn reify_complete_example() {
    let input = r#"## Check if student has any slot in subject for a week
reify has_any_slot_in_subject as $HasSubject;"#;
    let result = ColloMLParser::parse(Rule::reify_statement, input);
    assert!(
        result.is_ok(),
        "Should parse complete example: {:?}",
        result
    );
}
