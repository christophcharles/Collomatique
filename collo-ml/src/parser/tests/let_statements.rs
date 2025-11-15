use super::*;

#[test]
fn let_accepts_lin_expr_output() {
    let cases = vec![
        "let f() -> LinExpr = 5;",
        "let compute(x: Student) -> LinExpr = $Var(x);",
        "let sum_vars(x: Int) -> LinExpr = $V1(x) + $V2(x);",
        "let weighted(s: Student) -> LinExpr = 2 * $Assigned(s);",
        "let total() -> LinExpr = sum x in @[Student]: $V(x);",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_accepts_constraint_output() {
    let cases = vec![
        "let rule() -> Constraint = $V(x) <= 10;",
        "let enforce(s: Student) -> Constraint = $Assigned(s) == 1;",
        "let capacity(r: Room) -> Constraint = sum x in @[X]: $InRoom(x, r) <= r.capacity;",
        "let check() -> Constraint = forall x in @[X]: $V(x) >= 0;",
        "let combined() -> Constraint = $V1(x) <= 10 and $V2(y) >= 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_accepts_pub_modifier() {
    let cases = vec![
        "pub let f() -> LinExpr = 5;",
        "pub let rule() -> Constraint = $V(x) <= 10;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_accepts_multiple_parameters() {
    let cases = vec![
        "let f(x: Student, y: Week) -> LinExpr = $V(x, y);",
        "let rule(s: Student, w: Week, sl: Slot) -> Constraint = $InSlot(s, sl, w) <= 1;",
        "let compute(a: Int, b: Int, c: Bool) -> LinExpr = a * $V(b);",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_accepts_list_type_parameters() {
    let cases = vec![
        "let f(students: [Student]) -> LinExpr = 5;",
        "let rule(pairing: [Subject], s: Student) -> Constraint = $V(s) == 1;",
        "let nested(grid: [[Int]]) -> LinExpr = $V(x);",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_accepts_no_parameters() {
    let cases = vec![
        "let constant() -> LinExpr = 42;",
        "let global_rule() -> Constraint = $GlobalVar(x) <= 100;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_accepts_complex_lin_expr() {
    let cases = vec![
        "let f(x: Student) -> LinExpr = 2 * $V1(x) + 3 * $V2(x) + 5;",
        "let g(s: Student) -> LinExpr = sum w in @[Week]: $HasColle(s, w);",
        "let h(x: Int) -> LinExpr = if x > 5 { 10 } else { 0 };",
        "let i(s: Student) -> LinExpr = (|@[Week]|) * $V(s);",
        "let j(x: Student) -> LinExpr = $V(x) + compute_other(x);",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_accepts_complex_constraint() {
    let cases = vec![
        "let rule(s: Student) -> Constraint = forall w in @[Week]: $HasColle(s, w) <= 1;",
        "let check(x: Int) -> Constraint = $V1(x) <= 10 and $V2(x) >= 0;",
        "let enforce(s: Student) -> Constraint = if s.is_active { $Assigned(s) == 1 } else { $Assigned(s) == 0 };",
        "let capacity() -> Constraint = (sum x in @[X]: $V(x)) <= 100;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_accepts_docstrings() {
    let cases = vec![
        "## This is a docstring\nlet f() -> LinExpr = 5;",
        "## First line\n## Second line\nlet rule() -> Constraint = $V(x) <= 10;",
        "## Comment\npub let g(x: Student) -> LinExpr = $V(x);",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_rejects_invalid_output_types() {
    let cases = vec![
        "let f() -> Bool = true;",
        "let g() -> Int = 5;",
        "let h() -> Student = x;",
        "let i() -> [Subject] = pairing;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (invalid output type): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_rejects_missing_output_type() {
    let cases = vec!["let f() = 5;", "let g(x: Student) = $V(x);"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing output type): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_rejects_missing_body() {
    let cases = vec![
        "let f() -> LinExpr;",
        "let g(x: Student) -> Constraint;",
        "let h() -> LinExpr =;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should not parse '{}' (missing body): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_rejects_invalid_syntax() {
    let cases = vec![
        "let f[] -> LinExpr = 5;",             // wrong brackets
        "let f(x) -> LinExpr = 5;",            // missing type annotation
        "let f(x: Student, ) -> LinExpr = 5;", // trailing comma
        "let -> LinExpr = 5;",                 // missing name
        "f() -> LinExpr = 5;",                 // missing 'let'
        "let f() -> LinExpr == 5;",            // wrong assignment operator
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_err(), "Should not parse '{}': {:?}", case, result);
    }
}

#[test]
fn statement_accepts_varied_whitespace() {
    let cases = vec![
        "let f()->LinExpr=5;",                 // no spaces
        "let   f  (  )  ->  LinExpr  =  5  ;", // lots of spaces
        "let f(\n) -> LinExpr\n= 5\n;",        // newlines
        "let f() -> LinExpr = 5; # comment",   // trailing comment
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
