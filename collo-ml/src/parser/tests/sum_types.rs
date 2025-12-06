// =============================================================================
// SUM TYPE AND OPTION TYPE TESTS
// =============================================================================
// These tests validate sum type syntax (Type1 | Type2) and option type syntax (?Type)
// They test SYNTACTIC correctness only - semantic validation happens elsewhere

use super::*;

// =============================================================================
// OPTION TYPE SYNTAX - ?Type
// =============================================================================

#[test]
fn let_statement_with_option_primitive_types() {
    let cases = vec![
        "let f() -> ?Int = 5;",
        "let g() -> ?Bool = true;",
        "let h() -> ?LinExpr = 5;",
        "let i() -> ?Constraint = $V() === 0;",
        "let j() -> ?None = none;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_option_custom_types() {
    let cases = vec![
        "let f() -> ?Student = get_student();",
        "let g() -> ?Week = current_week();",
        "let h(maybe: ?Room) -> Int = 0;",
        "let i(x: ?CustomType) -> Bool = true;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_option_list_types() {
    let cases = vec![
        "let f() -> ?[Int] = [1, 2, 3];",
        "let g() -> ?[Student] = @[Student];",
        "let h() -> ?[[Int]] = [[1], [2]];",
        "let i(x: ?[Bool]) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_list_of_option_types() {
    let cases = vec![
        "let f() -> [?Int] = [1, none];",
        "let g() -> [?Student] = [];",
        "let h() -> [[?Bool]] = [[true], [none]];",
        "let i(x: [?CustomType]) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// SUM TYPE SYNTAX - Type1 | Type2
// =============================================================================

#[test]
fn let_statement_with_simple_sum_types() {
    let cases = vec![
        "let f() -> Int | Bool = 5;",
        "let g() -> Student | Teacher = get_person();",
        "let h() -> LinExpr | Int = compute();",
        "let i(x: Int | Bool) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_multiple_variant_sum_types() {
    let cases = vec![
        "let f() -> Int | Bool | LinExpr = 5;",
        "let g() -> Student | Teacher | Admin | Guest = get_person();",
        "let h(x: Int | Bool | Student | Week) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_sum_type_including_none() {
    let cases = vec![
        "let f() -> None | Int = 5;",
        "let g() -> Int | None = none;",
        "let h() -> None | Int | Bool = true;",
        "let i() -> Student | None | Teacher = get_person();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_sum_of_list_types() {
    let cases = vec![
        "let f() -> [Int] | [Bool] = [1, 2];",
        "let g() -> [Student] | [Teacher] = @[Student];",
        "let h(x: [Int] | [Bool] | [LinExpr]) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_list_of_sum_types() {
    let cases = vec![
        "let f() -> [Int | Bool] = [1, true];",
        "let g() -> [Student | Teacher] = [];",
        "let h() -> [[Int | Bool]] = [[1, true]];",
        "let i(x: [Int | Bool | LinExpr]) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// COMBINING OPTION AND SUM TYPES
// =============================================================================

#[test]
fn let_statement_option_is_sugar_for_sum_with_none() {
    // ?Type is equivalent to None | Type
    // Both should parse successfully (semantic equivalence checked elsewhere)
    let cases = vec![
        ("let f() -> ?Int = 5;", "let f() -> None | Int = 5;"),
        (
            "let g() -> ?Student = get();",
            "let g() -> None | Student = get();",
        ),
        (
            "let h(x: ?Bool) -> Int = 0;",
            "let h(x: None | Bool) -> Int = 0;",
        ),
    ];
    for (option_syntax, sum_syntax) in cases {
        let result1 = ColloMLParser::parse(Rule::let_statement_complete, option_syntax);
        let result2 = ColloMLParser::parse(Rule::let_statement_complete, sum_syntax);
        assert!(
            result1.is_ok(),
            "Should parse option syntax '{}': {:?}",
            option_syntax,
            result1
        );
        assert!(
            result2.is_ok(),
            "Should parse sum syntax '{}': {:?}",
            sum_syntax,
            result2
        );
    }
}

// =============================================================================
// COMPLEX COMBINATIONS
// =============================================================================

#[test]
fn let_statement_with_complex_nested_types() {
    let cases = vec![
        // Option of list of sum
        "let f() -> ?[Int | Bool] = [1, true];",
        // Sum of options
        "let h() -> ?Int | ?Bool = 5;",
        // Sum including option and non-option
        "let i() -> ?Int | Bool = true;",
        // Deep nesting
        "let j() -> [[Int | Bool] | [LinExpr]] = [];",
        "let k() -> ?[?[Int]] = [[none]];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse complex type '{}': {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// GLOB LISTS WITH SUM TYPES
// =============================================================================

#[test]
fn let_statement_with_glob_list_of_sum_types() {
    let cases = vec![
        "let f() -> [Student | Teacher] = @[Student | Teacher];",
        "let g() -> [Int] = @[Person | Room];", // Type mismatch but syntactically valid
        "let h() -> Bool = x in @[Student | Teacher];",
        "let i() -> LinExpr = sum p in @[Student | Teacher] { $V(p) };",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse glob with sum type '{}': {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// SYNTACTICALLY VALID BUT SEMANTICALLY INVALID
// =============================================================================
// These parse successfully but should be rejected during semantic analysis

#[test]
fn let_statement_accepts_multiple_question_marks() {
    // Semantically invalid (??Type should be rejected), but grammatically valid
    let cases = vec![
        "let f() -> ??Int = 5;",
        "let g() -> ???Student = get();",
        "let h(x: ??Bool) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (grammatically valid, semantically wrong): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_accepts_option_in_sum() {
    // Semantically invalid (?Type1 | Type2 should be None | Type1 | Type2)
    // but grammatically valid
    let cases = vec![
        "let f() -> ?Int | Bool = 5;",
        "let g() -> Int | ?Bool = true;",
        "let h() -> ?Student | ?Teacher = get();",
        "let i(x: ?Int | Bool | ?LinExpr) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (grammatically valid, semantically wrong): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_accepts_duplicate_types_in_sum() {
    // Semantically invalid (Int | Int should be just Int), but grammatically valid
    let cases = vec![
        "let f() -> Int | Int = 5;",
        "let g() -> Student | Student | Student = get();",
        "let h() -> Int | Bool | Int = 0;",
        "let i(x: None | None) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (grammatically valid, semantically wrong): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_accepts_redundant_option_with_none() {
    // Semantically redundant (?Int is None | Int, so ?Int | None is None | Int | None)
    // but grammatically valid
    let cases = vec![
        "let f() -> ?Int | None = 5;",
        "let g() -> None | ?Student = none;",
        "let h(x: ?Bool | None | Int) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (grammatically valid, semantically redundant): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// REALISTIC EXAMPLES WITH SUM TYPES
// =============================================================================

#[test]
fn let_statement_realistic_sum_type_examples() {
    let cases = vec![
        // Function that might return different types
        "let find_person(id: Int) -> Student | Teacher | None = lookup(id);",

        // Parameter accepting multiple types
        "let process(entity: Student | Teacher) -> Bool = entity.active;",

        // Optional result
        "let get_room(id: Int) -> ?Room = find_room(id);",

        // List of mixed entities
        "let all_people() -> [Student | Teacher] = @[Student | Teacher];",

        // Complex aggregation
        "let count_all() -> Int = |@[Student | Teacher]|;",

        // Working with optional lists
        "let get_students(week: ?Week) -> [Student] = if week == none { [] } else { get_for_week(week) };",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse realistic example '{}': {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// WHITESPACE AND FORMATTING
// =============================================================================

#[test]
fn let_statement_sum_types_with_varied_whitespace() {
    let cases = vec![
        "let f() -> Int|Bool = 5;",             // no spaces
        "let g() -> Int | Bool = 5;",           // normal spaces
        "let h() -> Int  |  Bool = 5;",         // extra spaces
        "let i() -> Int\n|\nBool = 5;",         // newlines
        "let j() -> Int|Bool|LinExpr = 5;",     // multiple, no spaces
        "let k() -> Int | Bool | LinExpr = 5;", // multiple, with spaces
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse with varied whitespace '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_option_types_with_varied_whitespace() {
    let cases = vec![
        "let f() -> ?Int = 5;",   // normal
        "let g() -> ? Int = 5;",  // space after ?
        "let h() -> ?  Int = 5;", // extra space after ?
        "let i() -> ?\nInt = 5;", // newline after ?
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option with varied whitespace '{}': {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// NEGATIVE TESTS - INVALID SYNTAX
// =============================================================================

#[test]
fn let_statement_rejects_malformed_sum_types() {
    let cases = vec![
        "let f() -> Int | = 5;",        // trailing |
        "let g() -> | Int = 5;",        // leading |
        "let h() -> Int || Bool = 5;",  // double ||
        "let i() -> Int | | Bool = 5;", // separated ||
        "let j() -> (Int | Bool) = 5;", // parentheses (not supported)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject malformed sum type '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_rejects_question_mark_in_wrong_position() {
    let cases = vec![
        "let f() -> Int? = 5;",    // postfix ? (not supported)
        "let g() -> [Int]? = [];", // postfix ? on list
        "let h() -> (? Int) = 5;", // parentheses with ?
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject question mark in wrong position '{}': {:?}",
            case,
            result
        );
    }
}
