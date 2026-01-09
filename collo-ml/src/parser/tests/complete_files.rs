use super::*;

// =============================================================================
// COMPLETE PROGRAM TESTS
// =============================================================================
// These tests validate that the parser can handle complete, multi-statement
// programs including combinations of let statements, reify statements, and
// docstrings.

#[test]
fn file_accepts_empty_program() {
    let result = ColloMLParser::parse(Rule::file, "");
    assert!(result.is_ok(), "Should parse empty file: {:?}", result);
}

#[test]
fn file_accepts_only_comments() {
    let program = "// comment\n// another comment\n// more comments";
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse file with only comments: {:?}",
        result
    );
}

#[test]
fn file_accepts_single_let_statement() {
    let program = "let f() -> LinExpr = 5;";
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse single let statement: {:?}",
        result
    );
}

#[test]
fn file_accepts_single_reify_statement() {
    let program = "reify my_constraint as $MyVar;";
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse single reify statement: {:?}",
        result
    );
}

#[test]
fn file_accepts_multiple_let_statements() {
    let program = r#"
let a() -> LinExpr = 5;
let b() -> LinExpr = 10;
let c() -> Constraint = $V() === 0;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse multiple let statements: {:?}",
        result
    );
}

#[test]
fn file_accepts_multiple_reify_statements() {
    let program = r#"
reify constraint1 as $Var1;
reify constraint2 as $Var2;
reify constraint3 as $Var3;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse multiple reify statements: {:?}",
        result
    );
}

#[test]
fn file_accepts_mixed_statements() {
    let program = r#"
let a() -> LinExpr = 5;
reify my_constraint as $Var1;
let b() -> LinExpr = 10;
reify another_constraint as $Var2;
pub let c() -> Constraint = $Var1() === 0 and $Var2() === 1;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse mixed let and reify statements: {:?}",
        result
    );
}

#[test]
fn file_accepts_statements_with_docstrings() {
    let program = r#"
/// This is a docstring for the first function
let compute_value() -> LinExpr = 42;

/// This is a constraint
/// It has multiple lines of documentation
reify my_constraint as $MyConstraint;

/// Public function
pub let final_result() -> Constraint = $MyConstraint() === 1;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse statements with docstrings: {:?}",
        result
    );
}

#[test]
fn file_accepts_statements_with_comments() {
    let program = r#"
// This is a regular comment
let a() -> LinExpr = 5;

// Another comment
let b() -> LinExpr = 10; // trailing comment

// Comment before reify
reify constraint as $Var; // trailing comment
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse statements with comments: {:?}",
        result
    );
}

#[test]
fn file_accepts_realistic_small_program() {
    let program = r#"
/// Define a linear expression counting assignments
let count_assignments(student: Student) -> LinExpr =
    sum week in @[Week] { $Assigned(student, week) };

/// Ensure each student has at least one assignment
let min_assignments(student: Student) -> Constraint =
    count_assignments(student) >== 1;

/// Reify the minimum assignment constraint
reify min_assignments as $MinAssignments;

/// Public final constraint combining reified variables
pub let enforce_all_rules() -> Constraint =
    forall s in @[Student] { $MinAssignments(s) === 1 };
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse realistic small program: {:?}",
        result
    );
}

#[test]
fn file_accepts_realistic_medium_program() {
    let program = r#"
/// Define a linear expression for total assignments
let compute_total(x: Student) -> LinExpr =
    sum w in @[Week] { $Assigned(x, w) };

/// Define a capacity constraint for rooms
let enforce_capacity() -> Constraint =
    forall r in @[Room] {
        sum s in @[Student] { $InRoom(s, r) } <== r.capacity
    };

/// Reify the capacity constraint
reify enforce_capacity as $CapacityOK;

/// Public constraint combining multiple rules
pub let final_rule() -> Constraint =
    $CapacityOK() === 1 and forall x in @[Student] {
        compute_total(x) >== 1
    };
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse realistic medium program: {:?}",
        result
    );
}

#[test]
fn file_accepts_complex_realistic_program() {
    let program = r#"
/// Calculate the number of slots a student is assigned to in a week
let student_slots_per_week(student: Student, week: Week) -> LinExpr =
    sum slot in @[Slot] { $StudentInSlot(student, slot, week) };

/// Check if a student has a subject in a given week
let has_subject_in_week(subject: Subject, student: Student, week: Week) -> Constraint =
    sum slot in subject.slots { $StudentInSlot(student, slot, week) } >== 1;

/// Ensure students don't exceed maximum slots per week
let max_slots_per_week(student: Student) -> Constraint =
    forall week in @[Week] {
        student_slots_per_week(student, week) <== student.max_slots
    };

/// Reify the maximum slots constraint
reify max_slots_per_week as $MaxSlots;

/// Ensure each student is assigned to exactly one slot per subject per week
let one_slot_per_subject_per_week(student: Student) -> Constraint =
    forall subject in @[Subject] {
        forall week in @[Week] {
            sum slot in subject.slots {
                $StudentInSlot(student, slot, week)
            } === 1
        }
    };

/// Reify the one slot per subject constraint
reify one_slot_per_subject_per_week as $OneSlotPerSubject;

/// Room capacity constraint
let room_capacity_check(room: Room) -> Constraint =
    forall week in @[Week] {
        forall slot in room.slots {
            sum student in @[Student] {
                $StudentInSlot(student, slot, week)
            } <== room.capacity
        }
    };

/// Reify room capacity
reify room_capacity_check as $RoomCapacity;

/// Public final constraint enforcing all rules
pub let enforce_schedule_rules() -> Constraint =
    forall student in @[Student] {
        $MaxSlots(student) === 1 and $OneSlotPerSubject(student) === 1
    } and forall room in @[Room] {
        $RoomCapacity(room) === 1
    };
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse complex realistic program: {:?}",
        result
    );
}

#[test]
fn file_accepts_program_with_varied_whitespace() {
    let program = r#"

let a()->LinExpr=5;


let    b   (   )   ->   LinExpr   =   10   ;

reify constraint as $Var;


pub   let   c   (   )   ->   Constraint   =   $Var()   ===   1   ;

"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse program with varied whitespace: {:?}",
        result
    );
}

#[test]
fn file_accepts_program_with_blank_lines() {
    let program = r#"
let a() -> LinExpr = 5;


let b() -> LinExpr = 10;



reify constraint as $Var;


"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse program with blank lines: {:?}",
        result
    );
}

#[test]
fn file_accepts_deeply_nested_expressions_in_program() {
    let program = r#"
let helper(x: Int) -> LinExpr = 
    if x > 0 { 
        sum y in @[Int] where y < x { 
            $V(y) 
        } 
    } else { 
        0 
    };

let main() -> Constraint = 
    forall x in @[Int] {
        forall y in @[Int] where y != x {
            helper(x) + helper(y) <== |@[Int]|
        }
    };

reify main as $Main;

pub let final() -> Constraint = $Main() === 1;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse program with deeply nested expressions: {:?}",
        result
    );
}

// =============================================================================
// NEGATIVE TESTS
// =============================================================================

#[test]
fn file_rejects_statement_without_semicolon() {
    let program = "let f() -> LinExpr = 5";
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_err(),
        "Should reject statement without semicolon: {:?}",
        result
    );
}

#[test]
fn file_rejects_invalid_statement_syntax() {
    let cases = vec![
        "let f() -> = 5;",               // missing type
        "f() -> LinExpr = 5;",           // missing 'let'
        "let -> LinExpr = 5;",           // missing name
        "reify as $Var;",                // missing constraint name
        "reify constraint as;",          // missing variable name
        "pub reify constraint as $Var;", // pub on reify (not allowed)
    ];

    for case in cases {
        let result = ColloMLParser::parse(Rule::file, case);
        assert!(
            result.is_err(),
            "Should reject invalid statement '{}': {:?}",
            case,
            result
        );
    }
}

#[test]
fn file_rejects_incomplete_program() {
    let program = r#"
let a() -> LinExpr = 5;
let b() -> LinExpr =
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_err(),
        "Should reject incomplete program: {:?}",
        result
    );
}

// =============================================================================
// MULTI-LINE COMMENT TESTS
// =============================================================================

#[test]
fn file_accepts_only_multiline_comments() {
    let program = "/* this is a multi-line comment */";
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse file with only multi-line comment: {:?}",
        result
    );
}

#[test]
fn file_accepts_multiline_comment_spanning_lines() {
    let program = r#"/* this comment
spans multiple
lines */"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse multi-line comment spanning lines: {:?}",
        result
    );
}

#[test]
fn file_accepts_multiline_comment_with_asterisks() {
    let program = r#"/*
 * This is a C-style block comment
 * with asterisks on each line
 * commonly used for documentation
 */"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse multi-line comment with asterisks: {:?}",
        result
    );
}

#[test]
fn file_accepts_statements_with_multiline_comments() {
    let program = r#"
/* Define a simple function */
let a() -> LinExpr = 5;

/* This function
   does something else */
let b() -> LinExpr = 10;

/* Reify the constraint */
reify constraint as $Var;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse statements with multi-line comments: {:?}",
        result
    );
}

#[test]
fn file_accepts_trailing_multiline_comment() {
    let program = "let f() -> Int = 5; /* trailing comment */";
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse trailing multi-line comment: {:?}",
        result
    );
}

#[test]
fn file_accepts_inline_multiline_comment() {
    let program = "let f() -> Int = /* inline */ 5;";
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse inline multi-line comment: {:?}",
        result
    );
}

#[test]
fn file_accepts_mixed_comment_styles() {
    let program = r#"
// Single-line comment
let a() -> LinExpr = 5;

/* Multi-line comment */
let b() -> LinExpr = 10;

/* Multi-line
   spanning lines */
let c() -> LinExpr = 15; // trailing single-line

let d() -> LinExpr = 20; /* trailing multi-line */
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse mixed comment styles: {:?}",
        result
    );
}

#[test]
fn file_accepts_multiline_comment_in_expression() {
    let program = r#"
let f(x: Int, y: Int) -> Int =
    x /* first operand */ + /* plus */ y /* second operand */;
"#;
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_ok(),
        "Should parse multi-line comments in expression: {:?}",
        result
    );
}

#[test]
fn file_rejects_unclosed_multiline_comment() {
    let program = "/* this comment is never closed";
    let result = ColloMLParser::parse(Rule::file, program);
    assert!(
        result.is_err(),
        "Should reject unclosed multi-line comment: {:?}",
        result
    );
}
