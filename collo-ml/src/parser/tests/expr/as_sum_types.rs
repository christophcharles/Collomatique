use super::*;

// =============================================================================
// TYPE ANNOTATION WITH SUM TYPES AND OPTION TYPES
// =============================================================================
// Tests for 'as' operator with sum type syntax (Type1 | Type2) and option syntax (?Type)

#[test]
fn type_annotation_option_types() {
    let cases = vec![
        "x as ?Int",
        "value as ?Bool",
        "student as ?Student",
        "[] as ?[Int]",
        "compute() as ?LinExpr",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option type annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_option_of_list_types() {
    let cases = vec![
        "[] as ?[Int]",
        "students as ?[Student]",
        "matrix as ?[[Int]]",
        "deep as ?[[[Bool]]]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option list annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_list_of_option_types() {
    let cases = vec![
        "[] as [?Int]",
        "[none, 5] as [?Int]",
        "values as [?Student]",
        "nested as [[?Bool]]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse list of option annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_simple_sum_types() {
    let cases = vec![
        "x as Int | Bool",
        "person as Student | Teacher",
        "value as LinExpr | Int",
        "entity as Room | Week",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum type annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_sum_types_with_multiple_variants() {
    let cases = vec![
        "x as Int | Bool | LinExpr",
        "person as Student | Teacher | Admin",
        "value as Int | Bool | Student | Week",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse multi-variant sum annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_sum_types_with_none() {
    let cases = vec![
        "x as None | Int",
        "value as Int | None",
        "person as None | Student | Teacher",
        "data as Int | Bool | None",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum with None annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_sum_of_list_types() {
    let cases = vec![
        "[] as [Int] | [Bool]",
        "collection as [Student] | [Teacher]",
        "lists as [Int] | [Bool] | [LinExpr]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum of lists annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_list_of_sum_types() {
    let cases = vec![
        "[] as [Int | Bool]",
        "[1, true] as [Int | Bool]",
        "people as [Student | Teacher]",
        "matrix as [[Int | Bool]]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse list of sum annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_complex_nested_sum_option_types() {
    let cases = vec![
        "[] as ?[Int | Bool]",
        "value as ?Int | Bool",
        "data as ?Int | ?Bool",
        "nested as [[Int | Bool] | [LinExpr]]",
        "deep as ?[?[Int]]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse complex nested annotation '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_sum_types_in_expressions() {
    let cases = vec![
        "(x as Int | Bool) + 5",
        "[entity for entity in (people as [Student | Teacher])]",
        "if flag { value as Int | Bool } else { 0 }",
        "sum x in ([] as [Int | Bool]) { x }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum type in expression '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_option_types_in_expressions() {
    let cases = vec![
        "(maybe_x as ?Int) + 5",
        "if present { value as ?Student } else { none }",
        "[x for x in (students as ?[Student])]",
        "|(maybe_list as ?[Int])|",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option in expression '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_sum_types_with_parentheses() {
    let cases = vec![
        "((x as Int | Bool))",
        "(value as Student | Teacher) + entity",
        "compute((x as Int | Bool), (y as Bool | LinExpr))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum with parens '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_option_types_with_parentheses() {
    let cases = vec![
        "((x as ?Int))",
        "(value as ?Student)",
        "func((x as ?Int), (y as ?Bool))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option with parens '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_sum_types_in_function_args() {
    let cases = vec![
        "process(x as Int | Bool)",
        "handle(entity as Student | Teacher, flag as Bool)",
        "compute((x as Int | Bool), (y as LinExpr | Int))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum in function args '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_option_types_in_function_args() {
    let cases = vec![
        "process(x as ?Int)",
        "handle(student as ?Student, week as ?Week)",
        "compute((x as ?Int), (y as ?Bool))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option in function args '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_precedence_with_sum_types() {
    // 'as' binds tighter than arithmetic, but how does it work with |?
    // Answer: The | is part of the type_name, so "x as Int | Bool" parses as "x as (Int | Bool)"
    let cases = vec![
        "x as Int | Bool + 5",               // (x as (Int | Bool)) + 5
        "2 * value as Int | Bool",           // 2 * (value as (Int | Bool))
        "a as Int | Bool + b as Bool | Int", // (a as (Int | Bool)) + (b as (Bool | Int))
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse precedence with sum '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_precedence_with_option_types() {
    let cases = vec![
        "x as ?Int + 5",         // (x as ?Int) + 5
        "2 * value as ?Bool",    // 2 * (value as ?Bool)
        "a as ?Int + b as ?Int", // (a as ?Int) + (b as ?Int)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse precedence with option '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_in_list_literals_with_sum_types() {
    let cases = vec![
        "[x as Int | Bool, y as Int | Bool]",
        "[([] as [Int | Bool]), [1, true]]",
        "[entity as Student | Teacher, another as Student | Teacher]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum in list literals '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_in_list_literals_with_option_types() {
    let cases = vec![
        "[x as ?Int, y as ?Int]",
        "[([] as ?[Int]), [1, 2]]",
        "[student as ?Student, teacher as ?Teacher]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option in list literals '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_with_constraints_and_sum_types() {
    let cases = vec![
        "$V(x as Int | Bool) <== 10",
        "sum x in ([] as [Student | Teacher]) { $V(x) } >== 0",
        "(value as Int | LinExpr) === 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum in constraints '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_with_constraints_and_option_types() {
    let cases = vec![
        "$V(x as ?Int) <== 10",
        "sum x in ([] as ?[Student]) { $V(x) } >== 0",
        "(value as ?LinExpr) === 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option in constraints '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_with_varied_whitespace_sum_types() {
    let cases = vec![
        "x as Int|Bool",             // no spaces around |
        "x as Int | Bool",           // normal spaces
        "x as Int  |  Bool",         // extra spaces
        "x as Int\n|\nBool",         // newlines
        "x as Int|Bool|LinExpr",     // multiple, no spaces
        "x as Int | Bool | LinExpr", // multiple, with spaces
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse sum with varied whitespace '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_with_varied_whitespace_option_types() {
    let cases = vec![
        "x as ?Int",   // normal
        "x as ? Int",  // space after ?
        "x as ?  Int", // extra space
        "x as ?\nInt", // newline after ?
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse option with varied whitespace '{}': {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// SEMANTICALLY INVALID BUT SYNTACTICALLY VALID
// =============================================================================

#[test]
fn type_annotation_accepts_multiple_question_marks() {
    // Semantically invalid, but grammatically valid
    let cases = vec!["x as ??Int", "value as ???Student", "[] as ??[Int]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse (syntactically valid, semantically wrong) '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_accepts_option_in_sum() {
    // ?Type1 | Type2 should be None | Type1 | Type2 semantically
    let cases = vec![
        "x as ?Int | Bool",
        "y as Int | ?Bool",
        "z as ?Student | ?Teacher",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse (syntactically valid, semantically wrong) '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_accepts_duplicate_types_in_sum() {
    let cases = vec![
        "x as Int | Int",
        "y as Student | Student | Student",
        "z as Int | Bool | Int",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse (syntactically valid, semantically redundant) '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_accepts_redundant_option_with_none() {
    let cases = vec![
        "x as ?Int | None",
        "y as None | ?Student",
        "z as ?Bool | None | Int",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse (syntactically valid, semantically redundant) '{}': {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// REALISTIC EXAMPLES
// =============================================================================

#[test]
fn type_annotation_realistic_sum_type_examples() {
    let cases = vec![
        // Disambiguating empty list types
        "[] as [Student | Teacher]",
        // Explicit coercion for mixed entity operations
        "sum entity in (all_people as [Student | Teacher]) { $V(entity) }",
        // Converting to optional
        "if found { result } else { none as ?Student }",
        // Type annotation in complex expressions
        "if flag { compute() as Int | LinExpr } else { 0 as Int | LinExpr }",
        // Glob list coercion
        "@[Student | Teacher] as [Student | Teacher]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse realistic example '{}': {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// NEGATIVE TESTS
// =============================================================================

#[test]
fn type_annotation_rejects_malformed_sum_types() {
    let cases = vec![
        "x as Int |",        // trailing |
        "x as | Int",        // leading |
        "x as Int || Bool",  // double ||
        "x as (Int | Bool)", // parentheses in type (not supported)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject malformed sum type '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn type_annotation_rejects_postfix_question_mark() {
    let cases = vec![
        "x as Int?",     // postfix ? (not supported)
        "x as [Int]?",   // postfix ? on list
        "x as Student?", // postfix ? on custom type
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject postfix question mark '{}': {:?}",
            case,
            result
        );
    }
}
