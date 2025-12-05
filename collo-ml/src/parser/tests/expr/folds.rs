use super::*;

// =============================================================================
// FOLD EXPRESSIONS
// =============================================================================
// Tests for: fold, rfold (with optional where clause)

#[test]
fn fold_accepts_simple_fold() {
    let cases = vec![
        "fold x in @[Student] with acc = 0 { acc + $Var(x) }",
        "fold x in [1, 2, 3] with summation = 0 { summation + x }",
        "fold s in collection with total = 0 { total + $V(s) }",
        "fold x in set with result = 1 { result * x }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fold_accepts_different_accumulator_names() {
    let cases = vec![
        "fold x in [1, 2, 3] with acc = 0 { acc + x }",
        "fold x in [1, 2, 3] with summation = 0 { summation + x }",
        "fold x in [1, 2, 3] with total = 0 { total + x }",
        "fold x in [1, 2, 3] with result = 0 { result + x }",
        "fold x in [1, 2, 3] with my_accumulator = 0 { my_accumulator + x }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fold_accepts_complex_initial_values() {
    let cases = vec![
        "fold x in list with acc = 2 * 5 { acc + x }",
        "fold x in list with acc = some_function() { acc + x }",
        "fold x in list with acc = a + b { acc + x }",
        "fold x in list with acc = if true { 0 } else { 1 } { acc + x }",
        "fold x in list with acc = [1, 2, 3] { acc + [x] }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fold_accepts_fold_with_where() {
    let cases = vec![
        "fold x in @[Student] with acc = 0 where x.is_active { acc + $Var(x) }",
        "fold slot in slots with summation = 0 where slot.hour > 8 { summation + $StudentInSlot(s, slot, w) }",
        "fold x in collection with acc = 0 where x.value > 0 and x.flag { acc + $V(x) }",
        "fold x in [1, 2, 3] with acc = 0 where x > 1 { acc + x * 2 }",
        "fold x in list with acc = 0 where x % 2 == 1 { acc + x }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fold_accepts_nested_folds() {
    let cases = vec![
        "fold x in @[X] with acc1 = 0 { fold y in @[Y] with acc2 = 0 { acc2 + $Var(x, y) } }",
        "fold outer in list1 with sum1 = 0 { fold inner in list2 with sum2 = 0 { sum2 + inner } }",
        "fold sub_list in matrix with acc = 0 { fold x in sub_list with acc2 = acc { acc2 + x } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fold_accepts_complex_body_expressions() {
    let cases = vec![
        "fold x in list with acc = 0 { if x > 0 { acc + x } else { acc } }",
        "fold x in list with acc = 0 { acc + (x * 2) }",
        "fold x in list with acc = 0 { let tmp = x * 2 { acc + tmp } }",
        "fold x in list with acc = [] { acc + [x] }",
        "fold x in list with acc = 0 { acc + sum y in x { y } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fold_accepts_fold_in_larger_expression() {
    let cases = vec![
        "(fold x in @[X] with acc = 0 { acc + $V(x) }) + 10",
        "2 * (fold x in @[X] with acc = 1 { acc * $V(x) })",
        "(fold x in @[X] with acc = 0 { acc + $V(x) }) - (fold y in @[Y] with acc = 0 { acc + $V(y) })",
        "fold x in @[X] with acc = 0 { acc + $V(x) + 5 }", // 5 inside fold body
        "fold x in @[X] with acc = 0 { acc + $V(x) } + 5", // 5 outside fold
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fold_accepts_fold_with_collection_operations() {
    let cases = vec![
        "fold s in @[Student] + @[Teacher] with acc = 0 { acc + $V(s) }",
        "fold s in group_a + group_b with summation = 0 { summation + $V(s) }",
        "fold x in (group_a - excluded) with acc = 0 { acc + $V(x) }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn fold_accepts_mixing_fold_with_sum_and_forall() {
    let cases = vec![
        "fold x in list with acc = 0 { acc + sum y in x { y } }",
        "sum x in list { fold y in x with acc = 0 { acc + y } }",
        "forall x in list { fold y in x with acc = 0 { acc + y } >= 10 }",
        "fold x in list with acc = 0 { acc + (forall y in x { $V(y) == 1 }) as Int }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// RFOLD EXPRESSIONS
// =============================================================================

#[test]
fn rfold_accepts_simple_rfold() {
    let cases = vec![
        "rfold x in @[Student] with acc = 0 { acc + $Var(x) }",
        "rfold x in [1, 2, 3] with summation = 0 { summation + x }",
        "rfold s in collection with total = 0 { total + $V(s) }",
        "rfold x in set with result = 1 { result * x }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn rfold_accepts_rfold_with_where() {
    let cases = vec![
        "rfold x in @[Student] with acc = 0 where x.is_active { acc + $Var(x) }",
        "rfold slot in slots with summation = 0 where slot.hour > 8 { summation + $StudentInSlot(s, slot, w) }",
        "rfold x in collection with acc = 0 where x.value > 0 and x.flag { acc + $V(x) }",
        "rfold x in [1, 2, 3] with acc = 0 where x > 1 { acc + x * 2 }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn rfold_accepts_nested_rfolds() {
    let cases = vec![
        "rfold x in @[X] with acc1 = 0 { rfold y in @[Y] with acc2 = 0 { acc2 + $Var(x, y) } }",
        "rfold outer in list1 with sum1 = 0 { rfold inner in list2 with sum2 = 0 { sum2 + inner } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn rfold_accepts_mixing_fold_and_rfold() {
    let cases = vec![
        "fold x in list1 with acc = 0 { rfold y in list2 with acc2 = 0 { acc2 + y } }",
        "rfold x in list1 with acc = 0 { fold y in list2 with acc2 = 0 { acc2 + y } }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// ERROR CASES
// =============================================================================

#[test]
fn fold_rejects_incomplete_expressions() {
    let cases = vec![
        "fold x in @[X] with acc = 0 {",         // missing closing brace
        "fold x in @[X] with acc = 0 acc + x",   // missing braces
        "fold x in @[X] with acc = 0",           // missing body
        "fold x in @[X] acc = 0 { acc + x }",    // missing 'with'
        "fold x in @[X] with { acc + x }",       // missing accumulator definition
        "fold x in @[X] with acc { acc + x }",   // missing '=' in accumulator
        "fold x @[X] with acc = 0 { acc + x }",  // missing 'in'
        "fold in @[X] with acc = 0 { acc + x }", // missing iterator variable
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn rfold_rejects_incomplete_expressions() {
    let cases = vec![
        "rfold x in @[X] with acc = 0 {",       // missing closing brace
        "rfold x in @[X] with acc = 0 acc + x", // missing braces
        "rfold x in @[X] with acc = 0",         // missing body
        "rfold x in @[X] acc = 0 { acc + x }",  // missing 'with'
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}

#[test]
fn fold_rejects_reserved_keywords_as_names() {
    let cases = vec![
        "fold let in @[X] with acc = 0 { acc + let }", // 'let' is reserved
        "fold in in @[X] with acc = 0 { acc + in }",   // 'in' is reserved
        "fold x in @[X] with fold = 0 { fold + x }",   // 'fold' is reserved as accumulator
        "fold x in @[X] with where = 0 { where + x }", // 'where' is reserved as accumulator
        "fold x in @[X] with with = 0 { with + x }",   // 'with' is reserved as accumulator
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_err(), "Should reject '{}': {:?}", case, result);
    }
}
