use super::*;
use pest::Parser;

mod args;
mod collection_expr;
mod computable_expr;
mod constraint_expr;
mod elementary_bits;
mod ident;
mod let_statements;
mod lin_expr;
mod reify_statements;

#[test]
fn file_accepts_complete_program() {
    let program = r#"
## Define a linear expression
let compute_total(x: Student) -> LinExpr = 
    sum w in @[Week]: $Assigned(x, w);

## Define a constraint
let enforce_capacity() -> Constraint = 
    forall r in @[Room]: (sum s in @[Student]: $InRoom(s, r)) <= r.capacity;

## Reify a constraint
reify enforce_capacity as $CapacityOK;

## Public constraint using reified variable
pub let final_rule() -> Constraint = 
    $CapacityOK() == 1 and forall x in @[Student]: compute_total(x) >= 1;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse complete program: {:?}",
        result
    );
}
