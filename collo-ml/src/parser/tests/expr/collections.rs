use super::*;

// =============================================================================
// COLLECTION EXPRESSIONS
// =============================================================================
// Tests for:
// - List literals: [1, 2, 3]
// - List comprehensions: [x for x in collection where condition]
// - Global collections: @[Type]
// - Set operations: union, inter, \
// - Membership tests: x in collection

// =============================================================================
// LIST LITERALS
// =============================================================================

#[test]
fn collection_accepts_empty_list() {
    let result = ColloMLParser::parse(Rule::expr_complete, "[]");
    assert!(result.is_ok(), "Should parse empty list: {:?}", result);
}

#[test]
fn collection_accepts_single_element_list() {
    let cases = vec!["[1]", "[x]", "[student]", "[$V(x)]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_multi_element_lists() {
    let cases = vec![
        "[1, 2, 3]",
        "[x, y, z]",
        "[student, teacher, admin]",
        "[$V1(x), $V2(y), $V3(z)]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_nested_lists() {
    let cases = vec!["[[1, 2], [3, 4]]", "[[x, y], [z]]", "[[], [1], [2, 3]]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_lists_with_complex_elements() {
    let cases = vec![
        "[x + 1, y + 2]",
        "[$V(x) + 5, $V(y) - 3]",
        "[if flag { 1 } else { 2 }, 3]",
        "[compute(x), calculate(y)]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// LIST LITERALS
// =============================================================================

#[test]
fn collection_accepts_lists_range_with_numbers() {
    let cases = vec![
        "[1..2]",
        "[0..42]",
        "[-1..-3]",
        "[compute(x), calculate(y)]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_lists_range_with_expr() {
    let cases = vec![
        "[f(x)..g(x)]",
        "[student.age..room.number]",
        "[1..|@[Student]|]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// LIST COMPREHENSIONS
// =============================================================================

#[test]
fn collection_accepts_simple_comprehensions() {
    let cases = vec![
        "[x for x in @[Student]]",
        "[s for s in students]",
        "[n for n in numbers]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_comprehensions_with_transformations() {
    let cases = vec![
        "[x * 2 for x in numbers]",
        "[s.age for s in @[Student]]",
        "[$V(x) for x in [1, 2, 3]]",
        "[x + y for x in list]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_comprehensions_with_where_clause() {
    let cases = vec![
        "[s for s in @[Student] where s.age > 18]",
        "[x for x in numbers where x > 0]",
        "[s.age for s in @[Student] where s.is_active]",
        "[x * 2 for x in nums where x > 0 and x < 100]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_comprehensions_with_complex_conditions() {
    let cases = vec![
        "[x for x in @[Int] where x > 0 and x < 10]",
        "[s for s in @[Student] where s.is_active or s.is_enrolled]",
        "[x for x in list where not x.flag]",
        "[s for s in @[Student] where s.age >= 18 and s in eligible]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_comprehensions_with_variable_calls() {
    let cases = vec![
        "[$V(x) for x in @[Student]]",
        "[$Assigned(s) for s in @[Student]]",
        "[$InSlot(s, sl) for s in students]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_comprehension_precedence_with_where() {
    // The where clause should bind to the comprehension, not be outside
    let cases = vec![
        "[x + 1 for x in list where x > 0]", // where applies to comprehension
        "([x for x in list where x > 0]) union other", // comprehension is complete unit
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn list_comprehension_accepts_multiple_for_clauses() {
    let cases = vec![
        // Basic double for
        "[x for x in xs for y in ys]",
        "[a + b for a in list1 for b in list2]",
        // Triple for (nested iteration)
        "[x * y * z for x in xs for y in ys for z in zs]",
        // Mix of idents and expressions
        "[p.name for p in @[Person] for c in p.courses]",
        "[x + y.age for x in numbers for y in @[Student]]",
        // With where clause at the end
        "[x + y for x in xs for y in ys where x > 0]",
        "[name for person in people for name in person.names where person.age >= 18]",
        // Complex expression, multiple for, and where
        "[x + y.num + 1 for x in @[A] for y in x.bs where y.active and x.id == y.a_id]",
        // Real-world style example
        "[x.age + y.num for x in @[Student] for y in @[Class] where x in y.students]",
        // Trailing commas not allowed (but no comma in syntax), just valid spacing
        "[a for a in a_list for b in b_list   where a > b]",
    ];

    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Failed to parse valid comprehension: '{}'\nError: {:?}",
            case,
            result
        );
    }
}

#[test]
fn list_comprehension_rejects_missing_in_after_second_for() {
    let cases = vec![
        "[x for x in xs for y ys]",             // missing `in`
        "[x for x in xs for y in]",             // incomplete
        "[x for x in xs for y in for z in zs]", // too many 'for'
    ];

    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject invalid syntax: '{}'", case);
    }
}

#[test]
fn list_comprehension_rejects_multiple_where_clauses() {
    let invalid_cases = vec![
        // Two where clauses – classic mistake
        "[x for x in xs where x > 0 where x < 10]",
        // Multiple for + multiple where
        "[a + b for a in as for b in bs where a > 0 where b < 5]",
        // Where in the middle (allowed by accident?) + another where
        "[x for x in xs where x > 0 for y in ys where y.is_active]",
        // Triple where just to be sure
        "[i for i in items where i > 0 where i < 100 where i % 2 == 0]",
        // Where before any for (should also fail, but mainly testing duplicate where)
        "[x where x > 0 for x in xs where x < 10]",
        // Real-world-looking mistake
        "[x.age + y.num for x in @[Student] for y in @[Class] where x in y.students where y.active]",
    ];

    for case in invalid_cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);

        assert!(
            result.is_err(),
            "Parser should reject multiple 'where' clauses in: '{}'\nGot: {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// GLOBAL COLLECTIONS
// =============================================================================

#[test]
fn collection_accepts_global_collections() {
    let cases = vec![
        "@[Student]",
        "@[Week]",
        "@[Slot]",
        "@[Int]",
        "@[Bool]",
        "@[Subject]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_global_collections_in_parentheses() {
    let cases = vec!["(@[Student])", "((@[Week]))", "(@[Int] union @[Bool])"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_missing_brackets_in_global() {
    let cases = vec![
        "@Student",  // missing brackets
        "@[Student", // missing closing bracket
        "@Student]", // missing opening bracket
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing bracket): {:?}",
            case,
            result
        );
    }
}

#[test]
fn collection_rejects_nested_global_collections() {
    let cases = vec!["@[@[Student]]", "@[[@[Student]]]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (nested global): {:?}",
            case,
            result
        );
    }
}

#[test]
fn collection_accepts_global_collections_of_lists() {
    let cases = vec!["@[[Student]]", "@[[[Subject]]]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should not reject '{}' (global with lists): {:?}",
            case,
            result
        );
    }
}

// =============================================================================
// SET OPERATIONS
// =============================================================================

#[test]
fn collection_accepts_union() {
    let cases = vec![
        "a union b",
        "@[Student] union @[Teacher]",
        "morning_slots union afternoon_slots",
        "group1 union group2 union group3",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_intersection() {
    let cases = vec![
        "a inter b",
        "@[Student] inter active_students",
        "available_slots inter preferred_slots",
        "group1 inter group2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_set_difference() {
    let cases = vec![
        "a \\ b",
        "@[Subject] \\ pairing",
        "all_slots \\ occupied_slots",
        "@[Week] \\ holidays",
        "subject.slots \\ morning_slots",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_chained_union() {
    let cases = vec![
        "a union b union c",
        "@[Student] union @[Teacher] union @[Admin]",
        "group1 union group2 union group3 union group4",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_chained_intersection() {
    let cases = vec![
        "a inter b inter c",
        "@[All] inter @[Active] inter @[Available]",
        "group1 inter group2 inter group3",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_chained_difference() {
    // Set difference is not associative, so chaining is rejected
    let cases = vec!["a \\ b \\ c", "@[Subject] \\ pairing1 \\ pairing2"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (chained difference): {:?}",
            case,
            result
        );
    }
}

#[test]
fn collection_accepts_combined_set_operations() {
    let cases = vec![
        "a union b inter c",
        "(a union b) \\ c",
        "a union (b \\ c)",
        "(@[Subject] \\ pairing) union extra_subjects",
        "all_slots \\ (morning_slots union evening_slots)",
        "group_a union group_b inter group_c",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_deeply_nested_operations() {
    let cases = vec![
        "((a union b) \\ c) inter d",
        "(a union (b union c))",
        "@[Student] \\ (excluded union suspended)",
        "((@[All] \\ @[Excluded]) union @[Special]) inter @[Active]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_with_function_calls() {
    let cases = vec![
        "get_eligible_students()",
        "compute_available_slots(week)",
        "get_group_a() union get_group_b()",
        "@[Student] \\ get_excluded()",
        "get_all() inter get_active()",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// MEMBERSHIP TESTS
// =============================================================================

#[test]
fn collection_accepts_membership_tests() {
    let cases = vec![
        "x in collection",
        "student in @[Student]",
        "subject in pairing",
        "5 in [1, 2, 3, 4, 5]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_membership_with_complex_collections() {
    let cases = vec![
        "item in (@[Type] \\ excluded)",
        "slot in (morning_slots union afternoon_slots)",
        "x in (a inter b)",
        "student in (all_students \\ suspended)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_membership_in_logical_expressions() {
    let cases = vec![
        "x in collection and y > 5",
        "student in @[Student] or student in @[Teacher]",
        "not (x in excluded)",
        "x in set1 or x in set2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// COLLECTIONS IN CONTEXT
// =============================================================================

#[test]
fn collection_accepts_collections_in_aggregations() {
    let cases = vec![
        "sum s in @[Student] union @[Teacher] { $V(s) }",
        "forall s in @[Student] union @[Teacher] { $V(s) >= 0 }",
        "forall x in (group_a \\ excluded) { $V(x) == 1 }",
        "sum x in (a inter b) { $V(x) }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_accepts_collections_in_comprehensions() {
    let cases = vec![
        "[x for x in @[Student] union @[Teacher]]",
        "[s for s in (all \\ excluded) where s.active]",
        "[x for x in (a inter b)]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// NEGATIVE TESTS
// =============================================================================

#[test]
fn collection_rejects_incomplete_operations() {
    let cases = vec!["a \\", "\\ b", "a union", "union b", "a inter", "inter b"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (incomplete): {:?}",
            case,
            result
        );
    }
}

#[test]
fn collection_rejects_unclosed_brackets() {
    let cases = vec!["[1, 2, 3", "1, 2, 3]", "[@[Student]", "[x for x in @[S]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (unclosed bracket): {:?}",
            case,
            result
        );
    }
}

#[test]
fn collection_rejects_missing_for_in_comprehension() {
    let cases = vec!["[x x in list]", "[s where s.active]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing 'for'): {:?}",
            case,
            result
        );
    }
}

#[test]
fn collection_rejects_lists_range_with_incomplete_syntax() {
    let cases = vec!["[..]", "[0..]", "[..5]"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn collection_rejects_lists_range_with_bad_syntax() {
    let cases = vec![
        "[0...1]",    // Extra points
        "[0.1]",      // Missing point
        "[0..1, 42]", // Mixing with literals
        "0..1",       // Missing brackets
        "[0..1",      // Missing bracket
        "0..1]",      // Missing bracket
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}
