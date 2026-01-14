use super::*;

// ========== Panic Expression Tests ==========

#[test]
fn panic_returns_error_with_int() {
    let input = "pub let f() -> Int = panic! 42;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast.quick_eval_fn("main", "f", vec![]);

    match result {
        Err(EvalError::Panic(value)) => {
            assert_eq!(*value, ExprValue::Int(42));
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn panic_returns_error_with_string() {
    let input = r#"pub let f() -> String = panic! "error message";"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast.quick_eval_fn("main", "f", vec![]);

    match result {
        Err(EvalError::Panic(value)) => {
            assert_eq!(*value, ExprValue::String("error message".to_string()));
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn panic_returns_error_with_bool() {
    let input = "pub let f() -> Bool = panic! true;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast.quick_eval_fn("main", "f", vec![]);

    match result {
        Err(EvalError::Panic(value)) => {
            assert_eq!(*value, ExprValue::Bool(true));
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn panic_evaluates_expression_first() {
    let input = "pub let f() -> Int = panic! (10 + 32);";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast.quick_eval_fn("main", "f", vec![]);

    match result {
        Err(EvalError::Panic(value)) => {
            assert_eq!(*value, ExprValue::Int(42));
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn panic_with_param() {
    let input = "pub let f(x: Int) -> Int = panic! x;";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast.quick_eval_fn("main", "f", vec![ExprValue::Int(99)]);

    match result {
        Err(EvalError::Panic(value)) => {
            assert_eq!(*value, ExprValue::Int(99));
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn panic_in_else_branch_triggers() {
    let input = r#"pub let f(x: Int) -> Int = if x > 0 { x } else { panic! 0 };"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    // x = -1, so else branch is taken
    let result = checked_ast.quick_eval_fn("main", "f", vec![ExprValue::Int(-1)]);

    match result {
        Err(EvalError::Panic(value)) => {
            assert_eq!(*value, ExprValue::Int(0));
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}

#[test]
fn panic_in_else_branch_not_triggered() {
    let input = r#"pub let f(x: Int) -> Int = if x > 0 { x } else { panic! 0 };"#;

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    // x = 5, so then branch is taken (panic not triggered)
    let result = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Int(5)])
        .expect("Should evaluate without panic");

    assert_eq!(result, ExprValue::Int(5));
}

#[test]
fn panic_with_list() {
    let input = "pub let f() -> [Int] = panic! [1, 2, 3];";

    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(
        &[ModuleSrc {
            name: "main".to_string(),
            src: input.to_string(),
        }],
        vars,
    )
    .expect("Should compile");

    let result = checked_ast.quick_eval_fn("main", "f", vec![]);

    match result {
        Err(EvalError::Panic(value)) => {
            assert_eq!(
                *value,
                ExprValue::List(vec![
                    ExprValue::Int(1),
                    ExprValue::Int(2),
                    ExprValue::Int(3)
                ])
            );
        }
        Ok(v) => panic!("Expected Panic error, got Ok({:?})", v),
        Err(e) => panic!("Expected Panic error, got {:?}", e),
    }
}
