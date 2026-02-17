use super::*;

// =============================================================================
// CARDINALITY EXPRESSIONS
// =============================================================================
// Tests for |expr| cardinality operator

#[test]
fn cardinality_accepts_simple_collections() {
    let cases = vec!["|students|", "|weeks|", "|numbers|"];
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
    let cases = vec!["|students - excluded|", "|a + b|", "|(a + b) - c|"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_in_arithmetic() {
    let cases = vec![
        "|students| + 1",
        "|collection| * 2",
        "5 + |weeks|",
        "|students| * $Var(x)",
        "(|students|) * $Var(x)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_in_comparisons() {
    let cases = vec![
        "|students| > 0",
        "|collection| == 10",
        "sum x in x_list { $V(x) } === |x_list|",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn cardinality_with_modulo() {
    let cases = vec!["|weeks| % 4", "(|students| / 2) * $Var(x)"];
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
        "$Var(x) + if flag { |x_list| } else { 0 } + $Var(y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
