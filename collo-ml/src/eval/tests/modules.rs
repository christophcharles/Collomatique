use super::*;

/// Helper to compile multiple modules
fn compile_multi(modules: &[(&str, &str)]) -> CheckedAST<NoObject> {
    let inputs: BTreeMap<&str, &str> = modules.iter().copied().collect();
    CheckedAST::new(&inputs, HashMap::new()).expect("Should compile")
}

#[test]
fn eval_functions_in_separate_modules() {
    let mod_a = r#"pub let add(x: Int, y: Int) -> Int = x + y;"#;
    let mod_b = r#"pub let multiply(x: Int, y: Int) -> Int = x * y;"#;

    let checked_ast = compile_multi(&[("mod_a", mod_a), ("mod_b", mod_b)]);

    let result_a = checked_ast
        .quick_eval_fn("mod_a", "add", vec![ExprValue::Int(2), ExprValue::Int(3)])
        .expect("Should evaluate");
    assert_eq!(result_a, ExprValue::Int(5));

    let result_b = checked_ast
        .quick_eval_fn(
            "mod_b",
            "multiply",
            vec![ExprValue::Int(2), ExprValue::Int(3)],
        )
        .expect("Should evaluate");
    assert_eq!(result_b, ExprValue::Int(6));
}

#[test]
fn eval_cross_module_function_call() {
    let mod_a = r#"pub let add(x: Int, y: Int) -> Int = x + y;"#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let add_three(x: Int, y: Int, z: Int) -> Int = a::add(a::add(x, y), z);
    "#;

    let checked_ast = compile_multi(&[("mod_a", mod_a), ("mod_b", mod_b)]);

    let result = checked_ast
        .quick_eval_fn(
            "mod_b",
            "add_three",
            vec![ExprValue::Int(1), ExprValue::Int(2), ExprValue::Int(3)],
        )
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(6));
}

#[test]
fn eval_cross_module_struct_creation() {
    let mod_a = r#"pub type Point = { x: Int, y: Int };"#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let origin() -> a::Point = a::Point { x: 0, y: 0 };
    "#;

    let checked_ast = compile_multi(&[("mod_a", mod_a), ("mod_b", mod_b)]);

    let result = checked_ast
        .quick_eval_fn("mod_b", "origin", vec![])
        .expect("Should evaluate");

    match result {
        ExprValue::Custom(custom) => {
            assert_eq!(custom.type_name, "Point");
            assert!(custom.variant.is_none());
            match &custom.content {
                ExprValue::Struct(fields) => {
                    assert_eq!(fields.get("x"), Some(&ExprValue::Int(0)));
                    assert_eq!(fields.get("y"), Some(&ExprValue::Int(0)));
                }
                _ => panic!("Expected Struct content, got {:?}", custom.content),
            }
        }
        _ => panic!("Expected Custom, got {:?}", result),
    }
}

#[test]
fn eval_cross_module_enum_variant() {
    let mod_a = r#"pub enum Option = Some { value: Int } | None;"#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let make_some(x: Int) -> a::Option = a::Option::Some { value: x };
    "#;

    let checked_ast = compile_multi(&[("mod_a", mod_a), ("mod_b", mod_b)]);

    let result = checked_ast
        .quick_eval_fn("mod_b", "make_some", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    match result {
        ExprValue::Custom(custom) => {
            assert_eq!(custom.type_name, "Option");
            assert_eq!(custom.variant, Some("Some".to_string()));
            match &custom.content {
                ExprValue::Struct(fields) => {
                    assert_eq!(fields.get("value"), Some(&ExprValue::Int(42)));
                }
                _ => panic!("Expected Struct content, got {:?}", custom.content),
            }
        }
        _ => panic!("Expected Custom, got {:?}", result),
    }
}

#[test]
fn eval_cross_module_reified_variable() {
    let mod_a = r#"
        pub let check(x: Int) -> Constraint = x >== 0;
        pub reify check as $Check;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let use_check(x: Int) -> LinExpr = a::$Check(x);
    "#;

    let checked_ast = compile_multi(&[("mod_a", mod_a), ("mod_b", mod_b)]);

    let result = checked_ast
        .quick_eval_fn("mod_b", "use_check", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => {}
        _ => panic!("Expected LinExpr, got {:?}", result),
    }
}

#[test]
fn eval_cross_module_reified_variable_list() {
    let mod_a = r#"
        pub let checks(x: Int) -> [Constraint] = [x >== 0, x <== 10];
        pub reify checks as $[CheckList];
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let use_check_list(x: Int) -> [LinExpr] = a::$[CheckList](x);
    "#;

    let checked_ast = compile_multi(&[("mod_a", mod_a), ("mod_b", mod_b)]);

    let result = checked_ast
        .quick_eval_fn("mod_b", "use_check_list", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::List(items) => {
            assert_eq!(items.len(), 2);
            for item in &items {
                match item {
                    ExprValue::LinExpr(_) => {}
                    _ => panic!("Expected LinExpr in list, got {:?}", item),
                }
            }
        }
        _ => panic!("Expected List, got {:?}", result),
    }
}
