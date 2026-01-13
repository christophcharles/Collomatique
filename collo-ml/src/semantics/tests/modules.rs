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
        let check_value(x: Int) -> Constraint = x === 42;
        reify check_value as $the_value;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let use_private_var(x: Int) -> LinExpr = a::$the_value(x);
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

#[test]
#[ignore]
fn module_b_uses_public_reified_variable_from_module_a() {
    let mod_a = r#"
        pub let is_valid(x: Int) -> Constraint = x >== 0;
        pub reify is_valid as $validity_check;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let check_both(x: Int, y: Int) -> LinExpr = a::$validity_check(x) + a::$validity_check(y);
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
fn two_complex_modules_no_crosstalk() {
    let mod_a = r#"
        pub type Point = { x: Int, y: Int };
        pub let make_point(x: Int, y: Int) -> Point = Point { x: x, y: y };
        pub let distance(p: Point) -> Int = p.x + p.y;
        pub let is_origin(p: Point) -> Constraint = p.x === 0 && p.y === 0;
        pub reify is_origin as $OriginCheck;
    "#;
    let mod_b = r#"
        pub type Color = { r: Int, g: Int, b: Int };
        pub let make_color(r: Int, g: Int, b: Int) -> Color = Color { r: r, g: g, b: b };
        pub let brightness(c: Color) -> Int = c.r + c.g + c.b;
        pub let is_black(c: Color) -> Constraint = c.r === 0 && c.g === 0 && c.b === 0;
        pub reify is_black as $BlackCheck;
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
fn opaque_type_chaining_with_private_intermediate_type() {
    // Module A: private type MyType + public () -> MyType + public MyType -> Int
    // Module B: chains these functions - should pass even though type is private
    let mod_a = r#"
        type MyType = { value: Int };
        pub let make_my_type() -> MyType = MyType { value: 42 };
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
fn enum_variants_across_modules() {
    let mod_a = r#"
        pub enum Option = Some { value: Int } | None;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        pub let make_some(x: Int) -> a::Option = a::Option::Some { value: x };
        pub let make_none() -> a::Option = a::Option::None;
        pub let extract(opt: a::Option) -> Int = match opt {
            x as a::Option::Some { x.value }
            _x as a::Option::None { 0 }
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
fn private_function_publicly_reified_no_unused_warning() {
    let mod_a = r#"
        let private_constraint(x: Int) -> Constraint = x === 42;
        pub reify private_constraint as $PublicVar;
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
fn private_function_privately_reified_unused_warning() {
    let mod_a = r#"
        let private_constraint(x: Int) -> Constraint = x === 42;
        reify private_constraint as $private_var;
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
fn module_b_wraps_and_reifies_public_function_from_module_a() {
    let mod_a = r#"
        pub let check_value(x: Int) -> Constraint = x === 42;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        let local_check(x: Int) -> Constraint = a::check_value(x);
        pub reify local_check as $MyCheck;
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
#[ignore]
fn three_module_chain_function_reify_use() {
    // Module A: defines constraint and reifies it
    // Module B: wraps A's constraint, reifies the wrapper
    // Module C: uses the reified variable from B
    let mod_a = r#"
        pub let is_answer(x: Int) -> Constraint = x === 42;
        pub reify is_answer as $the_answer;
    "#;
    let mod_b = r#"
        import "mod_a" as a;
        let is_double_answer(x: Int) -> Constraint = a::is_answer(x) && a::is_answer(x * 2);
        pub reify is_double_answer as $double_check;
    "#;
    let mod_c = r#"
        import "mod_b" as b;
        pub let combined_check(x: Int, y: Int) -> LinExpr = b::$double_check(x) + b::$double_check(y);
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
