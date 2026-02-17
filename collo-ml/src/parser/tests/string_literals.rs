use super::*;

// =============================================================================
// STRING LITERAL TESTS
// =============================================================================

// =============================================================================
// BASIC STRING LITERALS
// =============================================================================

#[test]
fn string_literal_basic() {
    let cases = vec![
        r#""hello""#,
        r#""world""#,
        r#""""#, // empty string
        r#""Hello, World!""#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn string_literal_with_spaces_and_punctuation() {
    let cases = vec![
        r#""hello world""#,
        r#""Hello, how are you?""#,
        r#""Multiple   spaces""#,
        r#""Punctuation: !@#$%^&*()""#,
        r#""Numbers: 123456789""#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn string_literal_with_newlines() {
    let cases = vec![
        "\"hello\nworld\"",
        "\"line1\nline2\nline3\"",
        "\"\n\"",              // just a newline
        "\"before\n\nafter\"", // double newline
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(
            result.is_ok(),
            "Should parse string with newlines: {:?}",
            result
        );
    }
}

#[test]
fn string_literal_with_unicode() {
    let cases = vec![
        r#""Hello 世界""#,
        r#""café""#,
        r#""😀🎉""#,
        r#""Здравствуй""#,
        r#""مرحبا""#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// RAW STRINGS WITH TILDES
// =============================================================================

#[test]
fn string_literal_with_quotes_using_tildes() {
    let cases = vec![
        r#"~"He said "hello""~"#,
        r#"~"She replied "hi""~"#,
        r#"~"Quote: ""~"#,
        r#"~"Multiple "quotes" in "string""~"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn string_literal_with_double_tildes() {
    let cases = vec![
        r#"~~"~"Contains "~ sequence"~~"#,
        r#"~~"~"Multiple "~ and "more"~~"#,
        r#"~~"Empty: "~~"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn string_literal_with_triple_tildes() {
    let cases = vec![
        r#"~~~"Contains "~~ sequence"~~~"#,
        r#"~~~"Has "~ and "~~ patterns"~~~"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn string_literal_all_tilde_levels() {
    let cases = vec![
        (r#""no tildes""#, "level 0"),
        (r#"~"one tilde"~"#, "level 1"),
        (r#"~~"two tildes"~~"#, "level 2"),
        (r#"~~~"three tildes"~~~"#, "level 3"),
        (r#"~~~~"four tildes"~~~~"#, "level 4"),
        (r#"~~~~~"five tildes"~~~~~"#, "level 5"),
    ];
    for (case, desc) in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(
            result.is_ok(),
            "Should parse {} '{}': {:?}",
            desc,
            case,
            result
        );
    }
}

// =============================================================================
// STRINGS IN LET STATEMENTS
// =============================================================================

#[test]
fn string_literal_in_let_statement() {
    let cases = vec![
        r#"let f() -> String = "hello";"#,
        r#"let name() -> String = "Alice";"#,
        r#"let msg() -> String = "Error: invalid input";"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn string_literal_with_quotes_in_let_statement() {
    let cases = vec![
        r#"let f() -> String = ~"He said "hello""~;"#,
        r#"let g() -> String = ~~"Contains "~ in it"~~;"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn string_literal_as_parameter() {
    let cases = vec![
        r#"let f(msg: String) -> Int = 0;"#,
        r#"let g(name: String, age: Int) -> String = "result";"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// STRINGS IN EXPRESSIONS
// =============================================================================

#[test]
fn string_literal_in_if_expression() {
    let cases = vec![
        r#"let f() -> String = if true { "yes" } else { "no" };"#,
        r#"let g(x: Int) -> String = if x > 0 { "positive" } else { "non-positive" };"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn string_literal_in_list() {
    let cases = vec![
        r#"let f() -> [String] = ["a", "b", "c"];"#,
        r#"let g() -> [String] = [];"#,
        r#"let h() -> [String] = [~"has "quotes""~, "normal"];"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn string_literal_empty() {
    let cases = vec![r#""""#, r#"~""~"#, r#"~~""~~"#];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(
            result.is_ok(),
            "Should parse empty string '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn string_literal_only_whitespace() {
    let cases = vec![r#"" ""#, r#""   ""#, "\"   \t  \n  \""];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(
            result.is_ok(),
            "Should parse whitespace-only string: {:?}",
            result
        );
    }
}

#[test]
fn string_literal_special_characters() {
    let cases = vec![
        r#""tab	here""#, // actual tab
        r#""backslash \ char""#,
        r#""newline
char""#, // actual newline
        r#""all kinds: \n \t \r""#, // literal backslash sequences (not interpreted)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(
            result.is_ok(),
            "Should parse special characters: {:?}",
            result
        );
    }
}

// =============================================================================
// NEGATIVE TESTS
// =============================================================================

#[test]
fn string_literal_rejects_mismatched_tildes() {
    let cases = vec![
        r#"~"missing closing tilde""#, // opened with ~ but closed without
        r#"~~"too few closing"~"#,     // opened with ~~ but closed with ~
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(
            result.is_err(),
            "Should reject mismatched tildes '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn string_literal_rejects_unclosed_strings() {
    let cases = vec![
        r#""unclosed"#,
        r#"~"unclosed~"#, // has opening tilde but no closing quote
        "\"no closing quote",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::string_literal, case);
        assert!(
            result.is_err(),
            "Should reject unclosed string '{}': {:?}",
            case,
            result
        );
    }
}
