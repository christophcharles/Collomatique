use super::*;

#[test]
fn arg_accepts_simple_paths() {
    let cases = vec![
        "student",
        "week",
        "student.age",
        "course.teacher.name",
        "x.y.z",
    ];
    for case in cases {
        // Test in context of function call
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(result.is_ok(), "Should parse '{}': {:?}", fn_call, result);
    }
}

#[test]
fn arg_accepts_integer_literals() {
    let cases = vec!["42", "0", "-5", "123"];
    for case in cases {
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(result.is_ok(), "Should parse '{}': {:?}", fn_call, result);
    }
}

#[test]
fn arg_accepts_arithmetic_expressions() {
    let cases = vec![
        "student.age + 5",
        "week.number - 1",
        "x * 2",
        "(a + b) * 3",
        "student.weight // 2",
        "week.number % 7",
    ];
    for case in cases {
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(result.is_ok(), "Should parse '{}': {:?}", fn_call, result);
    }
}

#[test]
fn arg_accepts_cardinality() {
    let cases = vec![
        "|@[Student]|",
        "|collection|",
        "|subject.slots|",
        "|@[Week]| + 1",
    ];
    for case in cases {
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(result.is_ok(), "Should parse '{}': {:?}", fn_call, result);
    }
}

#[test]
fn arg_accepts_if_expressions() {
    let cases = vec![
        "if flag { 10 } else { 20 }",
        "if student.is_active { student.age } else { 0 }",
        "if x > 5 { x + 1 } else { x - 1 }",
    ];
    for case in cases {
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(result.is_ok(), "Should parse '{}': {:?}", fn_call, result);
    }
}

#[test]
fn arg_accepts_parenthesized_expressions() {
    let cases = vec!["(student.age)", "((x + y))", "(|@[Week]| - 1)"];
    for case in cases {
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(result.is_ok(), "Should parse '{}': {:?}", fn_call, result);
    }
}

#[test]
fn arg_accepts_multiple_computable_args() {
    let cases = vec![
        "compute(student, week.number)",
        "compute(student.age + 5, week)",
        "calculate(|@[Week]|, student.weight, if x { 1 } else { 0 })",
        "func(a, b + 1, |collection|, (x + y) * 2)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::fn_call_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}

#[test]
fn arg_rejects_variables() {
    let cases = vec!["$Var(x)", "$StudentInSlot(s, sl, w)"];
    for case in cases {
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(
            result.is_err(),
            "Should not parse '{}' (ILP variable): {:?}",
            fn_call,
            result
        );
    }
}

#[test]
fn arg_rejects_linear_expressions() {
    let cases = vec!["$V1(x) + $V2(y)", "2 * $Var(x)", "sum x in @[X]: $V(x)"];
    for case in cases {
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(
            result.is_err(),
            "Should not parse '{}' (linear expression): {:?}",
            fn_call,
            result
        );
    }
}

#[test]
fn arg_rejects_constraints() {
    let cases = vec!["$V(x) <= 10", "forall x in @[X]: $V(x) >= 0"];
    for case in cases {
        let fn_call = format!("compute({})", case);
        let result = ColloMLParser::parse(Rule::fn_call_complete, &fn_call);
        assert!(
            result.is_err(),
            "Should not parse '{}' (constraint): {:?}",
            fn_call,
            result
        );
    }
}

#[test]
fn var_call_accepts_computable_args() {
    let cases = vec![
        "$Var(student.age + 5)",
        "$StudentInSlot(student, slot, week.number)",
        "$Assigned(student, |@[Week]|)",
        "$Value(if x { 1 } else { 0 }, y)",
    ];
    for case in cases {
        let result = ColloMLParser::parse(Rule::var_call_complete, case);
        assert!(result.is_ok(), "Should parse '{}': {:?}", case, result);
    }
}
