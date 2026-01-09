use super::*;

// =============================================================================
// FUNCTION FORWARD REFERENCES
// =============================================================================

#[test]
fn function_forward_reference_simple() {
    let input = r#"
        pub let f() -> Int = g();
        let g() -> Int = 42;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Forward reference to function should be allowed: {:?}",
        errors
    );
}

#[test]
fn function_forward_reference_with_params() {
    let input = r#"
        pub let f(x: Int) -> Int = g(x, 10);
        let g(a: Int, b: Int) -> Int = a + b;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Forward reference with parameters should work: {:?}",
        errors
    );
}

#[test]
fn function_forward_reference_chain() {
    let input = r#"
        pub let a() -> Int = b();
        let b() -> Int = c();
        let c() -> Int = 42;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Chain of forward references should work: {:?}",
        errors
    );
}

// =============================================================================
// DIRECT RECURSION
// =============================================================================

#[test]
fn direct_recursion_factorial() {
    let input = r#"
        pub let factorial(n: Int) -> Int =
            if n == 0 { 1 } else { n * factorial(n - 1) };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Direct recursion (factorial) should be allowed: {:?}",
        errors
    );
}

#[test]
fn direct_recursion_countdown() {
    let input = r#"
        pub let countdown(n: Int) -> Int =
            if n == 0 { 0 } else { countdown(n - 1) };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Direct recursion with countdown should work: {:?}",
        errors
    );
}

#[test]
fn direct_recursion_constraint_function() {
    let input = r#"
        pub let recursive_constraint(n: Int) -> Constraint =
            if n == 0 { 0 === 0 } else { n >== 0 and recursive_constraint(n - 1) };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Recursive constraint function should work: {:?}",
        errors
    );
}

// =============================================================================
// MUTUAL RECURSION
// =============================================================================

#[test]
fn mutual_recursion_even_odd() {
    let input = r#"
        pub let is_even(n: Int) -> Bool = if n == 0 { true } else { is_odd(n - 1) };
        pub let is_odd(n: Int) -> Bool = if n == 0 { false } else { is_even(n - 1) };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Mutual recursion (even/odd) should be allowed: {:?}",
        errors
    );
}

#[test]
fn mutual_recursion_three_functions() {
    let input = r#"
        let a(n: Int) -> Int = if n == 0 { 0 } else { b(n - 1) };
        let b(n: Int) -> Int = if n == 0 { 1 } else { c(n - 1) };
        let c(n: Int) -> Int = if n == 0 { 2 } else { a(n - 1) };
        pub let start(n: Int) -> Int = a(n);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Three-way mutual recursion should work: {:?}",
        errors
    );
}

// =============================================================================
// TYPE FORWARD REFERENCES
// =============================================================================

#[test]
fn type_forward_reference_simple() {
    let input = r#"
        type A = [B];
        type B = Int;
        let f() -> A = [5 into B] into A;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Forward reference to type should be allowed: {:?}",
        errors
    );
}

#[test]
fn type_forward_reference_in_function() {
    let input = r#"
        let f() -> B = 5 into B;
        type B = Int;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Function using forward-referenced type should work: {:?}",
        errors
    );
}

#[test]
fn type_forward_reference_chain() {
    let input = r#"
        type A = [B];
        type B = (C, Int);
        type C = Bool;
        let f() -> A = [(true into C, 5) into B] into A;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Chain of type forward references should work: {:?}",
        errors
    );
}

// =============================================================================
// GUARDED RECURSIVE TYPES (ALLOWED)
// =============================================================================

#[test]
fn guarded_recursion_in_list() {
    let input = r#"
        type Tree = [Tree];
        let empty_tree() -> Tree = [] into Tree;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Self-reference inside list is guarded and allowed: {:?}",
        errors
    );
}

#[test]
fn guarded_recursion_in_tuple() {
    let input = r#"
        type LinkedList = (Int, ?LinkedList);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Self-reference inside tuple is guarded and allowed: {:?}",
        errors
    );
}

#[test]
fn guarded_recursion_tree_structure() {
    let input = r#"
        type Tree = (Int, [Tree]);
        let value(t: Tree) -> Int = t.0;
        let children(t: Tree) -> [Tree] = t.1;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Tree structure with guarded recursion should work: {:?}",
        errors
    );
}

#[test]
fn guarded_recursion_union_with_list() {
    let input = r#"
        type MyList = Int | [MyList];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Union with recursion inside list is guarded: {:?}",
        errors
    );
}

#[test]
fn guarded_recursion_union_with_tuple() {
    let input = r#"
        type Expr = Int | (String, Expr);
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Union with recursion inside tuple is guarded: {:?}",
        errors
    );
}

#[test]
fn guarded_mutual_recursion() {
    let input = r#"
        type A = [B];
        type B = [A];
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Mutual recursion inside lists is guarded: {:?}",
        errors
    );
}

#[test]
fn guarded_recursion_nested_containers() {
    let input = r#"
        type Nested = ([Nested], (Int, ?Nested));
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Recursion in nested containers is guarded: {:?}",
        errors
    );
}

// =============================================================================
// UNGUARDED RECURSIVE TYPES (ERROR)
// =============================================================================

#[test]
fn error_unguarded_direct_recursion() {
    let input = r#"
        type MyType = MyType;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(!errors.is_empty(), "Direct self-reference should error");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnguardedRecursiveType { .. })));
}

#[test]
fn error_unguarded_union_recursion() {
    let input = r#"
        type MyType = Int | MyType;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Bare self-reference in union should error"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnguardedRecursiveType { .. })));
}

#[test]
fn error_unguarded_union_recursion_multiple_variants() {
    let input = r#"
        type MyType = Int | Bool | MyType | String;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Bare self-reference among multiple variants should error"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnguardedRecursiveType { .. })));
}

#[test]
fn error_unguarded_mutual_recursion() {
    let input = r#"
        type A = Int | B;
        type B = String | A;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Mutual unguarded recursion should error"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnguardedRecursiveType { .. })));
}

#[test]
fn error_unguarded_transitive_recursion() {
    let input = r#"
        type A = B;
        type B = C;
        type C = A;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Transitive unguarded recursion should error"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnguardedRecursiveType { .. })));
}

#[test]
fn error_unguarded_through_custom_type() {
    let input = r#"
        type Wrapper = Inner;
        type Inner = Int | Wrapper;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        !errors.is_empty(),
        "Unguarded recursion through alias should error"
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, SemError::UnguardedRecursiveType { .. })));
}

// =============================================================================
// REIFICATION FORWARD REFERENCES
// =============================================================================

#[test]
fn reify_forward_reference() {
    let input = r#"
        reify constraint_fn as $Var;
        pub let constraint_fn() -> Constraint = $Var() === 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Reify forward reference should be allowed: {:?}",
        errors
    );
}

#[test]
fn reify_forward_reference_with_params() {
    let input = r#"
        reify my_constraint as $MyVar;
        pub let my_constraint(x: Int) -> Constraint = $MyVar(x) >== 0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Reify forward reference with params should work: {:?}",
        errors
    );
}

#[test]
fn reify_forward_reference_list() {
    let input = r#"
        reify constraints as $[Vars];
        pub let constraints(x: Int) -> [Constraint] = [x >== 0, x <== 10];
        pub let use_vars(x: Int) -> LinExpr = sum v in $[Vars](x) { v };
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Reify forward reference to constraint list should work: {:?}",
        errors
    );
}

#[test]
fn multiple_reify_forward_references() {
    let input = r#"
        reify c1 as $V1;
        reify c2 as $V2;
        pub let c1() -> Constraint = $V2() === 0;
        pub let c2() -> Constraint = $V1() === 1;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Multiple reify forward references should work: {:?}",
        errors
    );
}

// =============================================================================
// MIXED FORWARD REFERENCES
// =============================================================================

#[test]
fn mixed_function_and_type_forward_refs() {
    let input = r#"
        pub let make_point() -> Point = (0, 0) into Point;
        type Point = (Int, Int);
        pub let get_x(p: Point) -> Int = p.0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Mixed function and type forward refs should work: {:?}",
        errors
    );
}

#[test]
fn complex_forward_reference_scenario() {
    let input = r#"
        pub let create_tree() -> Tree = (0, []) into Tree;
        type Tree = (Int, [Tree]);
        pub let tree_value(t: Tree) -> Int = helper(t);
        let helper(t: Tree) -> Int = t.0;
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Complex forward reference scenario should work: {:?}",
        errors
    );
}

#[test]
fn recursive_function_with_recursive_type() {
    let input = r#"
        type Tree = (Int, [Tree]);
        pub let sum_tree(t: Tree) -> Int =
            t.0 + (sum c in t.1 { sum_tree(c) });
    "#;
    let (_, errors, _) = analyze(input, HashMap::new(), HashMap::new());
    assert!(
        errors.is_empty(),
        "Recursive function with recursive type should work: {:?}",
        errors
    );
}
