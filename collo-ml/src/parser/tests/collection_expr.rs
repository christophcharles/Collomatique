use super::*;

#[test]
fn collection_accepts_global_sets() {
    let cases = vec![
        "@[Student]",
        "@[Subject]",
        "@[Week]",
        "@[Slot]",
        "@[Int]",
        "@[Bool]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_paths() {
    let cases = vec![
        "subject.slots",
        "student.courses",
        "pairing",
        "pairings_list",
        "teacher.available_slots",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_set_difference() {
    let cases = vec![
        "@[Subject] \\ pairing",
        "all_slots \\ occupied_slots",
        "@[Week] \\ holidays",
        "subject.slots \\ morning_slots",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_set_union() {
    let cases = vec![
        "morning_slots union afternoon_slots",
        "@[Student] union @[Teacher]",
        "group1 union group2 union group3",
        "pairing1 union pairing2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_set_intersection() {
    let cases = vec![
        "available_slots inter preferred_slots",
        "@[Student] inter active_students",
        "group1 inter group2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_parentheses() {
    let cases = vec![
        "(@[Subject])",
        "(pairing)",
        "(@[Subject] \\ pairing)",
        "(group1 union group2)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_complex_operations() {
    let cases = vec![
        "(@[Subject] \\ pairing1) \\ pairing2", // multiple differences
        "@[Subject] \\ (pairing1 \\ pairing2)", // multiple differences
        "a union b union c",                    // multiple unions
        "a inter b inter c",                    // multiple intersections
        "(a union b) \\ c",                     // parentheses with operations
        "a union (b \\ c)",                     // nested operations
        "(@[Subject] \\ pairing) union extra_subjects",
        "all_slots \\ (morning_slots union evening_slots)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_missing_brackets_in_global() {
    let cases = vec![
        "@Student",   // missing brackets
        "@[Student",  // missing closing bracket
        "@Student]",  // missing opening bracket
        "[@Student]", // missing @
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_invalid_operators() {
    let cases = vec![
        "a + b",   // wrong operator
        "a - b",   // wrong operator (use \\ for difference)
        "a & b",   // wrong operator
        "a | b",   // wrong operator
        "a and b", // logical operator, not set operator
        "a or b",  // logical operator, not set operator
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_incomplete_expressions() {
    let cases = vec![
        "a \\",        // missing right operand
        "\\ b",        // missing left operand
        "union b",     // missing left operand
        "a union",     // missing right operand
        "a inter",     // missing right operand
        "(@[Subject]", // unclosed parenthesis
        "@[Subject])", // unmatched parenthesis
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_nested_global_sets() {
    let cases = vec![
        "@[@[Student]]", // can't nest global sets
        "@[[Subject]]",  // wrong syntax
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_numbers_and_strings() {
    let cases = vec![
        "42",           // number, not collection
        "\"students\"", // string, not collection
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_multiple_differences() {
    let cases = vec![
        "@[Subject] \\ pairing1 \\ pairing2", // multiple differences
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_deeply_nested_operations() {
    let cases = vec![
        "((a union b) \\ c) inter d",
        "(a union (b union c))",
        "@[Student] \\ (excluded union suspended)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::collection_expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
