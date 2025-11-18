use super::*;

// ========== Reify Statement Tests ==========

#[test]
fn reify_constraint_function() {
    let input = r#"
        pub let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Reify should work with constraint function: {:?}",
        errors
    );
}

#[test]
fn reify_function_with_parameters() {
    let types = simple_object("Student");
    let input = r#"
        pub let constraint(s: Student) -> Constraint = 0 === 1;
        reify constraint as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(
        errors.is_empty(),
        "Reify should work with parameterized constraint: {:?}",
        errors
    );
}

#[test]
fn reify_undefined_function() {
    let input = "reify undefined_func as $MyVar;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Should error on undefined function in reify"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownIdentifer { .. })));
}

#[test]
fn reify_non_constraint_function() {
    let input = r#"
        pub let not_constraint() -> Int = 42;
        reify not_constraint as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Should error when reifying non-constraint function"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::FunctionTypeMismatch { .. })));
}

#[test]
fn reify_linexpr_coerces_to_constraint() {
    let input = r#"
        pub let linexpr_func() -> LinExpr = 5;
        reify linexpr_func as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    // LinExpr should not coerce to Constraint in reify
    assert!(!errors.is_empty(), "LinExpr should not be reifiable");
}

#[test]
fn duplicate_variable_name() {
    let input = r#"
        pub let c1() -> Constraint = 0 === 1;
        pub let c2() -> Constraint = 0 === 2;
        reify c1 as $MyVar;
        reify c2 as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        !errors.is_empty(),
        "Should error on duplicate variable name"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::VariableAlreadyDefined { .. })));
}

#[test]
fn multiple_valid_reify_statements() {
    let input = r#"
        pub let c1() -> Constraint = 0 === 1;
        pub let c2() -> Constraint = 0 === 2;
        reify c1 as $Var1;
        reify c2 as $Var2;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Multiple reify statements should work: {:?}",
        errors
    );
}

// ========== Using Reified Variables ==========

#[test]
fn using_reified_variable_in_constraint() {
    let input = r#"
        pub let base(x: Int) -> Constraint = x === 1;
        reify base as $BaseVar;
        pub let use_var(x: Int) -> Constraint = $BaseVar(x) === 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Should be able to use reified variable: {:?}",
        errors
    );
}

#[test]
fn using_undefined_variable() {
    let input = "pub let f(x: Int) -> Constraint = $UndefinedVar(x) === 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error on undefined variable");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownVariable { .. })));
}

#[test]
fn variable_call_with_wrong_arguments() {
    let input = r#"
        pub let base(x: Int, y: Int) -> Constraint = x === y;
        reify base as $BaseVar;
        pub let use_var(x: Int) -> Constraint = $BaseVar(x) === 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error on wrong argument count");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::ArgumentCountMismatch { .. })));
}

#[test]
fn variable_call_with_wrong_types() {
    let input = r#"
        pub let base(x: Int) -> Constraint = x === 0;
        reify base as $BaseVar;
        pub let use_var() -> Constraint = $BaseVar(true) === 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(!errors.is_empty(), "Should error on wrong argument type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

// ========== Pre-defined Variables ==========

#[test]
fn using_predefined_variable() {
    let vars = var_with_args("PredefinedVar", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> Constraint = $PredefinedVar(x) === 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Should be able to use predefined variable: {:?}",
        errors
    );
}

#[test]
fn predefined_variable_with_object_type() {
    let types = simple_object("Student");
    let vars = var_with_args("StudentVar", vec![ExprType::Object("Student".to_string())]);

    let input = "pub let f(s: Student) -> Constraint = $StudentVar(s) === 0;";
    let (_, errors, _) = analyze(input, types, vars);

    assert!(
        errors.is_empty(),
        "Predefined variable with object type should work: {:?}",
        errors
    );
}

#[test]
fn predefined_variable_with_multiple_args() {
    let types = simple_object("Student");
    let vars = var_with_args(
        "MultiVar",
        vec![ExprType::Object("Student".to_string()), ExprType::Int],
    );

    let input = "pub let f(s: Student, x: Int) -> Constraint = $MultiVar(s, x) === 0;";
    let (_, errors, _) = analyze(input, types, vars);

    assert!(
        errors.is_empty(),
        "Multi-argument predefined variable should work: {:?}",
        errors
    );
}

// ========== Variable Returns LinExpr ==========

#[test]
fn variable_call_returns_linexpr() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = $V(x);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Variable call should return LinExpr: {:?}",
        errors
    );
}

#[test]
fn variable_call_in_arithmetic() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = $V(x) + 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Variable call should work in arithmetic: {:?}",
        errors
    );
}

#[test]
fn variable_call_in_constraint() {
    let vars = var_with_args("V", vec![ExprType::Int]);
    let input = "pub let f(x: Int) -> Constraint = $V(x) === 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars);

    assert!(
        errors.is_empty(),
        "Variable call should work in constraints: {:?}",
        errors
    );
}

// ========== Let Statement Variations ==========

#[test]
fn let_with_docstring() {
    let input = r#"
        ## This is a docstring
        pub let f() -> Int = 42;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with docstring should work: {:?}",
        errors
    );
}

#[test]
fn let_with_multiple_docstrings() {
    let input = r#"
        ## First line
        ## Second line
        ## Third line
        pub let f() -> Int = 42;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Let with multiple docstrings should work: {:?}",
        errors
    );
}

#[test]
fn reify_with_docstring() {
    let input = r#"
        pub let c() -> Constraint = 0 === 1;
        ## Docstring for reify
        reify c as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Reify with docstring should work: {:?}",
        errors
    );
}

// ========== Complex Statement Sequences ==========

#[test]
fn multiple_lets_and_reifies() {
    let input = r#"
        pub let helper(x: Int) -> Int = x;
        pub let c1(x: Int) -> Constraint = helper(x) === 0;
        pub let c2(x: Int) -> Constraint = helper(x) === 1;
        reify c1 as $Var1;
        reify c2 as $Var2;
        pub let combined(x: Int) -> Constraint = $Var1(x) <== 1 and $Var2(x) >== 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(
        errors.is_empty(),
        "Complex statement sequence should work: {:?}",
        errors
    );
}

#[test]
fn forward_declaration_not_allowed() {
    let input = r#"
        pub let use_func() -> Int = helper(5);
        pub let helper(x: Int) -> Int = x;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    // Forward declaration should fail
    assert!(!errors.is_empty(), "Forward declaration should not work");
}
