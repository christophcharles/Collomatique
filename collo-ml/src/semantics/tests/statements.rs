use super::*;

// ========== Reify Statement Tests ==========

#[tokio::test]
async fn reify_constraint_function() {
    let input = r#"
        pub let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Reify should work with constraint function: {:?}",
        errors
    );
}

#[tokio::test]
async fn reify_constraint_list() {
    let input = r#"
        pub let my_constraints() -> [Constraint] = [0 === 1, 1 <== 2];
        reify my_constraints as $[MyVars];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Reify should work with constraint list function: {:?}",
        errors
    );
}

#[tokio::test]
async fn reify_function_with_parameters() {
    let types = simple_object("Student");
    let input = r#"
        pub let constraint(s: Student) -> Constraint = 0 === 1;
        reify constraint as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, types, HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Reify should work with parameterized constraint: {:?}",
        errors
    );
}

#[tokio::test]
async fn reify_function_with_parameters_into_var_list() {
    let input = r#"
        pub let constraints(s: Int) -> [Constraint] = [0 === 1, 0 <== s];
        reify constraints as $[MyVars];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Reify should work with parameterized constraint lists: {:?}",
        errors
    );
}

#[tokio::test]
async fn disallow_reify_constraint_list_into_simple_var() {
    let input = r#"
        pub let my_constraints() -> [Constraint] = [0 === 1, 1 <== 2];
        reify my_constraints as $MyVars;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Reify should not work with constraint list without a var_list: {:?}",
        errors
    );
}

#[tokio::test]
async fn disallow_reify_constraint_into_var_list() {
    let input = r#"
        pub let my_constraint() -> Constraint = 0 === 1;
        reify my_constraint as $[MyVars];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Reify should not work with single constraint into a var_list: {:?}",
        errors
    );
}

#[tokio::test]
async fn reify_undefined_function() {
    let input = "reify undefined_func as $MyVar;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Should error on undefined function in reify"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownIdentifer { .. })));
}

#[tokio::test]
async fn reify_non_constraint_function() {
    let input = r#"
        pub let not_constraint() -> Int = 42;
        reify not_constraint as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Should error when reifying non-constraint function"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::FunctionTypeMismatch { .. })));
}

#[tokio::test]
async fn reify_linexpr_coerces_to_constraint() {
    let input = r#"
        pub let linexpr_func() -> LinExpr = 5;
        reify linexpr_func as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    // LinExpr should not coerce to Constraint in reify
    assert!(!errors.is_empty(), "LinExpr should not be reifiable");
}

#[tokio::test]
async fn duplicate_variable_name() {
    let input = r#"
        pub let c1() -> Constraint = 0 === 1;
        pub let c2() -> Constraint = 0 === 2;
        reify c1 as $MyVar;
        reify c2 as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Should error on duplicate variable name"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::VariableAlreadyDefined { .. })));
}

#[tokio::test]
async fn multiple_valid_reify_statements() {
    let input = r#"
        pub let c1() -> Constraint = 0 === 1;
        pub let c2() -> Constraint = 0 === 2;
        reify c1 as $Var1;
        reify c2 as $Var2;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Multiple reify statements should work: {:?}",
        errors
    );
}

// ========== Using Reified Variables ==========

#[tokio::test]
async fn using_reified_variable_in_constraint() {
    let input = r#"
        pub let base(x: Int) -> Constraint = x === 1;
        reify base as $BaseVar;
        pub let use_var(x: Int) -> Constraint = $BaseVar(x) === 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Should be able to use reified variable: {:?}",
        errors
    );
}

#[tokio::test]
async fn using_undefined_variable() {
    let input = "pub let f(x: Int) -> Constraint = $UndefinedVar(x) === 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Should error on undefined variable");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownVariable { .. })));
}

#[tokio::test]
async fn variable_call_with_wrong_arguments() {
    let input = r#"
        pub let base(x: Int, y: Int) -> Constraint = x === y;
        reify base as $BaseVar;
        pub let use_var(x: Int) -> Constraint = $BaseVar(x) === 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Should error on wrong argument count");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::ArgumentCountMismatch { .. })));
}

#[tokio::test]
async fn variable_call_with_wrong_types() {
    let input = r#"
        pub let base(x: Int) -> Constraint = x === 0;
        reify base as $BaseVar;
        pub let use_var() -> Constraint = $BaseVar(true) === 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Should error on wrong argument type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

// ========== Pre-defined Variables ==========

#[tokio::test]
async fn using_predefined_variable() {
    let vars = var_with_args("PredefinedVar", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> Constraint = $PredefinedVar(x) === 0;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars).await;

    assert!(
        errors.is_empty(),
        "Should be able to use predefined variable: {:?}",
        errors
    );
}

#[tokio::test]
async fn predefined_variable_with_object_type() {
    let types = simple_object("Student");
    let vars = var_with_args(
        "StudentVar",
        vec![SimpleType::Object("Student".to_string())],
    );

    let input = "pub let f(s: Student) -> Constraint = $StudentVar(s) === 0;";
    let (_, errors, _) = analyze(input, types, vars).await;

    assert!(
        errors.is_empty(),
        "Predefined variable with object type should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn predefined_variable_with_multiple_args() {
    let types = simple_object("Student");
    let vars = var_with_args(
        "MultiVar",
        vec![SimpleType::Object("Student".to_string()), SimpleType::Int],
    );

    let input = "pub let f(s: Student, x: Int) -> Constraint = $MultiVar(s, x) === 0;";
    let (_, errors, _) = analyze(input, types, vars).await;

    assert!(
        errors.is_empty(),
        "Multi-argument predefined variable should work: {:?}",
        errors
    );
}

// ========== Variable Returns LinExpr ==========

#[tokio::test]
async fn variable_call_returns_linexpr() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = $V(x);";
    let (_, errors, _) = analyze(input, HashMap::new(), vars).await;

    assert!(
        errors.is_empty(),
        "Variable call should return LinExpr: {:?}",
        errors
    );
}

#[tokio::test]
async fn variable_call_in_arithmetic() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> LinExpr = $V(x) + 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars).await;

    assert!(
        errors.is_empty(),
        "Variable call should work in arithmetic: {:?}",
        errors
    );
}

#[tokio::test]
async fn variable_call_in_constraint() {
    let vars = var_with_args("V", vec![SimpleType::Int]);
    let input = "pub let f(x: Int) -> Constraint = $V(x) === 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), vars).await;

    assert!(
        errors.is_empty(),
        "Variable call should work in constraints: {:?}",
        errors
    );
}

// ========== Let Statement Variations ==========

#[tokio::test]
async fn let_with_docstring() {
    let input = r#"
        /// This is a docstring
        pub let f() -> Int = 42;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Let with docstring should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn let_with_multiple_docstrings() {
    let input = r#"
        /// First line
        /// Second line
        /// Third line
        pub let f() -> Int = 42;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Let with multiple docstrings should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn reify_with_docstring() {
    let input = r#"
        pub let c() -> Constraint = 0 === 1;
        /// Docstring for reify
        reify c as $MyVar;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Reify with docstring should work: {:?}",
        errors
    );
}

// ========== Complex Statement Sequences ==========

#[tokio::test]
async fn multiple_lets_and_reifies() {
    let input = r#"
        pub let helper(x: Int) -> Int = x;
        pub let c1(x: Int) -> Constraint = helper(x) === 0;
        pub let c2(x: Int) -> Constraint = helper(x) === 1;
        reify c1 as $Var1;
        reify c2 as $Var2;
        pub let combined(x: Int) -> Constraint = $Var1(x) <== 1 and $Var2(x) >== 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Complex statement sequence should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn forward_declaration_now_allowed() {
    // Forward references to functions are now allowed
    let input = r#"
        pub let use_func() -> Int = helper(5);
        pub let helper(x: Int) -> Int = x;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Forward declaration should now be allowed: {:?}",
        errors
    );
}

// ========== Query Statement Tests ==========

#[tokio::test]
async fn query_with_db_param_and_option_struct_return() {
    let input = r#"
        pub query get_student(db: #{"CREATE TABLE students(id INTEGER, name TEXT)"}, id: Int) -> ?{name: String} = "SELECT name FROM students WHERE id = ?";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Query with db param and option struct return should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_with_db_param_and_list_struct_return() {
    let input = r#"
        pub query all_students(db: #{"CREATE TABLE students(id INTEGER, name TEXT)"}) -> [{id: Int, name: String}] = "SELECT id, name FROM students";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Query with db param and list struct return should work: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_callable_from_function() {
    let input = r#"
        type MyDb = #{"CREATE TABLE students(id INTEGER, name TEXT)"};
        pub query get_student(db: MyDb, id: Int) -> ?{name: String} = "SELECT name FROM students WHERE id = ?";
        pub let wrapper(db: MyDb, id: Int) -> ?{name: String} = get_student(db, id);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        errors.is_empty(),
        "Query should be callable from function: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_wrong_argument_count() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        query get_row(db: MyDb, id: Int) -> ?{id: Int} = "SELECT id FROM t WHERE id = ?";
        pub let wrapper(db: MyDb) -> ?{id: Int} = get_row(db);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Should error on wrong argument count");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::ArgumentCountMismatch { .. })));
}

#[tokio::test]
async fn query_wrong_argument_type() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        query get_row(db: MyDb, id: Int) -> ?{id: Int} = "SELECT id FROM t WHERE id = ?";
        pub let wrapper(db: MyDb) -> ?{id: Int} = get_row(db, "not an int");
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Should error on wrong argument type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[tokio::test]
async fn query_duplicate_name_with_query() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        query my_query(db: MyDb) -> [{id: Int}] = "SELECT id FROM t";
        query my_query(db: MyDb) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(
        !errors.is_empty(),
        "Should error when query has same name as another query"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryAlreadyDefined { .. })));
}

#[tokio::test]
async fn unused_private_query_warning() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        query unused_query(db: MyDb) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(errors.is_empty(), "Should not have errors: {:?}", errors);
    assert!(
        !warnings.is_empty(),
        "Should warn about unused private query"
    );
    assert!(warnings
        .iter()
        .any(|w| matches!(w, SemWarning::UnusedQuery { .. })));
}

#[tokio::test]
async fn public_query_not_warned_as_unused() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query public_query(db: MyDb) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, warnings) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(errors.is_empty(), "Should not have errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Public query should not generate unused warning: {:?}",
        warnings
    );
}

// ========== Symbol Conflict Tests ==========

#[tokio::test]
async fn function_conflicts_with_type() {
    let input = r#"
        type my_name = Int;
        pub let my_name() -> Int = 42;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Function should conflict with type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::SymbolConflict { .. })));
}

#[tokio::test]
async fn query_conflicts_with_type() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        type my_name = Int;
        pub query my_name(db: MyDb) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Query should conflict with type");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::SymbolConflict { .. })));
}

#[tokio::test]
async fn query_conflicts_with_function() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub let my_name() -> Int = 42;
        pub query my_name(db: MyDb) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Query should conflict with function");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::SymbolConflict { .. })));
}

#[tokio::test]
async fn function_conflicts_with_query() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query my_name(db: MyDb) -> [{id: Int}] = "SELECT id FROM t";
        pub let my_name() -> Int = 42;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;

    assert!(!errors.is_empty(), "Function should conflict with query");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::SymbolConflict { .. })));
}

// ========== Query Output Type Validation Tests ==========

// --- Valid output types ---

#[tokio::test]
async fn query_output_list_struct_direct() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> [{name: String}] = "SELECT name FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Direct list of struct should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_optional_struct_direct() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> ?{name: String} = "SELECT name FROM t LIMIT 1";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Direct optional struct should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_custom_alias_in_list() {
    let input = r#"
        type T = {name: String};
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> [T] = "SELECT name FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Custom alias inside list should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_custom_alias_optional() {
    let input = r#"
        type T = {name: String};
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> ?T = "SELECT name FROM t LIMIT 1";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Custom alias in optional should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_alias_wrapping_optional() {
    let input = r#"
        type T = ?{name: String};
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> T = "SELECT name FROM t LIMIT 1";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Alias wrapping optional struct should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_alias_wrapping_list() {
    let input = r#"
        type T = [{name: String}];
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> T = "SELECT name FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Alias wrapping list of struct should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_none_union_alias() {
    let input = r#"
        type T = {name: String};
        type U = None | T;
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> U = "SELECT name FROM t LIMIT 1";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Union through alias (None | Struct) should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_nested_alias() {
    let input = r#"
        type T = {name: String};
        type U = ?T;
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> U = "SELECT name FROM t LIMIT 1";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Chained alias (?T where T is struct) should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_enum_none_struct_variant() {
    let input = r#"
        enum E = None | V{name: String};
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> E = "SELECT name FROM t LIMIT 1";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Enum with None + struct variant should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_alias_chain_none() {
    let input = r#"
        type MyNone = None;
        type T = {name: String};
        type U = MyNone | T;
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> U = "SELECT name FROM t LIMIT 1";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Alias to None in union should be valid: {:?}",
        errors
    );
}

// --- Invalid output types ---

#[tokio::test]
async fn query_output_int_rejected() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> Int = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "Int output type should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryInvalidOutputType { .. })));
}

#[tokio::test]
async fn query_output_list_int_rejected() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> [Int] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "List of Int output type should be rejected"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryInvalidOutputType { .. })));
}

#[tokio::test]
async fn query_output_optional_int_rejected() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> ?Int = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Optional Int output type should be rejected"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryInvalidOutputType { .. })));
}

#[tokio::test]
async fn query_output_bare_struct_rejected() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> {name: String} = "SELECT name FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Bare struct output type should be rejected"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryInvalidOutputType { .. })));
}

#[tokio::test]
async fn query_output_enum_multi_unit_rejected() {
    let input = r#"
        enum E = A | B | S{name: String};
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> E = "SELECT name FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Enum with 2 unit + 1 struct variant should be rejected (3 resolved variants)"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryInvalidOutputType { .. })));
}

// ========== Query First Parameter Validation Tests ==========

// --- Valid first parameter ---

#[tokio::test]
async fn query_first_param_direct_schema() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Direct database schema param should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_first_param_alias() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query q(db: MyDb) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Alias to database schema param should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_first_param_nested_alias() {
    let input = r#"
        type A = #{"CREATE TABLE t(id INTEGER)"};
        type B = A;
        pub query q(db: B) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Nested alias to database schema param should be valid: {:?}",
        errors
    );
}

// --- Invalid first parameter ---

#[tokio::test]
async fn query_first_param_int_rejected() {
    let input = r#"
        pub query q(db: Int) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "Int as first param should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryFirstParamNotDatabase { .. })));
}

#[tokio::test]
async fn query_first_param_struct_rejected() {
    let input = r#"
        pub query q(db: {name: String}) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Struct as first param should be rejected"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryFirstParamNotDatabase { .. })));
}

#[tokio::test]
async fn query_no_params_rejected() {
    let input = r#"
        pub query q() -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        !errors.is_empty(),
        "Query with no params should be rejected"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryMissingDatabaseParam { .. })));
}

// ========== Query Parameter SQL-Compatibility Tests ==========

// --- Valid parameter types ---

#[tokio::test]
async fn query_param_int() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query q(db: MyDb, id: Int) -> [{id: Int}] = "SELECT id FROM t WHERE id = ?";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(errors.is_empty(), "Int param should be valid: {:?}", errors);
}

#[tokio::test]
async fn query_param_bool() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER, active BOOLEAN)"};
        pub query q(db: MyDb, flag: Bool) -> [{id: Int}] = "SELECT id FROM t WHERE active = ?";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Bool param should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_param_string() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER, name TEXT)"};
        pub query q(db: MyDb, name: String) -> [{id: Int}] = "SELECT id FROM t WHERE name = ?";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "String param should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_param_optional_int() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query q(db: MyDb, id: ?Int) -> [{id: Int}] = "SELECT id FROM t WHERE id = ?";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Optional Int param should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_param_alias_to_int() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        type MyId = Int;
        pub query q(db: MyDb, id: MyId) -> [{id: Int}] = "SELECT id FROM t WHERE id = ?";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Alias to Int param should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_param_enum_none_string() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER, name TEXT)"};
        enum MaybeName = None | Name(String);
        pub query q(db: MyDb, n: MaybeName) -> [{id: Int}] = "SELECT id FROM t WHERE name = ?";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Enum resolving to ?String param should be valid: {:?}",
        errors
    );
}

// --- Invalid parameter types ---

#[tokio::test]
async fn query_param_struct_rejected() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query q(db: MyDb, s: {name: String}) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "Struct param should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryParamNotSqlCompatible { .. })));
}

#[tokio::test]
async fn query_param_list_rejected() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query q(db: MyDb, ids: [Int]) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "List param should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryParamNotSqlCompatible { .. })));
}

#[tokio::test]
async fn query_param_linexpr_rejected() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query q(db: MyDb, x: LinExpr) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "LinExpr param should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryParamNotSqlCompatible { .. })));
}

#[tokio::test]
async fn query_param_constraint_rejected() {
    let input = r#"
        type MyDb = #{"CREATE TABLE t(id INTEGER)"};
        pub query q(db: MyDb, c: Constraint) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "Constraint param should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryParamNotSqlCompatible { .. })));
}

// ========== Query Output Field SQL-Compatibility Tests ==========

// --- Valid output struct field types ---

#[tokio::test]
async fn query_output_field_int() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> [{id: Int}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(errors.is_empty(), "Int field should be valid: {:?}", errors);
}

#[tokio::test]
async fn query_output_field_optional_string() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> [{name: ?String}] = "SELECT name FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Optional String field should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_field_alias_to_int() {
    let input = r#"
        type MyId = Int;
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> [{id: MyId}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Alias to Int field should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_field_enum_nullable() {
    let input = r#"
        enum MaybeName = None | Name(String);
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> [{name: MaybeName}] = "SELECT name FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Enum resolving to ?String field should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_field_enum_variant() {
    let input = r#"
        enum MaybeName = None | Name(String);
        pub query q(db: #{"CREATE TABLE t(name TEXT)"}) -> [{name: MaybeName::Name}] = "SELECT name FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Enum variant resolving to String field should be valid: {:?}",
        errors
    );
}

#[tokio::test]
async fn query_output_field_optional_struct() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER, name TEXT)"}) -> ?{id: Int, name: String} = "SELECT id, name FROM t LIMIT 1";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(
        errors.is_empty(),
        "Fields in optional struct should be valid: {:?}",
        errors
    );
}

// --- Invalid output struct field types ---

#[tokio::test]
async fn query_output_field_struct_rejected() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> [{nested: {a: Int}}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "Struct field should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryOutputFieldNotSqlCompatible { .. })));
}

#[tokio::test]
async fn query_output_field_list_rejected() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> [{ids: [Int]}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "List field should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryOutputFieldNotSqlCompatible { .. })));
}

#[tokio::test]
async fn query_output_field_linexpr_rejected() {
    let input = r#"
        pub query q(db: #{"CREATE TABLE t(id INTEGER)"}) -> [{x: LinExpr}] = "SELECT id FROM t";
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new()).await;
    assert!(!errors.is_empty(), "LinExpr field should be rejected");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::QueryOutputFieldNotSqlCompatible { .. })));
}
