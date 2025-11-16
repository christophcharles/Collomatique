use super::*;
use pest::Parser;

mod elementary_bits;
mod expr;
mod ident;
mod let_statements;
mod reify_statements;

#[test]
fn file_accepts_complete_program() {
    let program = r#"
## Define a linear expression
let compute_total(x: Student) -> LinExpr = 
    sum w in @[Week] { $Assigned(x, w) };

## Define a constraint
let enforce_capacity() -> Constraint = 
    forall r in @[Room] { (sum s in @[Student] { $InRoom(s, r) }) <= r.capacity };

## Reify a constraint
reify enforce_capacity as $CapacityOK;

## Public constraint using reified variable
pub let final_rule() -> Constraint = 
    $CapacityOK() == 1 and forall x in @[Student] { compute_total(x) >= 1 };
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse complete program: {:?}",
        result
    );
}

#[test]
fn file_accepts_empty_file() {
    let result = ColloMLParser::parse(Rule::file, "");
    assert!(result.is_ok(), "Should parse empty file");
}

#[test]
fn file_accepts_only_comments() {
    let result = ColloMLParser::parse(Rule::file, "# comment\n# another comment");
    assert!(result.is_ok(), "Should parse file with only comments");
}

#[test]
fn file_accepts_multiple_statements() {
    let program = r#"
let a() -> LinExpr = 5;
let b() -> LinExpr = 10;
reify some_constraint as $Var1;
reify another_constraint as $Var2;
pub let c() -> Constraint = $Var1() == 1 and $Var2() == 1;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse multiple statements: {:?}",
        result
    );
}
