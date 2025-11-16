use super::*;

// ========== Basic Expression Tests ==========

#[test]
fn expr_accepts_simple_variables() {
    let cases = vec![
        "$StudentInSlot(student, slot, week)",
        "$HasSubject(subject, student, week)",
        "$Var(x)",
        "$V(a, b, c)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_constants() {
    let cases = vec!["0", "1", "42", "-5", "-100"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_coefficient_times_variable() {
    let cases = vec![
        "2 * $Var(x)",
        "-1 * $StudentInSlot(s, sl, w)",
        "5 * $HasSubject(subj, stud, week)",
        "student.weight * $Assigned(student)",
        "(a + b) * $Var(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_addition_and_subtraction() {
    let cases = vec![
        "$V1(x) + $V2(y)",
        "$V1(x) - $V2(y)",
        "$V1(x) + $V2(y) + $V3(z)",
        "$V1(x) - $V2(y) + $V3(z)",
        "5 + $Var(x)",
        "$Var(x) + 10",
        "$Var(x) - 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_complex_linear_combinations() {
    let cases = vec![
        "2 * $V1(x) + 3 * $V2(y)",
        "$V1(x) + 2 * $V2(y) - $V3(z)",
        "10 + 2 * $V1(x) - 3 * $V2(y) + 5",
        "-1 * $V1(x) + -1 * $V2(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_sum() {
    let cases = vec![
        "sum x in @[Student] { $Var(x) }",
        "sum slot in subject.slots { $StudentInSlot(student, slot, week) }",
        "sum s in collection { 2 * $V(s) }",
        "sum x in set { $V1(x) + $V2(x) }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_sum_with_where() {
    let cases = vec![
        "sum x in @[Student] where x.is_active { $Var(x) }",
        "sum slot in slots where slot.hour > 8 { $StudentInSlot(s, slot, w) }",
        "sum x in collection where x.value > 0 and x.flag { $V(x) }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_nested_sums() {
    let cases = vec![
        "sum x in @[X] { sum y in @[Y] { $Var(x, y) } }",
        "sum student in @[Student] { sum week in @[Week] { $HasSubject(math, student, week) } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_sum_in_larger_expression() {
    let cases = vec![
        "(sum x in @[X] { $V(x) }) + 10",
        "2 * (sum x in @[X] { $V(x) })",
        "(sum x in @[X] { $V(x) }) - (sum y in @[Y] { $V(y) })",
        // Note: This is now unambiguous!
        "sum x in @[X] { $V(x) + 5 }", // 5 is inside the sum (in braces)
        "sum x in @[X] { $V(x) } + 5", // 5 is outside the sum
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_if_expressions() {
    let cases = vec![
        "if x > 5 { $V1(x) } else { $V2(x) }",
        "if condition { 10 } else { 20 }",
        "if flag { sum x in @[X] { $V(x) } } else { 0 }",
        "2 * (if x { $V(x) } else { $W(x) })",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_not_on_variables() {
    let cases = vec![
        "1 - $Var(x)", // This is how 'not' is expressed
        "(1 - $HasSubject(subject, student, week))",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_variable_times_variable() {
    let cases = vec![
        "$V1(x) * $V2(y)",   // Non-linear!
        "$Var(x) * $Var(x)", // Non-linear!
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_ok(),
            "Should parse '{}' (non-linear): {:?}",
            case,
            result
        );
    }
}

#[test]
fn expr_accepts_division_and_modulo_in_coefficients() {
    // Now that we're unified, division/modulo are allowed in computable contexts
    let cases = vec![
        "(10 // 2) * $Var(x)", // coefficient with division
        "(x % 3) * $Var(y)",   // coefficient with modulo
        "x // 2 + 5",          // pure computable
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_logical_operators() {
    // These now parse (they're boolean/constraint operations)
    let cases = vec![
        "$V1(x) >= 0 and $V2(y) <= 10",
        "x > 5 or y < 3",
        "not (x == y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_function_calls_in_multiplication() {
    let cases = vec![
        // Function call alone
        "compute_value(student, week)",
        // Function call in multiplication (now allowed on both sides)
        "2 * compute_value(student, week)",
        "compute_value(x) * 3", // Now OK!
        "student.weight * get_coefficient(x, y)",
        "(a + b) * calculate(x, y)",
        // In larger expressions
        "$Var(x) + 2 * compute_value(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_complex_computable_as_coefficients() {
    let cases = vec![
        "(2 + 3) * $Var(x)",
        "(10 - 5) * $V(y)",
        "(a * 2) * $V(x)",
        "(if flag { 2 } else { 3 }) * $Var(x)",
        "(|@[Student]|) * $Var(x)",
        "(student.weight) * $Assigned(student)",
        "((a + b) * 2) * $Var(x)",
        "(|@[Student]| // 2) * $Var(x)",
        "(week % 4 + 1) * $V(week)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_complex_expressions_as_terms() {
    let cases = vec![
        "$Var(x) + (a + b)",
        "$Var(x) - (10 - 5)",
        "$Var(x) + if flag { 10 } else { 20 }",
        "$Var(x) + |@[Student]|",
        "|collection| + $Var(x) + |other_collection|",
        "$Var(x) + if flag { |@[X]| } else { 0 } + $Var(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_pure_arithmetic() {
    let cases = vec![
        "2 * 3",
        "2 * 3 + 4",
        "10 // 2 + 5",
        "x * y",
        "x * y + z",
        "(a + b) * (c - d)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== List Literal Tests ==========

#[test]
fn expr_accepts_empty_list() {
    let result = ColloMLParser::parse(Rule::expr_complete, "[]");
    assert!(result.is_ok(), "Should parse empty list: {:?}", result);
}

#[test]
fn expr_accepts_list_literals() {
    let cases = vec![
        "[1, 2, 3]",
        "[student, teacher, admin]",
        "[x, y, z]",
        "[1]",
        "[[1, 2], [3, 4]]", // nested lists
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== List Comprehension Tests ==========

#[test]
fn expr_accepts_simple_comprehensions() {
    let cases = vec![
        "[x for x in @[Student]]",
        "[s.age for s in @[Student]]",
        "[x * 2 for x in numbers]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_comprehensions_with_where() {
    let cases = vec![
        "[s for s in @[Student] where s.age > 18]",
        "[s.age for s in @[Student] where s.is_active]",
        "[x * 2 for x in nums where x > 0 and x < 100]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Collection Operation Tests ==========

#[test]
fn expr_accepts_collection_operations() {
    let cases = vec![
        "@[Student] union @[Teacher]",
        "@[Active] inter @[Available]",
        "@[All] \\ @[Excluded]",
        "group_a union group_b inter group_c",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_collections_in_larger_context() {
    let cases = vec![
        "forall s in @[Student] union @[Teacher] { $V(s) >= 0 }",
        "|@[Student] \\ excluded|",
        "sum s in group_a union group_b { $V(s) }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_simple_comparisons() {
    let cases = vec![
        "$Var(x) <= 10",
        "$Var(x) >= 0",
        "$Var(x) == 1",
        "$V1(x) + $V2(y) <= 5",
        "sum x in @[X] { $V(x) } >= 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_complex_linear_expressions_in_comparisons() {
    let cases = vec![
        "2 * $V1(x) + 3 * $V2(y) <= 10",
        "$V1(x) - $V2(y) + 5 >= 0",
        "(sum x in @[X] { $V(x) }) == |@[X]|",
        "(if flag { $V1(x) } else { $V2(x) }) <= 10",
        "$V(x) + compute_value(y) >= 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_and_operator() {
    let cases = vec![
        "$V1(x) <= 10 and $V2(y) >= 0",
        "$V1(x) <= 10 && $V2(y) >= 0",
        "$V(x) == 1 and $V(y) == 1 and $V(z) == 1",
        "sum x in @[X] { $V(x) } >= 1 and sum y in @[Y] { $V(y) } <= 10",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_forall() {
    let cases = vec![
        "forall x in @[Student] { $Assigned(x) == 1 }",
        "forall week in @[Week] { sum slot in slots { $Used(slot, week) } <= 10 }",
        "forall s in collection { $V(s) >= 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_forall_with_where() {
    let cases = vec![
        "forall x in @[Student] where x.is_active { $Assigned(x) == 1 }",
        "forall week in @[Week] where week.number > 10 { $V(week) <= 5 }",
        "forall s in collection where s.valid and s.flag { $V(s) >= 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_nested_forall() {
    let cases = vec![
        "forall x in @[X] { forall y in @[Y] { $V(x, y) <= 1 } }",
        "forall student in @[Student] { forall week in @[Week] { sum slot in slots { $Assigned(student, slot, week) } <= 1 } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_forall_with_and() {
    let cases = vec![
        "forall x in @[X] { $V1(x) >= 0 and $V2(x) <= 10 }",
        "forall s in @[Student] { $Assigned(s) == 1 and sum w in @[Week] { $HasColle(s, w) } >= 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_if_constraint_expressions() {
    let cases = vec![
        "if flag { $V1(x) <= 10 } else { $V2(x) <= 10 }",
        "if x > 5 { $V(x) == 1 } else { $V(x) == 0 }",
        "if condition { forall y in @[Y] { $V(y) >= 0 } } else { $V(z) == 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_function_calls_with_and() {
    let cases = vec![
        "enforce_rule1(x) and enforce_rule2(y)",
        "$V(x) <= 10 and apply_constraint(x, y)",
        "forall x in @[X] { check_rule(x) and $V(x) >= 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_complex_constraint_combinations() {
    let cases = vec![
        "forall student in @[Student] { forall week in @[Week] { (sum slot in subject.slots { $StudentInSlot(student, slot, week) }) <= 1 and $HasSubject(subject, student, week) == 1 } }",
        "(forall x in @[X] { $V(x) >= 0 }) and (forall y in @[Y] { $V(y) <= 10 })",
        "if flag { forall x in @[X] { $V(x) == 1 } } else { sum x in @[X] { $V(x) } >= 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_or_operator() {
    // Now or is allowed (it's part of unified expressions)
    let cases = vec![
        "$V1(x) <= 10 or $V2(y) >= 0",
        "$V(x) == 1 || $V(y) == 1",
        "x > 5 or y < 3",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_not_operator() {
    // Now not is allowed (it's part of unified expressions)
    let cases = vec!["not ($V(x) <= 10)", "!(x == 1)", "not x or not y"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_exists() {
    let cases = vec!["exists x in @[X] { $V(x) == 1 }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (exists not implemented): {:?}",
            case,
            result
        );
    }
}

#[test]
fn expr_accepts_all_comparison_operators() {
    // Now all comparison operators are allowed (type-checking will determine validity)
    let cases = vec![
        "$V(x) < 10",
        "$V(x) > 0",
        "$V(x) != 1",
        "$V(x) <= 10",
        "$V(x) >= 0",
        "$V(x) == 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_arithmetic_without_comparison() {
    // Now these parse (they're LinExpr, type-checking determines if context allows)
    let cases = vec!["$V(x) + $V(y)", "2 * $V(x)", "sum x in @[X] { $V(x) }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_incomplete_constraint_expressions() {
    let cases = vec![
        "$V(x) <=",
        "forall x in @[X] {",          // missing closing brace
        "forall x in @[X] $V(x) <= 1", // missing braces
        "$V(x) <= 10 and",
        "and $V(x) >= 0",
        "($V(x) <= 10",
        "$V(x) <= 10)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

// ========== Numbers and Paths ==========

#[test]
fn expr_accepts_simple_numbers() {
    let cases = vec!["42", "-17", "0"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_paths() {
    let cases = vec![
        "student.age",
        "course.duration",
        "x.y.z",
        "student.is_active",
        "flag",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Arithmetic Operations ==========

#[test]
fn expr_accepts_arithmetic() {
    let cases = vec![
        "5 + 3",
        "10 - 7",
        "3 * 4",
        "5 + 3 * 2",
        "(5 + 3) * 2",
        "student.age + 5",
        "10 // 3",
        "10 % 3",
        "student.age // 10",
        "week_number % 2",
        "(10 + 5) // 3",
        "|@[Week]| % 4",
        "5 * 3 // 2",
        "10 // 2 % 3",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_cardinality() {
    let cases = vec![
        "|@[Student]|",
        "|pairing|",
        "|subject.slots|",
        "|@[Student]| + 1",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Comparisons ==========

#[test]
fn expr_accepts_comparisons() {
    let cases = vec![
        "x == y",
        "student.age > 18",
        "count < 10",
        "a >= b",
        "x <= y",
        "name != other_name",
        "5 == 5",
        "student.group.size > 0",
        "$Var(x) > 5",
        "$V1(x) == $V2(y)",
        "$Var(x) <= 10",
        "x == y", // comparing computables
        "5 < 10", // comparing numbers
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_arithmetic_in_comparisons() {
    let cases = vec![
        "x + 5 > 10",
        "student.age * 2 == 36",
        "|@[Student]| > 0",
        "(a + b) <= (c * 2)",
        "x // 2 == 3",
        "week % 2 == 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_in_tests() {
    let cases = vec![
        "subject in pairing",
        "student in @[Student]",
        "x in collection",
        "item in (@[Type] \\ excluded)",
        "slot in (morning_slots union afternoon_slots)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Boolean Operations ==========

#[test]
fn expr_accepts_logical_and() {
    let cases = vec![
        "x > 0 and y > 0",
        "x > 0 && y > 0",
        "student.is_active and student.age > 18",
        "a == b and c == d and e == f",
        "x in collection and y > 5",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_logical_or() {
    let cases = vec![
        "x > 0 or y > 0",
        "x > 0 || y > 0",
        "student.is_french or student.is_german",
        "a == b or c == d or e == f",
        "x in set1 or x in set2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_logical_not() {
    let cases = vec![
        "not x",
        "!x",
        "not student.is_active",
        "!(x > 5)",
        "not (a and b)",
        "not not x",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== If Expressions ==========

#[test]
fn expr_accepts_simple_if() {
    let cases = vec![
        "if x > 5 { 10 } else { 20 }",
        "if student.is_active { 1 } else { 0 }",
        "if flag { 100 } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_if_with_paths() {
    let cases = vec![
        "if condition { student.age } else { 0 }",
        "if x { a.value } else { b.value }",
        "if flag { |collection| } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_if_with_arithmetic() {
    let cases = vec![
        "if x > 0 { x + 5 } else { 0 }",
        "if condition { a * 2 } else { b * 3 }",
        "if flag { (x + y) * 2 } else { x - y }",
        "if x { |collection| + 1 } else { |collection| - 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_if_with_boolean_branches() {
    let cases = vec![
        "if flag { x > 5 } else { y > 10 }",
        "if condition { a and b } else { c or d }",
        "if x { student.is_active } else { false_flag }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_nested_if() {
    let cases = vec![
        "if x { if y { 1 } else { 2 } } else { 3 }",
        "if a { 10 } else { if b { 20 } else { 30 } }",
        "if x > 0 { if y > 0 { x + y } else { x } } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_if_in_arithmetic() {
    let cases = vec![
        "(if x { 10 } else { 20 }) + 5",
        "2 * (if flag { a } else { b })",
        "if x { 1 } else { 2 } + if y { 3 } else { 4 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_complex_conditions_in_if() {
    let cases = vec![
        "if x > 5 and y > 10 { 100 } else { 0 }",
        "if a in collection or b in collection { 1 } else { 0 }",
        "if not flag { x } else { y }",
        "if (x > 0 and y > 0) or z > 0 { 1 } else { 0 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Parentheses ==========

#[test]
fn expr_accepts_parentheses() {
    let cases = vec![
        "(x > 5)",
        "(student.is_active)",
        "((x > 5))",
        "(x > 5) and (y > 5)",
        "(5 + 3)",
        "((a + b) * c)",
        "($Var(x))",
        "(2 * $Var(x))",
        "($V1(x) + $V2(y))",
        "((($Var(x))))",
        "(sum x in @[X] { $V(x) })",
        "($V(x) <= 10)",
        "(forall x in @[X] { $V(x) >= 0 })",
        "($V1(x) <= 10) and ($V2(y) >= 0)",
        "((($V(x) == 1)))",
    ];

    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Complex Combinations ==========

#[test]
fn expr_accepts_complex_combinations() {
    let cases = vec![
        "x > 0 and y > 0 or z > 0",
        "(x > 0 and y > 0) or z > 0",
        "x > 0 and (y > 0 or z > 0)",
        "not (x > 0 and y > 0)",
        "a in set1 and not (b in set2)",
        "x == 5 or (y > 10 and z < 20)",
        "student.is_active and student.age > 18 and student in @[Student]",
        "(a + b) * 2 > 10 and x or y",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Rejections ==========

#[test]
fn expr_rejects_single_slash_division() {
    let result = ColloMLParser::parse(Rule::expr_complete, "10 / 2");
    assert!(result.is_err(), "Should not parse single slash division");
}

#[test]
fn expr_accepts_variables() {
    // Now variables are part of unified expressions
    let cases = vec!["$StudentInSlot(s, sl, w)", "$HasSubject(subj, stud, week)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_assignment() {
    let cases = vec!["x = 5"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_incomplete_expressions() {
    let cases = vec![
        "$Var(x) +",
        "+ $Var(x)",
        "2 * ",
        " * $Var(x)",
        "sum x in @[X] {",       // missing closing brace
        "sum x in @[X] $Var(x)", // missing braces entirely
        "($Var(x)",
        "$Var(x))",
        "5 +",
        "* 3",
        "(5 + 3",
        "10 //",
        "x // // y",
        "// 5",
        "x >",
        "> 5",
        "x and",
        "or y",
        "not",
        "x in",
        "in collection",
        "x > 5)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_invalid_operators() {
    let cases = vec!["x & y", "x | y", "x ^ y", "x <> y"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_if_without_else() {
    let cases = vec!["if x > 5 { 10 }", "if condition { value }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing else): {:?}",
            case,
            result
        );
    }
}

#[test]
fn expr_rejects_if_with_mismatched_braces() {
    let cases = vec![
        "if x { 10 } else 20 }",
        "if x { 10 else { 20 }",
        "if x 10 } else { 20 }",
        "if x { 10 } else { 20",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_if_without_condition() {
    let cases = vec!["if { 10 } else { 20 }"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_if_with_empty_branches() {
    let cases = vec![
        "if x > 5 { } else { 20 }",
        "if x > 5 { 10 } else { }",
        "if x > 5 { } else { }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (empty branch): {:?}",
            case,
            result
        );
    }
}

#[test]
fn expr_accepts_function_calls() {
    // Now function calls are part of unified expressions
    let cases = vec![
        "enforce_rule(student, week)",
        "check_capacity(room, slot)",
        "apply_constraint(x, y, z)",
        "compute_value(student, week)",
        "get_coefficient(x, y)",
        "calculate(a, b, c)",
        "compute_value(x)",
        "calculate(student, week)",
        "get_coefficient(a, b, c)",
        "my_function(x, y)",
        "func()",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_function_calls_in_expressions() {
    // Now function calls are allowed everywhere
    let cases = vec![
        "compute_value(x) + 5",
        "2 * calculate(student, week)",
        "if flag { get_value(x) } else { 0 }",
        "|collection| + compute(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_respects_operator_precedence() {
    let cases = vec![
        "1 + 2 * 3",       // should be 1 + (2 * 3)
        "10 - 5 - 2",      // should be (10 - 5) - 2 (left associative)
        "x and y or z",    // should be (x and y) or z (and before or)
        "not x and y",     // should be (not x) and y (not before and)
        "a < b and c > d", // should be (a < b) and (c > d)
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_global_collections() {
    let cases = vec![
        "@[Student]",
        "@[Subject]",
        "@[Week]",
        "@[Slot]",
        "@[Int]",
        "@[Bool]",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_collection_paths() {
    let cases = vec![
        "subject.slots",
        "student.courses",
        "pairing",
        "pairings_list",
        "teacher.available_slots",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_set_difference() {
    let cases = vec![
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
fn expr_accepts_set_union() {
    let cases = vec![
        "morning_slots union afternoon_slots",
        "@[Student] union @[Teacher]",
        "group1 union group2 union group3",
        "pairing1 union pairing2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_set_intersection() {
    let cases = vec![
        "available_slots inter preferred_slots",
        "@[Student] inter active_students",
        "group1 inter group2",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_parentheses_around_collections() {
    let cases = vec![
        "(@[Subject])",
        "(pairing)",
        "(@[Subject] \\ pairing)",
        "(group1 union group2)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_complex_collection_operations() {
    let cases = vec![
        "(@[Subject] \\ pairing1) \\ pairing2",
        "a union b union c",
        "a inter b inter c",
        "(a union b) \\ c",
        "a union (b \\ c)",
        "(@[Subject] \\ pairing) union extra_subjects",
        "all_slots \\ (morning_slots union evening_slots)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_missing_brackets_in_global_collection() {
    let cases = vec![
        "@Student",  // missing brackets
        "@[Student", // missing closing bracket
        "@Student]", // missing opening bracket
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_arithmetic_and_logical_operators() {
    // These now parse (they're different operations than collection operations)
    let cases = vec![
        "a + b",   // arithmetic
        "a - b",   // arithmetic
        "a and b", // logical
        "a or b",  // logical
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_incomplete_collection_expressions() {
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
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_nested_global_collections() {
    let cases = vec![
        "@[@[Student]]", // can't nest global sets
        "@[[Subject]]",  // wrong syntax
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_numbers_as_expressions() {
    // Numbers now parse (they're valid expressions)
    let cases = vec!["42", "0", "-5"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_multiple_differences() {
    let cases = vec!["@[Subject] \\ pairing1 \\ pairing2"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_deeply_nested_collection_operations() {
    let cases = vec![
        "((a union b) \\ c) inter d",
        "(a union (b union c))",
        "@[Student] \\ (excluded union suspended)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_collections_in_forall() {
    let cases = vec![
        "forall s in @[Student] union @[Teacher] { $V(s) >= 0 }",
        "forall x in (group_a \\ excluded) { $V(x) == 1 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_function_calls_returning_collections() {
    let cases = vec!["get_eligible_students()", "compute_available_slots(week)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_collections_with_function_calls() {
    let cases = vec![
        "get_group_a() union get_group_b()",
        "@[Student] \\ get_excluded()",
        "get_all() inter get_active()",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Function Call Arguments ==========

#[test]
fn expr_accepts_multiple_args_in_function_calls() {
    let cases = vec![
        "compute(student, week.number)",
        "compute(student.age + 5, week)",
        "calculate(|@[Week]|, student.weight, if x { 1 } else { 0 })",
        "func(a, b + 1, |collection|, (x + y) * 2)",
        "process(1, 2, 3, 4, 5)", // many args
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_empty_arg_list() {
    let cases = vec!["compute()", "get_value()"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_accepts_complex_expressions_as_args() {
    let cases = vec![
        "func($V1(x) + $V2(y))",                // LinExpr arg
        "check($V(x) <= 10)",                   // Constraint arg
        "process(@[Student] union @[Teacher])", // Collection arg
        "compute([1, 2, 3])",                   // List literal arg
        "func([x for x in @[S]])",              // Comprehension arg
        "nested(outer(inner(x)))",              // Nested function calls
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// ========== Variable Call Arguments ==========

#[test]
fn expr_accepts_var_calls_with_various_args() {
    let cases = vec![
        "$Var()",                                     // empty
        "$Var(x)",                                    // single
        "$Var(x, y, z)",                              // multiple
        "$StudentInSlot(student, slot, week.number)", // paths and field access
        "$Assigned(student, |@[Week]|)",              // cardinality
        "$Value(if x { 1 } else { 0 }, y)",           // if expression
        "$V(x + 5, y * 2)",                           // arithmetic
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn expr_rejects_malformed_arg_lists() {
    let cases = vec![
        "func(,)",    // empty arg in list
        "func(x,,y)", // double comma
        "func(x,)",   // trailing comma
        "func(,x)",   // leading comma
        "func(x y)",  // missing comma
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}
