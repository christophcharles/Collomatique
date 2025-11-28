use super::*;

// =============================================================================
// IDENTIFIER GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of identifiers.
//
// Grammar: ident = @{ !(reserved_keyword ~ !ident_char) ~ (ASCII_ALPHA | "_") ~ ident_char* }
// where ident_char = ASCII_ALPHANUMERIC | "_"
//
// Valid identifiers:
// - Start with ASCII letter or underscore
// - Contain only ASCII letters, digits, or underscores
// - Cannot be reserved keywords (but can contain them as substrings)

// =============================================================================
// VALID IDENTIFIERS - BASIC
// =============================================================================

#[test]
fn ident_accepts_single_character() {
    let cases = vec!["x", "a", "A", "Z", "_"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_simple_names() {
    let cases = vec!["x", "student", "myVariable", "value", "name"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_names_with_numbers() {
    let cases = vec!["var123", "x1", "name2", "value3test", "a1b2c3"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_names_with_underscores() {
    let cases = vec![
        "my_variable",
        "snake_case_name",
        "x_",
        "a__b",
        "name___",
        "var_1_2_3",
        "test_123_var",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_names_starting_with_underscore() {
    let cases = vec![
        "_variable",
        "_student",
        "__double",
        "_123",
        "_",
        "_x",
        "_my_var",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (starts with underscore): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// VALID IDENTIFIERS - CASE VARIATIONS
// =============================================================================

#[test]
fn ident_accepts_lowercase() {
    let cases = vec!["lowercase", "simple", "test", "variable"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_uppercase() {
    let cases = vec!["UPPERCASE", "CONSTANT", "MAX", "VALUE"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_mixed_case() {
    let cases = vec![
        "MixedCase",
        "camelCase",
        "PascalCase",
        "snake_Case",
        "SCREAMING_SNAKE_CASE",
        "mixedCase123",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// VALID IDENTIFIERS - WITH KEYWORD SUBSTRINGS
// =============================================================================

#[test]
fn ident_accepts_keywords_as_prefix() {
    // Keywords can appear as prefixes in identifiers
    let cases = vec![
        "sum_total",
        "forall_items",
        "if_condition",
        "let_binding",
        "return_value",
        "where_clause",
        "in_scope",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_keywords_as_suffix() {
    // Keywords can appear as suffixes in identifiers
    let cases = vec![
        "total_sum",
        "items_forall",
        "condition_if",
        "binding_let",
        "clause_where",
        "scope_in",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn ident_accepts_keywords_as_substring() {
    // Keywords can appear in the middle of identifiers
    let cases = vec![
        "my_sum_value",
        "check_if_valid",
        "has_in_list",
        "compute_and_store",
        "value_or_default",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// NEGATIVE TESTS - RESERVED KEYWORDS
// =============================================================================

#[test]
fn ident_rejects_reserved_keywords() {
    let cases = vec![
        // Control flow
        "if", "else", // Declarations
        "let", "pub", "reify", "as", // Quantifiers/aggregations
        "forall", "sum", "where", "in", "for", // Logical operators
        "and", "or", "not", // Boolean literals
        "true", "false",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (reserved keyword): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// NEGATIVE TESTS - INVALID CHARACTERS
// =============================================================================

#[test]
fn ident_rejects_names_starting_with_digit() {
    let cases = vec![
        "1variable",
        "123",
        "0student",
        "9name",
        "1_underscore",
        "2test",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (starts with digit): {:?}",
            case,
            result
        );
    }
}

#[test]
fn ident_rejects_names_with_hyphens() {
    let cases = vec!["my-variable", "test-name", "some-identifier", "x-y"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (contains hyphen): {:?}",
            case,
            result
        );
    }
}

#[test]
fn ident_rejects_names_with_dots() {
    let cases = vec!["my.variable", "test.name", "x.y.z"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (contains dot): {:?}",
            case,
            result
        );
    }
}

#[test]
fn ident_rejects_names_with_spaces() {
    let cases = vec!["my variable", "test name", "x y"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (contains space): {:?}",
            case,
            result
        );
    }
}

#[test]
fn ident_rejects_names_with_special_characters() {
    let cases = vec![
        "my@variable",
        "my$variable",
        "my%variable",
        "my!variable",
        "my&variable",
        "my*variable",
        "my+variable",
        "my=variable",
        "my[variable",
        "my]variable",
        "my{variable",
        "my}variable",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (contains special character): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// NEGATIVE TESTS - EMPTY OR INVALID
// =============================================================================

#[test]
fn ident_rejects_empty_string() {
    let result = ColloMLParser::parse(Rule::ident_complete, "");
    assert!(result.is_err(), "Should reject empty string: {:?}", result);
}

#[test]
fn ident_rejects_only_digits() {
    let cases = vec!["123", "0", "999"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (only digits): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn ident_accepts_single_underscore() {
    let result = ColloMLParser::parse(Rule::ident_complete, "_");
    assert!(
        result.is_ok(),
        "Should accept single underscore: {:?}",
        result
    );
}

#[test]
fn ident_accepts_very_long_names() {
    let long_name = "a".repeat(100);
    let result = ColloMLParser::parse(Rule::ident_complete, &long_name);
    assert!(
        result.is_ok(),
        "Should accept very long identifier: {:?}",
        result
    );
}

#[test]
fn ident_distinguishes_keywords_from_similar_names() {
    // These should be accepted (not exact keywords)
    let cases = vec![
        "lets",    // "let" + "s"
        "sum_",    // "sum" + "_"
        "foralls", // "forall" + "s"
        "ifs",     // "if" + "s"
        "ins",     // "in" + "s"
        "ands",    // "and" + "s"
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::ident_complete, case);
        assert!(
            result.is_ok(),
            "Should accept '{}' (not a keyword): {:?}",
            case,
            result
        );
    }
}
