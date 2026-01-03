use super::*;

// =============================================================================
// LET STATEMENT GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of let statements only.
// They do NOT validate semantic correctness - type checking, scope, etc.
// are handled elsewhere.
//
// Grammar: let_statement = pub? "let" ident "(" params? ")" "->" type_name "=" expr ";"

// =============================================================================
// BASIC STRUCTURE
// =============================================================================

#[test]
fn let_statement_minimal_structure() {
    // Most basic valid let statements
    let cases = vec![
        "let f() -> Int = 0;",
        "let g() -> Bool = true;",
        "let h() -> LinExpr = 5;",
        "let i() -> Constraint = $V() === 0;",
        "let j() -> String = get_str();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_pub_modifier() {
    let cases = vec![
        "pub let f() -> LinExpr = 5;",
        "pub let rule() -> Constraint = $V() >== 0;",
        "pub let check() -> Bool = true;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_single_docstring() {
    let cases = vec![
        "## This is a docstring\nlet f() -> LinExpr = 5;",
        "## Calculate something\nlet compute() -> Int = 42;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_multiple_docstrings() {
    let cases = vec![
        "## First line\n## Second line\nlet f() -> LinExpr = 5;",
        "## Line 1\n## Line 2\n## Line 3\npub let g() -> Constraint = $V() === 1;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_varied_whitespace() {
    let cases = vec![
        "let f()->LinExpr=5;",                 // no spaces
        "let   f  (  )  ->  LinExpr  =  5  ;", // many spaces
        "let f(\n) -> LinExpr\n= 5\n;",        // newlines
        "let f() -> LinExpr = 5; # comment",   // trailing comment
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// PARAMETERS
// =============================================================================

#[test]
fn let_statement_with_no_parameters() {
    let cases = vec![
        "let constant() -> Int = 42;",
        "let get_value() -> LinExpr = 100;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_single_parameter() {
    let cases = vec![
        "let compute(x: Int) -> Int = x;",
        "let check(s: Student) -> Bool = s.active;",
        "let value(n: Bool) -> Int = 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_multiple_parameters() {
    let cases = vec![
        "let f(x: Int, y: Int) -> Int = x;",
        "let g(s: Student, w: Week, sl: Slot) -> Bool = true;",
        "let h(a: Int, b: Bool, c: Student) -> LinExpr = 5;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_list_type_parameters() {
    let cases = vec![
        "let f(students: [Student]) -> Int = 0;",
        "let g(items: [Int], flag: Bool) -> Bool = flag;",
        "let h(matrix: [[Int]]) -> Int = 0;",
        "let i(deep: [[[Student]]]) -> Bool = true;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_parameters_with_custom_types() {
    let cases = vec![
        "let f(s: Student) -> Int = 0;",
        "let g(r: Room, w: Week) -> Bool = true;",
        "let h(obj: CustomType) -> LinExpr = 5;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// RETURN TYPES
// =============================================================================

#[test]
fn let_statement_with_primitive_return_types() {
    let cases = vec![
        "let f() -> Int = 42;",
        "let g() -> Bool = true;",
        "let h() -> LinExpr = 5;",
        "let i() -> Constraint = $V() === 0;",
        "let j() -> String = get_name();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_custom_return_types() {
    let cases = vec![
        "let f() -> Student = get_student();",
        "let g() -> Week = current_week();",
        "let h() -> Room = get_room();",
        "let i() -> CustomType = value();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_list_return_types() {
    let cases = vec![
        "let f() -> [Int] = [1, 2, 3];",
        "let g() -> [Student] = @[Student];",
        "let h() -> [LinExpr] = [];",
        "let i() -> [[Int]] = [[1, 2], [3, 4]];",
        "let j() -> [[[Bool]]] = [];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// STRING TYPE TESTS
// =============================================================================

#[test]
fn let_statement_with_string_return_type() {
    let cases = vec![
        "let f() -> String = get_name();",
        "let g() -> String = format_output();",
        "let h(s: Student) -> String = s.name;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_string_parameter() {
    let cases = vec![
        "let f(name: String) -> Int = 0;",
        "let g(s: String, n: Int) -> Bool = true;",
        "let h(prefix: String, suffix: String) -> String = prefix + suffix;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_string_list_types() {
    let cases = vec![
        "let f() -> [String] = [];",
        "let g() -> [String] = get_names();",
        "let h(names: [String]) -> Int = |names|;",
        "let i() -> [[String]] = [];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - LITERALS AND SIMPLE VALUES
// =============================================================================

#[test]
fn let_statement_with_literal_expressions() {
    let cases = vec![
        "let f() -> Int = 42;",
        "let g() -> Int = -10;",
        "let h() -> Bool = true;",
        "let i() -> Bool = false;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_path_expressions() {
    let cases = vec![
        "let f(s: Student) -> Int = s.age;",
        "let g(r: Room) -> Int = r.capacity;",
        "let h(x: Something) -> Int = x.field.nested.deep;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - VARIABLE CALLS
// =============================================================================

#[test]
fn let_statement_with_variable_calls() {
    // Reified variables are called with $Name(args)
    let cases = vec![
        "let f() -> LinExpr = $V();",
        "let g(x: Int) -> LinExpr = $V(x);",
        "let h(s: Student) -> LinExpr = $Assigned(s);",
        "let i(s: Student, w: Week) -> LinExpr = $InWeek(s, w);",
        "let j() -> Constraint = $Check() === 0;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_multiple_variable_calls() {
    let cases = vec![
        "let f(x: Int, y: Int) -> LinExpr = $V1(x) + $V2(y);",
        "let g(s: Student) -> LinExpr = $V1(s) + $V2(s) + $V3(s);",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - FUNCTION CALLS
// =============================================================================

#[test]
fn let_statement_with_function_calls() {
    let cases = vec![
        "let f() -> LinExpr = helper();",
        "let g(x: Student) -> LinExpr = compute(x);",
        "let h(a: Int, b: Int) -> Int = add(a, b);",
        "let i() -> LinExpr = nested(compute(value()));",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - ARITHMETIC
// =============================================================================

#[test]
fn let_statement_with_arithmetic_expressions() {
    let cases = vec![
        "let f() -> Int = 2 + 3;",
        "let g() -> Int = 10 - 5;",
        "let h() -> Int = 4 * 7;",
        "let i() -> Int = 20 // 4;",
        "let j() -> Int = 17 % 5;",
        "let k() -> Int = 2 + 3 * 4;",
        "let l() -> Int = (2 + 3) * 4;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_complex_linexpr() {
    let cases = vec![
        "let f(x: Int) -> LinExpr = 2 * $V1(x) + 3 * $V2(x) + 5;",
        "let g(s: Student) -> LinExpr = $Var(s) * s.weight + 10;",
        "let h(x: Int, y: Int) -> LinExpr = x * $V1() + y * $V2();",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - COMPARISONS
// =============================================================================

#[test]
fn let_statement_with_comparison_expressions() {
    let cases = vec![
        "let f() -> Bool = 5 < 10;",
        "let g() -> Bool = 10 > 5;",
        "let h() -> Bool = 5 <= 10;",
        "let i() -> Bool = 10 >= 5;",
        "let j() -> Bool = 5 == 5;",
        "let k() -> Bool = 5 != 6;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_constraint_comparisons() {
    // Constraint-specific comparison operators: ===, <==, >==
    let cases = vec![
        "let f() -> Constraint = $V() === 0;",
        "let g() -> Constraint = $V() <== 10;",
        "let h() -> Constraint = $V() >== 0;",
        "let i(s: Student) -> Constraint = $Assigned(s) === 1;",
        "let j(x: Int) -> Constraint = $V(x) <== x;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - LOGICAL OPERATORS
// =============================================================================

#[test]
fn let_statement_with_logical_expressions() {
    let cases = vec![
        "let f() -> Bool = true and false;",
        "let g() -> Bool = true or false;",
        "let h() -> Bool = not true;",
        "let i() -> Bool = true && false;",
        "let j() -> Bool = true || false;",
        "let k() -> Bool = !true;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_constraint_conjunctions() {
    let cases = vec![
        "let f() -> Constraint = $V1() === 0 and $V2() >== 1;",
        "let g() -> Constraint = $Check1() === 1 and $Check2() === 0 and $Check3() <== 1;",
        "let h(x: Int) -> Constraint = $V(x) >== 0 and $V(x) <== 10;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - COLLECTIONS
// =============================================================================

#[test]
fn let_statement_with_list_literals() {
    let cases = vec![
        "let f() -> [Int] = [];",
        "let g() -> [Int] = [1];",
        "let h() -> [Int] = [1, 2, 3];",
        "let i() -> [[Int]] = [[1, 2], [3, 4]];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_list_comprehensions() {
    let cases = vec![
        "let f() -> [Int] = [x for x in [1, 2, 3]];",
        "let g() -> [Int] = [x * 2 for x in [1, 2, 3]];",
        "let h() -> [Int] = [x for x in [1, 2, 3, 4, 5] where x > 2];",
        "let i() -> [LinExpr] = [$V(x) for x in [1, 2, 3]];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_global_collections() {
    let cases = vec![
        "let f() -> [Student] = @[Student];",
        "let g() -> [Week] = @[Week];",
        "let h() -> [Int] = @[Int];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_collection_operations() {
    let cases = vec![
        "let f() -> [Int] = [1, 2] + [3, 4];",
        "let h() -> [Int] = [1, 2, 3] - [2];",
        "let i() -> Bool = 5 in [1, 2, 3, 4, 5];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_cardinality() {
    let cases = vec![
        "let f() -> Int = |[1, 2, 3]|;",
        "let g() -> Int = |@[Student]|;",
        "let h(students: [Student]) -> Int = |students|;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - AGGREGATIONS
// =============================================================================

#[test]
fn let_statement_with_sum_expressions() {
    let cases = vec![
        "let f() -> LinExpr = sum x in [1, 2, 3] { x };",
        "let g() -> LinExpr = sum s in @[Student] { $V(s) };",
        "let h() -> LinExpr = sum x in [1, 2, 3] where x > 1 { x * 2 };",
        "let i(students: [Student]) -> LinExpr = sum s in students { $Assigned(s) };",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_with_forall_expressions() {
    let cases = vec![
        "let f() -> Constraint = forall x in [1, 2, 3] { $V(x) >== 0 };",
        "let g() -> Constraint = forall s in @[Student] { $Assigned(s) === 1 };",
        "let h() -> Constraint = forall x in @[Int] where x > 0 { $V(x) <== 10 };",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - CONTROL FLOW
// =============================================================================

#[test]
fn let_statement_with_if_expressions() {
    let cases = vec![
        "let f(x: Int) -> Int = if x > 0 { x } else { 0 };",
        "let g(b: Bool) -> LinExpr = if b { $V1() } else { $V2() };",
        "let h(s: Student) -> Constraint = if s.active { $Assigned(s) === 1 } else { $Assigned(s) === 0 };",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// EXPRESSION BODIES - TYPE CASTS
// =============================================================================

#[test]
fn let_statement_with_explicit_type_casts() {
    let cases = vec![
        "let f() -> LinExpr = 5 as LinExpr;",
        "let g(x: Int) -> LinExpr = x as LinExpr;",
        "let h() -> Int = true as Int;",
        "let i() -> [Student] = [] as [Student];",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// REALISTIC EXAMPLES
// =============================================================================

#[test]
fn let_statement_realistic_linexpr_examples() {
    let cases = vec![
        // Sum with cardinality multiplier
        "let f(s: Student) -> LinExpr = |@[Week]| * $V(s);",
        // Weighted sum
        "let g() -> LinExpr = sum s in @[Student] { s.weight * $Assigned(s) };",
        // Conditional expression
        "let h(s: Student) -> LinExpr = if s.priority > 5 { 10 * $V(s) } else { $V(s) };",
        // Complex arithmetic with function calls
        "let i(s: Student) -> LinExpr = compute_base(s) + 2 * $Extra(s);",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn let_statement_realistic_constraint_examples() {
    let cases = vec![
        // Forall with sum constraint
        "let f() -> Constraint = forall w in @[Week] { sum s in @[Student] { $Assigned(s, w) } <== 10 };",
        // Conjunction of constraints
        "let g(s: Student) -> Constraint = $V1(s) >== 0 and $V1(s) <== s.max_value;",
        // Nested forall
        "let h() -> Constraint = forall s in @[Student] { forall w in @[Week] { $InWeek(s, w) <== 1 } };",
        // Conditional constraint
        "let i(r: Room) -> Constraint = if r.available { sum s in @[Student] { $InRoom(s, r) } <== r.capacity } else { sum s in @[Student] { $InRoom(s, r) } === 0 };",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// SEMANTICALLY INCORRECT BUT GRAMMATICALLY VALID
// =============================================================================
// These tests explicitly check that the parser accepts code that is
// syntactically correct but semantically wrong. Type checking and semantic
// analysis should catch these errors later.

#[test]
fn let_statement_accepts_type_mismatches() {
    // Parser should accept these even though they're semantically wrong
    let cases = vec![
        // Returning wrong type
        "let f() -> Int = true;",          // Bool where Int expected
        "let g() -> Bool = 42;",           // Int where Bool expected
        "let h() -> LinExpr = [1, 2, 3];", // List where LinExpr expected
        "let i() -> Constraint = 5;",      // Int where Constraint expected
        // Using undefined variables (grammar doesn't check scope)
        "let j() -> LinExpr = $UndefinedVar(x);", // x not in scope
        "let k() -> Int = undefined_function();", // function doesn't exist
        // Type mismatches in operations
        "let l() -> Int = 5 + true;",               // Int + Bool
        "let m() -> LinExpr = $V() + [1, 2];",      // LinExpr + List
        "let n() -> Bool = if 5 { x } else { y };", // Int as condition
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
fn let_statement_accepts_wrong_operator_usage() {
    // Parser should accept these; semantics will reject them
    let cases = vec![
        // Using constraint operators on non-LinExpr
        "let f() -> Bool = [5] === [5,7];",  // [Int] === [Int]
        "let g() -> Bool = true <== false;", // Bool <== Bool
        // Using regular operators where constraint ops expected
        "let h() -> Constraint = $V() == 5;",  // Should use ===
        "let i() -> Constraint = $V() <= 10;", // Should use <==
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

// =============================================================================
// NEGATIVE TESTS - INVALID SYNTAX
// =============================================================================

#[test]
fn let_statement_rejects_missing_return_type() {
    let cases = vec!["let f() = 5;", "let g(x: Student) = x.age;"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing return type): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_rejects_missing_body() {
    let cases = vec![
        "let f() -> LinExpr;",
        "let g(x: Student) -> Constraint;",
        "let h() -> LinExpr =;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing body): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_rejects_missing_semicolon() {
    let cases = vec![
        "let f() -> LinExpr = 5",
        "let g() -> Constraint = $V() === 0",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (missing semicolon): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_rejects_invalid_parameter_syntax() {
    let cases = vec![
        "let f(x) -> LinExpr = 5;",                  // missing type annotation
        "let f(: Student) -> LinExpr = 5;",          // missing parameter name
        "let f(x Student) -> LinExpr = 5;",          // missing colon
        "let f(x: Student, ) -> LinExpr = 5;",       // trailing comma
        "let f(x: Student y: Week) -> LinExpr = 5;", // missing comma
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (invalid parameter syntax): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_rejects_invalid_syntax() {
    let cases = vec![
        "let f[] -> LinExpr = 5;",         // wrong brackets for params
        "let -> LinExpr = 5;",             // missing name
        "f() -> LinExpr = 5;",             // missing 'let'
        "let f() -> LinExpr == 5;",        // wrong assignment operator
        "let f() -> = 5;",                 // missing type
        "let f() LinExpr = 5;",            // missing arrow
        "pub pub let f() -> LinExpr = 5;", // double pub
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (invalid syntax): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_rejects_reserved_keywords_as_names() {
    let cases = vec![
        "let let() -> LinExpr = 5;",
        "let forall() -> LinExpr = 5;",
        "let sum() -> LinExpr = 5;",
        "let if() -> LinExpr = 5;",
        "let reify() -> LinExpr = 5;",
        "let where() -> LinExpr = 5;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (reserved keyword as name): {:?}",
            case,
            result
        );
    }
}

#[test]
fn let_statement_rejects_reserved_keywords_as_parameters() {
    let cases = vec![
        "let f(let: Int) -> Int = 0;",
        "let g(sum: Student) -> Bool = true;",
        "let h(if: Bool) -> LinExpr = 5;",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::let_statement_complete, case);
        assert!(
            result.is_err(),
            "Should reject '{}' (reserved keyword as parameter): {:?}",
            case,
            result
        );
    }
}
