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

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(42));
}

#[test]
fn function_forward_reference_with_params() {
    let input = r#"
        pub let f(x: Int) -> Int = g(x, 10);
        let g(a: Int, b: Int) -> Int = a + b;
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    // g(5, 10) = 5 + 10 = 15
    assert_eq!(result, ExprValue::Int(15));
}

#[test]
fn function_forward_reference_chain() {
    let input = r#"
        pub let a() -> Int = b();
        let b() -> Int = c();
        let c() -> Int = 42;
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("a", vec![])
        .expect("Should evaluate");
    // a() -> b() -> c() -> 42
    assert_eq!(result, ExprValue::Int(42));
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

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // factorial(0) = 1
    let result0 = checked_ast
        .quick_eval_fn("factorial", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(result0, ExprValue::Int(1));

    // factorial(1) = 1
    let result1 = checked_ast
        .quick_eval_fn("factorial", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(1));

    // factorial(5) = 120
    let result5 = checked_ast
        .quick_eval_fn("factorial", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result5, ExprValue::Int(120));

    // factorial(10) = 3628800
    let result10 = checked_ast
        .quick_eval_fn("factorial", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result10, ExprValue::Int(3628800));
}

#[test]
fn direct_recursion_countdown() {
    // Note: Recursion depth limits due to stack size:
    // - Debug mode: ~19
    // - Release mode: ~250-350
    // Each ColloML recursive call uses significant stack space.
    let input = r#"
        pub let countdown(n: Int) -> Int =
            if n == 0 { 0 } else { countdown(n - 1) };
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // Keep depth at 10 to work safely in both debug and release
    let result = checked_ast
        .quick_eval_fn("countdown", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result, ExprValue::Int(0));
}

#[test]
fn direct_recursion_sum_to_n() {
    let input = r#"
        pub let sum_to(n: Int) -> Int =
            if n == 0 { 0 } else { n + sum_to(n - 1) };
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // sum_to(0) = 0
    let result0 = checked_ast
        .quick_eval_fn("sum_to", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(result0, ExprValue::Int(0));

    // sum_to(5) = 5 + 4 + 3 + 2 + 1 = 15
    let result5 = checked_ast
        .quick_eval_fn("sum_to", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(result5, ExprValue::Int(15));

    // sum_to(10) = 55
    let result10 = checked_ast
        .quick_eval_fn("sum_to", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result10, ExprValue::Int(55));
}

#[test]
fn direct_recursion_fibonacci() {
    let input = r#"
        pub let fib(n: Int) -> Int =
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) };
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // fib(0) = 0
    let result0 = checked_ast
        .quick_eval_fn("fib", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(result0, ExprValue::Int(0));

    // fib(1) = 1
    let result1 = checked_ast
        .quick_eval_fn("fib", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(1));

    // fib(10) = 55
    let result10 = checked_ast
        .quick_eval_fn("fib", vec![ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result10, ExprValue::Int(55));
}

#[test]
fn direct_recursion_constraint_function() {
    let input = r#"
        pub let recursive_constraint(n: Int) -> Constraint =
            if n == 0 { 0 === 0 } else { n >== 0 and recursive_constraint(n - 1) };
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // recursive_constraint(3) should produce constraints: 3 >= 0 and 2 >= 0 and 1 >= 0 and 0 == 0
    let result = checked_ast
        .quick_eval_fn("recursive_constraint", vec![ExprValue::Int(3)])
        .expect("Should evaluate");

    match result {
        ExprValue::Constraint(constraints) => {
            // Should have 4 constraints: 0===0, 1>=0, 2>=0, 3>=0
            assert_eq!(constraints.len(), 4);
        }
        _ => panic!("Expected Constraint"),
    }
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

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // Test is_even
    let even0 = checked_ast
        .quick_eval_fn("is_even", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(even0, ExprValue::Bool(true));

    let even1 = checked_ast
        .quick_eval_fn("is_even", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(even1, ExprValue::Bool(false));

    let even4 = checked_ast
        .quick_eval_fn("is_even", vec![ExprValue::Int(4)])
        .expect("Should evaluate");
    assert_eq!(even4, ExprValue::Bool(true));

    let even7 = checked_ast
        .quick_eval_fn("is_even", vec![ExprValue::Int(7)])
        .expect("Should evaluate");
    assert_eq!(even7, ExprValue::Bool(false));

    // Test is_odd
    let odd0 = checked_ast
        .quick_eval_fn("is_odd", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(odd0, ExprValue::Bool(false));

    let odd1 = checked_ast
        .quick_eval_fn("is_odd", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(odd1, ExprValue::Bool(true));

    let odd5 = checked_ast
        .quick_eval_fn("is_odd", vec![ExprValue::Int(5)])
        .expect("Should evaluate");
    assert_eq!(odd5, ExprValue::Bool(true));

    let odd6 = checked_ast
        .quick_eval_fn("is_odd", vec![ExprValue::Int(6)])
        .expect("Should evaluate");
    assert_eq!(odd6, ExprValue::Bool(false));
}

#[test]
fn mutual_recursion_three_functions() {
    let input = r#"
        let a(n: Int) -> Int = if n == 0 { 0 } else { b(n - 1) };
        let b(n: Int) -> Int = if n == 0 { 1 } else { c(n - 1) };
        let c(n: Int) -> Int = if n == 0 { 2 } else { a(n - 1) };
        pub let start(n: Int) -> Int = a(n);
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // start(0) = a(0) = 0
    let result0 = checked_ast
        .quick_eval_fn("start", vec![ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(result0, ExprValue::Int(0));

    // start(1) = a(1) = b(0) = 1
    let result1 = checked_ast
        .quick_eval_fn("start", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(1));

    // start(2) = a(2) = b(1) = c(0) = 2
    let result2 = checked_ast
        .quick_eval_fn("start", vec![ExprValue::Int(2)])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(2));

    // start(3) = a(3) = b(2) = c(1) = a(0) = 0
    let result3 = checked_ast
        .quick_eval_fn("start", vec![ExprValue::Int(3)])
        .expect("Should evaluate");
    assert_eq!(result3, ExprValue::Int(0));
}

// =============================================================================
// TYPE FORWARD REFERENCES
// =============================================================================

#[test]
fn type_forward_reference_simple() {
    let input = r#"
        type A = [B];
        type B = Int;
        pub let f() -> A = A([B(5)]);
        pub let unwrap(a: A) -> [Int] = [Int(x) for x in [B](a)];
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    // The result is a custom type wrapping [5]
    match result {
        ExprValue::Custom { .. } => {
            // Custom type created successfully
        }
        _ => panic!("Expected Custom type"),
    }
}

#[test]
fn type_forward_reference_in_function() {
    let input = r#"
        pub let f() -> B = B(5);
        type B = Int;
        pub let unwrap(b: B) -> Int = Int(b);
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let f_result = checked_ast
        .quick_eval_fn("f", vec![])
        .expect("Should evaluate");

    // Unwrap to verify the value
    let unwrap_result = checked_ast
        .quick_eval_fn("unwrap", vec![f_result])
        .expect("Should evaluate");
    assert_eq!(unwrap_result, ExprValue::Int(5));
}

// =============================================================================
// GUARDED RECURSIVE TYPES
// =============================================================================

#[test]
fn guarded_recursion_tree_structure() {
    let input = r#"
        type Tree = (Int, [Tree]);
        pub let value(t: Tree) -> Int = t.0;
        pub let children(t: Tree) -> [Tree] = t.1;
        pub let leaf(v: Int) -> Tree = Tree(v, []);
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // Create a leaf node with value 42
    let leaf = checked_ast
        .quick_eval_fn("leaf", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    // Get value from leaf
    let value = checked_ast
        .quick_eval_fn("value", vec![leaf.clone()])
        .expect("Should evaluate");
    assert_eq!(value, ExprValue::Int(42));

    // Get children from leaf (should be empty)
    let children = checked_ast
        .quick_eval_fn("children", vec![leaf])
        .expect("Should evaluate");
    match children {
        ExprValue::List(list) => assert!(list.is_empty()),
        _ => panic!("Expected List"),
    }
}

#[test]
fn recursive_function_with_recursive_type() {
    let input = r#"
        type Tree = (Int, [Tree]);
        pub let sum_tree(t: Tree) -> Int =
            t.0 + (sum c in t.1 { sum_tree(c) });
        pub let leaf(v: Int) -> Tree = Tree(v, []);
        pub let node(v: Int, children: [Tree]) -> Tree = Tree(v, children);
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // Create a leaf with value 5
    let leaf5 = checked_ast
        .quick_eval_fn("leaf", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    // sum_tree of a leaf should be just its value
    let sum_leaf = checked_ast
        .quick_eval_fn("sum_tree", vec![leaf5.clone()])
        .expect("Should evaluate");
    assert_eq!(sum_leaf, ExprValue::Int(5));

    // Create another leaf
    let leaf3 = checked_ast
        .quick_eval_fn("leaf", vec![ExprValue::Int(3)])
        .expect("Should evaluate");

    // Create a parent node with value 10 and two children
    let children = ExprValue::List(vec![leaf5, leaf3]);
    let parent = checked_ast
        .quick_eval_fn("node", vec![ExprValue::Int(10), children])
        .expect("Should evaluate");

    // sum_tree of parent should be 10 + 5 + 3 = 18
    let sum_parent = checked_ast
        .quick_eval_fn("sum_tree", vec![parent])
        .expect("Should evaluate");
    assert_eq!(sum_parent, ExprValue::Int(18));
}

// =============================================================================
// REIFICATION FORWARD REFERENCES
// =============================================================================

// Note: Tests where constraint functions reference their own reified variables
// cause infinite loops when evaluated (e.g., constraint_fn uses $Var which is
// reified from constraint_fn itself). These are valid for semantic analysis
// but would loop forever at runtime.
//
// Tests removed:
// - reify_forward_reference: constraint_fn uses $Var() === 0
// - reify_forward_reference_with_params: my_constraint uses $MyVar(x)
//
// The reify_forward_reference_list test is safe because use_vars doesn't
// create a self-referential loop.

#[test]
fn reify_forward_reference_list() {
    let input = r#"
        reify constraints as $[Vars];
        pub let constraints(x: Int) -> [Constraint] = [x >== 0, x <== 10];
        pub let use_vars(x: Int) -> LinExpr = sum v in $[Vars](x) { v };
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("use_vars", vec![ExprValue::Int(5)])
        .expect("Should evaluate");

    match result {
        ExprValue::LinExpr(_) => {
            // LinExpr created successfully from summing the variable list
        }
        _ => panic!("Expected LinExpr"),
    }
}

// =============================================================================
// MIXED FORWARD REFERENCES
// =============================================================================

#[test]
fn mixed_function_and_type_forward_refs() {
    let input = r#"
        pub let make_point() -> Point = Point(0, 0);
        type Point = (Int, Int);
        pub let get_x(p: Point) -> Int = p.0;
        pub let get_y(p: Point) -> Int = p.1;
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let point = checked_ast
        .quick_eval_fn("make_point", vec![])
        .expect("Should evaluate");

    let x = checked_ast
        .quick_eval_fn("get_x", vec![point.clone()])
        .expect("Should evaluate");
    assert_eq!(x, ExprValue::Int(0));

    let y = checked_ast
        .quick_eval_fn("get_y", vec![point])
        .expect("Should evaluate");
    assert_eq!(y, ExprValue::Int(0));
}

#[test]
fn complex_forward_reference_scenario() {
    let input = r#"
        pub let create_tree() -> Tree = Tree(0, []);
        type Tree = (Int, [Tree]);
        pub let tree_value(t: Tree) -> Int = helper(t);
        let helper(t: Tree) -> Int = t.0;
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let tree = checked_ast
        .quick_eval_fn("create_tree", vec![])
        .expect("Should evaluate");

    let value = checked_ast
        .quick_eval_fn("tree_value", vec![tree])
        .expect("Should evaluate");
    assert_eq!(value, ExprValue::Int(0));
}

// =============================================================================
// ADDITIONAL RECURSION TESTS
// =============================================================================

#[test]
fn recursion_with_list_processing() {
    let input = r#"
        pub let list_length(xs: [Int]) -> Int =
            if |xs| == 0 { 0 } else { 1 + list_length([x for x in xs where x != xs[0]!]) };
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // Note: This is a quirky way to compute length that removes first element each time
    // For unique lists it works correctly
    let empty = ExprValue::List(vec![]);
    let result0 = checked_ast
        .quick_eval_fn("list_length", vec![empty])
        .expect("Should evaluate");
    assert_eq!(result0, ExprValue::Int(0));
}

#[test]
fn recursion_with_accumulator_pattern() {
    let input = r#"
        let sum_helper(xs: [Int], acc: Int) -> Int =
            if |xs| == 0 { acc } else { sum_helper([xs[i]! for i in [1..|xs|]], acc + xs[0]!) };
        pub let list_sum(xs: [Int]) -> Int = sum_helper(xs, 0);
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    let empty = ExprValue::List(vec![]);
    let result_empty = checked_ast
        .quick_eval_fn("list_sum", vec![empty])
        .expect("Should evaluate");
    assert_eq!(result_empty, ExprValue::Int(0));

    let list123 = ExprValue::List(vec![
        ExprValue::Int(1),
        ExprValue::Int(2),
        ExprValue::Int(3),
    ]);
    let result123 = checked_ast
        .quick_eval_fn("list_sum", vec![list123])
        .expect("Should evaluate");
    assert_eq!(result123, ExprValue::Int(6));
}

#[test]
fn recursion_gcd() {
    let input = r#"
        pub let gcd(a: Int, b: Int) -> Int =
            if b == 0 { a } else { gcd(b, a % b) };
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // gcd(48, 18) = 6
    let result1 = checked_ast
        .quick_eval_fn("gcd", vec![ExprValue::Int(48), ExprValue::Int(18)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(6));

    // gcd(100, 25) = 25
    let result2 = checked_ast
        .quick_eval_fn("gcd", vec![ExprValue::Int(100), ExprValue::Int(25)])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(25));

    // gcd(17, 13) = 1 (coprime)
    let result3 = checked_ast
        .quick_eval_fn("gcd", vec![ExprValue::Int(17), ExprValue::Int(13)])
        .expect("Should evaluate");
    assert_eq!(result3, ExprValue::Int(1));
}

#[test]
fn recursion_power() {
    let input = r#"
        pub let power(base: Int, exp: Int) -> Int =
            if exp == 0 { 1 } else { base * power(base, exp - 1) };
    "#;

    let checked_ast = CheckedAST::new(input, HashMap::new()).expect("Should compile");

    // 2^0 = 1
    let result1 = checked_ast
        .quick_eval_fn("power", vec![ExprValue::Int(2), ExprValue::Int(0)])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(1));

    // 2^10 = 1024
    let result2 = checked_ast
        .quick_eval_fn("power", vec![ExprValue::Int(2), ExprValue::Int(10)])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(1024));

    // 3^4 = 81
    let result3 = checked_ast
        .quick_eval_fn("power", vec![ExprValue::Int(3), ExprValue::Int(4)])
        .expect("Should evaluate");
    assert_eq!(result3, ExprValue::Int(81));
}
