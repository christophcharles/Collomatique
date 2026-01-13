use super::*;

// =============================================================================
// IMPORT STATEMENT GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of import statements and
// qualified variable access expressions.
//
// Syntax:
//   import "module_name" as mod;    - Named import
//   import "module_name" as *;      - Wildcard import
//   mod::$Var(args)                 - Qualified variable call
//   mod::$[VarList](args)           - Qualified variable list call
//
// These are grammar tests only - they do NOT validate semantic correctness.

// =============================================================================
// IMPORT STATEMENT PARSING
// =============================================================================

#[test]
fn import_statement_named() {
    let cases = vec![
        r#"import "module" as m;"#,
        r#"import "my_module" as myMod;"#,
        r#"import "path/to/module" as mod;"#,
        r#"import "some-module" as some_module;"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::import_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn import_statement_wildcard() {
    let cases = vec![
        r#"import "module" as *;"#,
        r#"import "my_module" as *;"#,
        r#"import "path/to/module" as *;"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::import_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn import_statement_with_raw_strings() {
    let cases = vec![
        r#"import ~"module with "quotes""~ as m;"#,
        r#"import ~~"complex "string" here"~~ as m;"#,
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::import_statement_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn import_statement_rejects_missing_as() {
    let result = ColloMLParser::parse(Rule::import_statement_complete, r#"import "module" mod;"#);
    assert!(result.is_err(), "Should reject missing 'as' keyword");
}

#[test]
fn import_statement_rejects_missing_semicolon() {
    let result = ColloMLParser::parse(Rule::import_statement_complete, r#"import "module" as mod"#);
    assert!(result.is_err(), "Should reject missing semicolon");
}

#[test]
fn import_statement_rejects_missing_target() {
    let result = ColloMLParser::parse(Rule::import_statement_complete, r#"import "module" as ;"#);
    assert!(result.is_err(), "Should reject missing import target");
}

// =============================================================================
// QUALIFIED VARIABLE CALL PARSING
// =============================================================================

#[test]
fn qualified_var_call_basic() {
    let cases = vec![
        "mod::$Var()",
        "mod::$Var(x)",
        "mod::$Var(x, y)",
        "myModule::$MyVar(arg1, arg2, arg3)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn qualified_var_call_with_expressions() {
    let cases = vec![
        "mod::$Var(1 + 2)",
        "mod::$Var(if true { 1 } else { 2 })",
        "mod::$Var(x.field)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// QUALIFIED VARIABLE LIST CALL PARSING
// =============================================================================

#[test]
fn qualified_var_list_call_basic() {
    let cases = vec![
        "mod::$[VarList]()",
        "mod::$[VarList](x)",
        "mod::$[VarList](x, y)",
        "myModule::$[MyVarList](arg1, arg2)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn qualified_var_list_call_with_expressions() {
    let cases = vec![
        "mod::$[VarList](1 + 2)",
        "mod::$[VarList](if true { 1 } else { 2 })",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// QUALIFIED CALLS IN EXPRESSIONS
// =============================================================================

#[test]
fn qualified_var_call_in_expression() {
    let cases = vec![
        "mod::$Var(x) === 1",
        "mod::$Var(x) + other::$Var(y)",
        "sum i in @[Int] { mod::$Assigned(i) }",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn qualified_var_list_call_in_expression() {
    let cases = vec!["|mod::$[VarList](x)|", "mod::$[VarList](x)[0]!"];
    for case in cases {
        let result = ColloMLParser::parse(Rule::expr_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

// =============================================================================
// IMPORT STATEMENTS IN FILES
// =============================================================================

#[test]
fn file_accepts_import_statement() {
    let program = r#"import "other_module" as other;"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse file with import statement: {:?}",
        result
    );
}

#[test]
fn file_accepts_multiple_imports() {
    let program = r#"
import "module1" as m1;
import "module2" as m2;
import "module3" as *;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse file with multiple imports: {:?}",
        result
    );
}

#[test]
fn file_accepts_imports_with_statements() {
    let program = r#"
import "reifications" as reif;
import "constraints" as *;

let my_func(x: Int) -> LinExpr = x;

pub let constraint() -> Constraint =
    $Var() === 1;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse file with imports and statements: {:?}",
        result
    );
}
