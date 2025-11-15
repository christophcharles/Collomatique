use super::*;

#[test]
fn ident_accepts_valid_identifiers() {
    let cases = vec![
        "x",
        "student",
        "myVariable",
        "var123",
        "my_variable",
        "snake_case_name",
        "camelCaseName",
        "PascalCaseName",
        "name_with_123_numbers",
        "a",
        "A",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_keywords_with_suffix() {
    let cases = vec![
        "sum_total",
        "forall_items",
        "if_condition",
        "let_binding",
        "while_loop",
        "return_value",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_rejects_starting_with_digit() {
    let cases = vec!["1variable", "123", "0student", "9name", "1_underscore"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (starts with digit): {:?}",
            case,
            result
        );
    }
}

#[test]
fn ident_rejects_starting_with_underscore() {
    let cases = vec!["_variable", "_student", "__double", "_123", "_"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (starts with underscore): {:?}",
            case,
            result
        );
    }
}

#[test]
fn ident_rejects_reserved_keywords() {
    let cases = vec![
        "let", "pub", "reify", "as", "forall", "sum", "if", "else", "where", "in", "and", "or",
        "not", "union", "inter",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (reserved keyword): {:?}",
            case,
            result
        );
    }
}

#[test]
fn ident_rejects_special_characters() {
    let cases = vec![
        "my-variable",
        "my.variable",
        "my variable",
        "my@variable",
        "my$variable",
        "my%variable",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (special character): {:?}",
            case,
            result
        );
    }
}

#[test]
fn ident_rejects_empty_string() {
    let result = ColloMLParser::parse(Rule::ident_complete, "");
    assert!(result.is_err(), "Should not parse empty string");
}

#[test]
fn ident_accepts_case_variations() {
    let cases = vec![
        "lowercase",
        "UPPERCASE",
        "MixedCase",
        "camelCase",
        "PascalCase",
        "snake_case",
        "SCREAMING_SNAKE_CASE",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_with_only_underscores_after_letter() {
    let cases = vec!["x_", "a__", "name___", "var_"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_mixed_numbers_and_underscores() {
    let cases = vec!["var_1_2_3", "a1b2c3", "test_123_var", "x1_y2_z3"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
