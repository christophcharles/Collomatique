use super::*;

// ========== Basic Tests (no cross-module references) ==========

#[test]
fn two_simple_modules_no_crosstalk() {
    let mod_a = r#"
        pub let add(x: Int, y: Int) -> Int = x + y;
    "#;
    let mod_b = r#"
        pub let multiply(x: Int, y: Int) -> Int = x * y;
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

// ========== Cross-Module Function Tests ==========

#[test]
fn module_b_uses_public_function_from_module_a() {
    let mod_a = r#"
        pub let add(x: Int, y: Int) -> Int = x + y;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let add_three(x: Int, y: Int, z: Int) -> Int = a::add(a::add(x, y), z);
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
fn module_b_uses_private_function_from_module_a_should_fail() {
    let mod_a = r#"
        let private_add(x: Int, y: Int) -> Int = x + y;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let use_private(x: Int, y: Int) -> Int = a::private_add(x, y);
    "#;

    let (_, errors, _) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(
        !errors.is_empty(),
        "Should fail when accessing private function"
    );
}

// ========== Cross-Module Type Tests ==========

#[test]
fn module_b_uses_public_type_from_module_a() {
    let mod_a = r#"
        pub type Point = { x: Int, y: Int };
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let origin() -> a::Point = a::Point { x: 0, y: 0 };
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
fn module_b_uses_private_type_from_module_a_should_fail() {
    let mod_a = r#"
        type PrivatePoint = { x: Int, y: Int };
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let origin() -> a::PrivatePoint = { x: 0, y: 0 };
    "#;

    let (_, errors, _) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(
        !errors.is_empty(),
        "Should fail when accessing private type"
    );
}

// ========== Cross-Module Variable Tests ==========

#[test]
fn module_b_uses_private_reified_variable_from_module_a_should_fail() {
    let mod_a = r#"
        pub let get_value() -> Int = 42;
        reify get_value as $the_value;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let double_value() -> Int = a::the_value + a::the_value;
    "#;

    let (_, errors, _) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(
        !errors.is_empty(),
        "Should fail when accessing private reified variable"
    );
}

// ========== Type Resolution Tests ==========

#[test]
fn module_b_uses_public_type_in_function_definition() {
    let mod_a = r#"
        pub type MyInt = Int;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let identity(x: a::MyInt) -> a::MyInt = x;
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
fn module_b_uses_private_type_in_function_definition_should_fail() {
    let mod_a = r#"
        type PrivateInt = Int;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let identity(x: a::PrivateInt) -> a::PrivateInt = x;
    "#;

    let (_, errors, _) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(
        !errors.is_empty(),
        "Should fail when using private type in function definition"
    );
}

// ========== Import Tests ==========

#[test]
fn wildcard_import_uses_function() {
    let mod_a = r#"
        pub let add(x: Int, y: Int) -> Int = x + y;
    "#;
    let mod_b = r#"
        import "mod_a" as *;
        pub let add_three(x: Int, y: Int, z: Int) -> Int = add(add(x, y), z);
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
fn wildcard_import_conflict_should_fail() {
    let mod_a = r#"
        pub let duplicate_fn() -> Int = 1;
    "#;
    let mod_b = r#"
        import "mod_a" as *;
        pub let duplicate_fn() -> Int = 2;
    "#;

    let (_, errors, _) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(
        !errors.is_empty(),
        "Should fail on duplicate function from wildcard import"
    );
}

#[test]
fn alias_shadowing_local_shadows_imported() {
    // Module B imports A with alias, defines local function with same name as imported
    // Local should shadow imported
    let mod_a = r#"
        pub let value() -> Int = 1;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        let value() -> Int = 2;
        pub let get_local() -> Int = value();
        pub let get_imported() -> Int = a::value();
    "#;

    let (_, errors, _warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    // Note: there might be a warning for unused local function, which is fine
}

// ========== Ignored Tests (require additional features) ==========
// The following tests are currently ignored because they depend on features
// that aren't fully implemented yet:
//
// 1. Reify type checking - The type of a reified variable needs to be inferred
//    from the function's return type, but currently the reify system expects
//    the type annotation which can be `Constraint`.
//
// 2. Struct coercion to custom types - When a function returns a custom type
//    and the body is a struct literal, the struct should be coerced to the
//    custom type. This requires struct-to-custom-type coercion.
//
// 3. Enum pattern matching with qualified paths - The parser doesn't support
//    qualified paths in match patterns (e.g., `a::Option::Some { ... }`).

#[test]
#[ignore = "TODO: Reify type checking needs function return type inference"]
fn module_b_uses_public_reified_variable_from_module_a() {
    let mod_a = r#"
        pub let get_value() -> Int = 42;
        pub reify get_value as $the_value;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let double_value() -> Int = a::the_value + a::the_value;
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
#[ignore = "TODO: Reify type checking needs function return type inference"]
fn two_complex_modules_no_crosstalk() {
    let mod_a = r#"
        pub type Point = { x: Int, y: Int };
        pub let make_point(x: Int, y: Int) -> Point = { x: x, y: y };
        pub let distance(p: Point) -> Int = p.x + p.y;
        pub let origin() -> Point = { x: 0, y: 0 };
        pub reify origin as $default_point;
    "#;
    let mod_b = r#"
        pub type Color = { r: Int, g: Int, b: Int };
        pub let make_color(r: Int, g: Int, b: Int) -> Color = { r: r, g: g, b: b };
        pub let brightness(c: Color) -> Int = c.r + c.g + c.b;
        pub let black() -> Color = { r: 0, g: 0, b: 0 };
        pub reify black as $default_color;
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
#[ignore = "TODO: Struct coercion to custom types not working in multi-module"]
fn opaque_type_chaining_with_private_intermediate_type() {
    // Module A: private type MyType + public () -> MyType + public MyType -> Int
    // Module B: chains these functions - should pass even though type is private
    let mod_a = r#"
        type MyType = { value: Int };
        pub let make_my_type() -> MyType = { value: 42 };
        pub let extract_value(x: MyType) -> Int = x.value;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let get_value() -> Int = a::extract_value(a::make_my_type());
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
#[ignore = "TODO: Parser doesn't support qualified paths in match patterns"]
fn enum_variants_across_modules() {
    let mod_a = r#"
        pub enum Option = Some { value: Int } | None;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let make_some(x: Int) -> a::Option = a::Option::Some { value: x };
        pub let make_none() -> a::Option = a::Option::None;
        pub let extract(opt: a::Option) -> Int = match opt {
            a::Option::Some { value } => value,
            a::Option::None => 0,
        };
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
#[ignore = "TODO: Reify type checking needs function return type inference"]
fn private_function_publicly_reified_no_unused_warning() {
    let mod_a = r#"
        let private_fn() -> Int = 42;
        pub reify private_fn as $public_var;
    "#;

    let (_, errors, warnings) = analyze_multi(&[("mod_a", mod_a)], HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings when private function is publicly reified: {:?}",
        warnings
    );
}

#[test]
#[ignore = "TODO: Reify type checking needs function return type inference"]
fn private_function_privately_reified_unused_warning() {
    let mod_a = r#"
        let private_fn() -> Int = 42;
        reify private_fn as $private_var;
    "#;

    let (_, errors, warnings) = analyze_multi(&[("mod_a", mod_a)], HashMap::new(), HashMap::new());

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        !warnings.is_empty(),
        "Should have unused warning for private variable: {:?}",
        warnings
    );
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, SemWarning::UnusedVariable { .. })),
        "Should have UnusedVariable warning: {:?}",
        warnings
    );
}

#[test]
#[ignore = "TODO: Reify type checking needs function return type inference"]
fn module_b_wraps_and_reifies_public_function_from_module_a() {
    let mod_a = r#"
        pub let get_value() -> Int = 42;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        let local_get_value() -> Int = a::get_value();
        pub reify local_get_value as $my_value;
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}

#[test]
#[ignore = "TODO: Reify type checking needs function return type inference"]
fn three_module_chain_function_reify_use() {
    // Module A: defines public function and reifies it
    // Module B: imports and re-exports as a variable
    // Module C: uses the reified variable from B
    let mod_a = r#"
        pub let get_answer() -> Int = 42;
        pub reify get_answer as $the_answer;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        let local_answer() -> Int = a::the_answer;
        pub reify local_answer as $b_answer;
    "#;
    let mod_c = r#"
        import "mod_b" as b;
        pub let double_answer() -> Int = b::b_answer + b::b_answer;
    "#;

    let (_, errors, warnings) = analyze_multi(
        &[("mod_a", mod_a), ("mod_b", mod_b), ("mod_c", mod_c)],
        HashMap::new(),
        HashMap::new(),
    );

    assert!(errors.is_empty(), "Should have no errors: {:?}", errors);
    assert!(
        warnings.is_empty(),
        "Should have no warnings: {:?}",
        warnings
    );
}
