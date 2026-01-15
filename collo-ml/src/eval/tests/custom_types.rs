use super::*;

// =============================================================================
// CUSTOM TYPE BASIC OPERATIONS
// =============================================================================

#[test]
fn custom_type_wrap_and_unwrap() {
    let input = r#"
        type MyInt = Int;
        pub let wrap(x: Int) -> MyInt = MyInt(x);
        pub let unwrap(x: MyInt) -> Int = Int(x);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    // Test wrapping
    let wrapped = checked_ast
        .quick_eval_fn("main", "wrap", vec![ExprValue::Int(42)])
        .expect("Should evaluate");

    assert_eq!(
        wrapped,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(42),
        }))
    );

    // Test unwrapping
    let unwrapped = checked_ast
        .quick_eval_fn("main", "unwrap", vec![wrapped])
        .expect("Should evaluate");

    assert_eq!(unwrapped, ExprValue::Int(42));
}

#[test]
fn custom_type_roundtrip() {
    let input = r#"
        type MyInt = Int;
        pub let roundtrip(x: Int) -> Int = Int(MyInt(x));
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "roundtrip", vec![ExprValue::Int(123)])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(123));
}

#[test]
fn custom_type_with_tuple() {
    let input = r#"
        type Point = (Int, Int);
        pub let make_point(x: Int, y: Int) -> Point = Point(x, y);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn(
            "main",
            "make_point",
            vec![ExprValue::Int(3), ExprValue::Int(4)],
        )
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Point".to_string(),
            variant: None,
            content: ExprValue::Tuple(vec![ExprValue::Int(3), ExprValue::Int(4)]),
        }))
    );
}

#[test]
fn custom_type_with_list() {
    let input = r#"
        type IntList = [Int];
        pub let make_list() -> IntList = IntList([1, 2, 3]);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "make_list", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "IntList".to_string(),
            variant: None,
            content: ExprValue::List(vec![
                ExprValue::Int(1),
                ExprValue::Int(2),
                ExprValue::Int(3)
            ]),
        }))
    );
}

// =============================================================================
// FIELD ACCESS THROUGH CUSTOM TYPES
// =============================================================================

#[test]
fn custom_type_tuple_field_access() {
    let input = r#"
        type Point = (Int, Int);
        pub let get_x(p: Point) -> Int = p.0;
        pub let get_y(p: Point) -> Int = p.1;
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let point = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "Point".to_string(),
        variant: None,
        content: ExprValue::Tuple(vec![ExprValue::Int(10), ExprValue::Int(20)]),
    }));

    let x = checked_ast
        .quick_eval_fn("main", "get_x", vec![point.clone()])
        .expect("Should evaluate");
    assert_eq!(x, ExprValue::Int(10));

    let y = checked_ast
        .quick_eval_fn("main", "get_y", vec![point])
        .expect("Should evaluate");
    assert_eq!(y, ExprValue::Int(20));
}

#[test]
fn custom_type_nested_tuple_field_access() {
    let input = r#"
        type Point = (Int, Int);
        type NamedPoint = (String, Point);
        pub let get_x(np: NamedPoint) -> Int = np.1.0;
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let named_point = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "NamedPoint".to_string(),
        variant: None,
        content: ExprValue::Tuple(vec![
            ExprValue::String("origin".to_string()),
            ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "Point".to_string(),
                variant: None,
                content: ExprValue::Tuple(vec![ExprValue::Int(0), ExprValue::Int(0)]),
            })),
        ]),
    }));

    let x = checked_ast
        .quick_eval_fn("main", "get_x", vec![named_point])
        .expect("Should evaluate");
    assert_eq!(x, ExprValue::Int(0));
}

// =============================================================================
// CUSTOM TYPES IN COLLECTIONS
// =============================================================================

#[test]
fn custom_type_in_list() {
    let input = r#"
        type MyInt = Int;
        pub let make_list() -> [MyInt] = [MyInt(1), MyInt(2), MyInt(3)];
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "make_list", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: ExprValue::Int(1),
            })),
            ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: ExprValue::Int(2),
            })),
            ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: ExprValue::Int(3),
            })),
        ])
    );
}

#[test]
fn sum_over_custom_type_list() {
    let input = r#"
        type MyInt = Int;
        pub let total(xs: [MyInt]) -> Int = sum x in xs { Int(x) };
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let list = ExprValue::List(vec![
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(1),
        })),
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(2),
        })),
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(3),
        })),
    ]);

    let result = checked_ast
        .quick_eval_fn("main", "total", vec![list])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(6));
}

// =============================================================================
// CUSTOM TYPES IN CONTROL FLOW
// =============================================================================

#[test]
fn custom_type_in_if_expression() {
    let input = r#"
        type MyInt = Int;
        pub let f(b: Bool) -> MyInt = if b { MyInt(1) } else { MyInt(0) };
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result_true = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(true)])
        .expect("Should evaluate");
    assert_eq!(
        result_true,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(1),
        }))
    );

    let result_false = checked_ast
        .quick_eval_fn("main", "f", vec![ExprValue::Bool(false)])
        .expect("Should evaluate");
    assert_eq!(
        result_false,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(0),
        }))
    );
}

#[test]
fn custom_type_in_let_expression() {
    let input = r#"
        type MyInt = Int;
        pub let f() -> Int = let x = MyInt(42) { Int(x) };
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "f", vec![])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(42));
}

// =============================================================================
// CUSTOM TYPE STRING CONVERSION
// =============================================================================

#[test]
fn custom_type_to_string() {
    let input = r#"
        type MyInt = Int;
        pub let to_str(x: MyInt) -> String = String(x);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let value = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "MyInt".to_string(),
        variant: None,
        content: ExprValue::Int(42),
    }));

    let result = checked_ast
        .quick_eval_fn("main", "to_str", vec![value])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("MyInt(42)".to_string()));
}

#[test]
fn custom_type_tuple_to_string() {
    let input = r#"
        type Point = (Int, Int);
        pub let to_str(p: Point) -> String = String(p);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let value = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "Point".to_string(),
        variant: None,
        content: ExprValue::Tuple(vec![ExprValue::Int(3), ExprValue::Int(4)]),
    }));

    let result = checked_ast
        .quick_eval_fn("main", "to_str", vec![value])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::String("Point((3, 4))".to_string()));
}

// =============================================================================
// MULTIPLE CUSTOM TYPES
// =============================================================================

#[test]
fn multiple_custom_types() {
    let input = r#"
        type TypeA = Int;
        type TypeB = Int;
        pub let make_a(x: Int) -> TypeA = TypeA(x);
        pub let make_b(x: Int) -> TypeB = TypeB(x);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let a = checked_ast
        .quick_eval_fn("main", "make_a", vec![ExprValue::Int(1)])
        .expect("Should evaluate");
    let b = checked_ast
        .quick_eval_fn("main", "make_b", vec![ExprValue::Int(1)])
        .expect("Should evaluate");

    // Even though both are Int underneath, they should be different custom types
    assert_eq!(
        a,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "TypeA".to_string(),
            variant: None,
            content: ExprValue::Int(1),
        }))
    );
    assert_eq!(
        b,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "TypeB".to_string(),
            variant: None,
            content: ExprValue::Int(1),
        }))
    );
    assert_ne!(a, b);
}

#[test]
fn custom_type_referencing_another() {
    let input = r#"
        type Inner = Int;
        type Outer = [Inner];
        pub let make() -> Outer = Outer([Inner(1), Inner(2)]);
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let result = checked_ast
        .quick_eval_fn("main", "make", vec![])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "Outer".to_string(),
            variant: None,
            content: ExprValue::List(vec![
                ExprValue::Custom(Box::new(CustomValue {
                    module: "main".to_string(),
                    type_name: "Inner".to_string(),
                    variant: None,
                    content: ExprValue::Int(1),
                })),
                ExprValue::Custom(Box::new(CustomValue {
                    module: "main".to_string(),
                    type_name: "Inner".to_string(),
                    variant: None,
                    content: ExprValue::Int(2),
                })),
            ]),
        }))
    );
}

// =============================================================================
// CUSTOM TYPES WITH FOLDS
// =============================================================================

#[test]
fn custom_type_in_fold() {
    let input = r#"
        type MyInt = Int;
        pub let sum_custom(xs: [MyInt]) -> Int = fold x in xs with acc = 0 { acc + (Int(x)) };
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let list = ExprValue::List(vec![
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(1),
        })),
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(2),
        })),
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(3),
        })),
    ]);

    let result = checked_ast
        .quick_eval_fn("main", "sum_custom", vec![list])
        .expect("Should evaluate");

    assert_eq!(result, ExprValue::Int(6));
}

// =============================================================================
// CUSTOM TYPES WITH LIST COMPREHENSIONS
// =============================================================================

#[test]
fn custom_type_in_list_comprehension() {
    let input = r#"
        type MyInt = Int;
        pub let double_all(xs: [MyInt]) -> [MyInt] = [MyInt(Int(x) * 2) for x in xs];
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    let list = ExprValue::List(vec![
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(1),
        })),
        ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "MyInt".to_string(),
            variant: None,
            content: ExprValue::Int(2),
        })),
    ]);

    let result = checked_ast
        .quick_eval_fn("main", "double_all", vec![list])
        .expect("Should evaluate");

    assert_eq!(
        result,
        ExprValue::List(vec![
            ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: ExprValue::Int(2),
            })),
            ExprValue::Custom(Box::new(CustomValue {
                module: "main".to_string(),
                type_name: "MyInt".to_string(),
                variant: None,
                content: ExprValue::Int(4),
            })),
        ])
    );
}

// =============================================================================
// CUSTOM TYPES WRAPPING UNION TYPES
// =============================================================================

#[test]
fn custom_type_wrapping_union_tuple_index() {
    // Custom type wraps union of tuples, tuple index access should work
    let input = r#"
        type MyType = (Int, Bool) | (String, Bool);
        pub let get_second(x: MyType) -> Bool = x.1;
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    // Test with first variant (Int, Bool)
    let value1 = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "MyType".to_string(),
        variant: None,
        content: ExprValue::Tuple(vec![ExprValue::Int(42), ExprValue::Bool(true)]),
    }));
    let result1 = checked_ast
        .quick_eval_fn("main", "get_second", vec![value1])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Bool(true));

    // Test with second variant (String, Bool)
    let value2 = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "MyType".to_string(),
        variant: None,
        content: ExprValue::Tuple(vec![
            ExprValue::String("hello".to_string()),
            ExprValue::Bool(false),
        ]),
    }));
    let result2 = checked_ast
        .quick_eval_fn("main", "get_second", vec![value2])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Bool(false));
}

#[test]
fn custom_type_wrapping_union_tuple_index_returns_union() {
    // Custom type wraps union of tuples with different first element types
    let input = r#"
        type MyType = (Int, Bool) | (String, Bool);
        pub let get_first(x: MyType) -> Int | String = x.0;
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    // Test with first variant (Int, Bool)
    let value1 = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "MyType".to_string(),
        variant: None,
        content: ExprValue::Tuple(vec![ExprValue::Int(42), ExprValue::Bool(true)]),
    }));
    let result1 = checked_ast
        .quick_eval_fn("main", "get_first", vec![value1])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(42));

    // Test with second variant (String, Bool)
    let value2 = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "MyType".to_string(),
        variant: None,
        content: ExprValue::Tuple(vec![
            ExprValue::String("hello".to_string()),
            ExprValue::Bool(false),
        ]),
    }));
    let result2 = checked_ast
        .quick_eval_fn("main", "get_first", vec![value2])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::String("hello".to_string()));
}

#[test]
fn custom_type_wrapping_nested_custom_type_union() {
    // type A wraps tuple, type B is union containing A
    let input = r#"
        type A = (Int, Int);
        type B = A | (String, Int);
        pub let get_second(x: B) -> Int = x.1;
    "#;
    let checked_ast = CheckedAST::new(&BTreeMap::from([("main", input)]), HashMap::new())
        .expect("Should compile");

    // Test with A variant (wrapped in B)
    let value1 = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "B".to_string(),
        variant: None,
        content: ExprValue::Custom(Box::new(CustomValue {
            module: "main".to_string(),
            type_name: "A".to_string(),
            variant: None,
            content: ExprValue::Tuple(vec![ExprValue::Int(1), ExprValue::Int(2)]),
        })),
    }));
    let result1 = checked_ast
        .quick_eval_fn("main", "get_second", vec![value1])
        .expect("Should evaluate");
    assert_eq!(result1, ExprValue::Int(2));

    // Test with (String, Int) variant
    let value2 = ExprValue::Custom(Box::new(CustomValue {
        module: "main".to_string(),
        type_name: "B".to_string(),
        variant: None,
        content: ExprValue::Tuple(vec![
            ExprValue::String("test".to_string()),
            ExprValue::Int(99),
        ]),
    }));
    let result2 = checked_ast
        .quick_eval_fn("main", "get_second", vec![value2])
        .expect("Should evaluate");
    assert_eq!(result2, ExprValue::Int(99));
}
