use super::*;
use crate::parser::{ColloMLParser, Rule};
use pest::Parser;

fn analyze(
    input: &str,
    types: HashMap<String, ObjectFields>,
    vars: HashMap<String, ArgsType>,
) -> (TypeInfo, Vec<SemError>, Vec<SemWarning>) {
    let pairs = ColloMLParser::parse(Rule::file, input).expect("Parse failed");
    let file = crate::ast::File::from_pest(pairs.into_iter().next().unwrap())
        .expect("AST conversion failed");

    let mut global_env = GlobalEnv::new(types, vars).expect("GlobalEnv creation failed");

    global_env.expand(&file)
}

#[test]
fn test_simple_function_definition() {
    let input = "let add(x: Int, y: Int) -> LinExpr = x + y;";
    let (_type_info, errors, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 0, "Should have no errors: {:?}", errors);
    assert_eq!(warnings.len(), 0, "Should have no warnings: {:?}", warnings);
}

#[test]
fn test_unknown_type_in_parameter() {
    let input = "let f(x: UnknownType) -> LinExpr = 5;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], SemError::UnknownInputType { .. }));
}

#[test]
fn test_body_type_mismatch() {
    let input = "let f() -> LinExpr = 5 <= 10;"; // Constraint body, LinExpr expected
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], SemError::BodyTypeMismatch { .. }));
}

#[test]
fn test_duplicate_parameter() {
    let input = "let f(x: Int, x: Int) -> LinExpr = x;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0],
        SemError::ParameterAlreadyDefined { .. }
    ));
}

#[test]
fn test_unknown_variable_in_linexpr() {
    let input = "let f() -> Constraint = $UnknownVar(5) <= 10;";
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());

    assert!(errors.len() > 0);
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownVariable { .. })));
}

#[test]
fn test_variable_argument_type_mismatch() {
    let mut vars = HashMap::new();
    vars.insert("MyVar".to_string(), vec![InputType::Int]);

    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "let f() -> Constraint = $MyVar(5) <= 10;";
    let (_, errors, _) = analyze(input, types.clone(), vars.clone());

    assert_eq!(errors.len(), 0, "Should accept Int argument: {:?}", errors);

    // Wrong type
    let input2 = "let f(s: Student) -> Constraint = $MyVar(s) <= 10;";
    let (_, errors2, _) = analyze(input2, types, vars);

    assert!(errors2
        .iter()
        .any(|e| matches!(e, SemError::TypeMismatch { .. })));
}

#[test]
fn test_forall_with_collection() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "let f() -> Constraint = forall s in @[Student]: 0 <= 1;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors.is_empty(), "Should accept forall with Student type");
}

#[test]
fn test_naming_convention_warnings() {
    let input = "let MyFunction() -> LinExpr = 5;"; // PascalCase instead of snake_case
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        SemWarning::FunctionNamingConvention { .. }
    ));
}

#[test]
fn test_path_field_access() {
    let mut types = HashMap::new();
    let mut student_fields = HashMap::new();
    student_fields.insert("age".to_string(), InputType::Int);
    types.insert("Student".to_string(), student_fields);

    let input = "let f(s: Student) -> LinExpr = s.age;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert_eq!(
        errors.len(),
        0,
        "Should access field successfully: {:?}",
        errors
    );
}

#[test]
fn test_unknown_field_access() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "let f(s: Student) -> LinExpr = s.unknown_field;";
    let (_, errors, _) = analyze(input, types, HashMap::new());

    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnknownField { .. })));
}

#[test]
fn test_unused_parameter_warning() {
    let input = "let f(x: Int, y: Int) -> LinExpr = x;"; // y is unused
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    assert_eq!(warnings.len(), 1);
    assert!(matches!(warnings[0], SemWarning::UnusedIdentifier { .. }));
    if let SemWarning::UnusedIdentifier { identifier, .. } = &warnings[0] {
        assert_eq!(identifier, "y");
    }
}

#[test]
fn test_unused_forall_variable() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "let f() -> Constraint = forall s in @[Student]: 0 <= 1;"; // s unused
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(warnings
        .iter()
        .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })));
}

#[test]
fn test_unused_sum_variable() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let input = "let f() -> LinExpr = sum s in @[Student]: 5;"; // s unused
    let (_, _, warnings) = analyze(input, types, HashMap::new());

    assert!(warnings
        .iter()
        .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })));
    if let Some(SemWarning::UnusedIdentifier { identifier, .. }) = warnings
        .iter()
        .find(|w| matches!(w, SemWarning::UnusedIdentifier { .. }))
    {
        assert_eq!(identifier, "s");
    }
}

#[test]
fn test_no_warning_when_parameter_used() {
    let input = "let f(x: Int, y: Int) -> LinExpr = x + y;"; // Both used
    let (_, _, warnings) = analyze(input, HashMap::new(), HashMap::new());

    // Should only have warnings for naming conventions, not unused
    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn about unused when all parameters are used: {:?}",
        warnings
    );
}

#[test]
fn test_no_warning_when_forall_variable_used() {
    let mut types = HashMap::new();
    let mut student_fields = HashMap::new();
    student_fields.insert("age".to_string(), InputType::Int);
    types.insert("Student".to_string(), student_fields);

    let mut vars = HashMap::new();
    vars.insert(
        "V".to_string(),
        vec![InputType::Object("Student".to_string())],
    );

    let input = "let f() -> Constraint = forall s in @[Student]: $V(s) >= 0;"; // s is used
    let (_, _, warnings) = analyze(input, types, vars);

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn about unused when forall variable is used: {:?}",
        warnings
    );
}

#[test]
fn test_no_warning_when_sum_variable_used() {
    let mut types = HashMap::new();
    types.insert("Student".to_string(), HashMap::new());

    let mut vars = HashMap::new();
    vars.insert(
        "V".to_string(),
        vec![InputType::Object("Student".to_string())],
    );

    let input = "let f() -> LinExpr = sum s in @[Student]: $V(s);"; // s is used
    let (_, _, warnings) = analyze(input, types, vars);

    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedIdentifier { .. })),
        "Should not warn about unused when sum variable is used: {:?}",
        warnings
    );
}
