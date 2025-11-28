use super::*;

// =============================================================================
// CARDINALITY EXPRESSIONS
// =============================================================================
// Tests for |expr| cardinality operator

#[test]
fn cardinality_accepts_simple_collections() {
    let cases = vec!["|@[Student]|", "|@[Week]|", "|@[Int]|"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_accepts_paths() {
    let cases = vec![
        "|pairing|",
        "|subject.slots|",
        "|student.courses|",
        "|collection|",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_accepts_list_literals() {
    let cases = vec!["|[1, 2, 3]|", "|[]|", "|[x, y, z]|"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_accepts_set_operations() {
    let cases = vec![
        "|@[Student] \\ excluded|",
        "|a union b|",
        "|(a union b) \\ c|",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_in_arithmetic() {
    let cases = vec![
        "|@[Student]| + 1",
        "|collection| * 2",
        "5 + |@[Week]|",
        "|@[Student]| * $Var(x)",
        "(|@[Student]|) * $Var(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_in_comparisons() {
    let cases = vec![
        "|@[Student]| > 0",
        "|collection| == 10",
        "sum x in @[X] { $V(x) } === |@[X]|",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_with_modulo() {
    let cases = vec!["|@[Week]| % 4", "(|@[Student]| // 2) * $Var(x)"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_nested_expressions() {
    let cases = vec![
        "|collection| + $Var(x) + |other_collection|",
        "if x { |collection| + 1 } else { |collection| - 1 }",
        "$Var(x) + if flag { |@[X]| } else { 0 } + $Var(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
