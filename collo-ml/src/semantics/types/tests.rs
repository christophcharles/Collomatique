use super::*;

// ========== SimpleType Helper Functions ==========

#[test]
fn simple_type_is_primitive_type() {
    assert!(SimpleType::Int.is_primitive_type());
    assert!(SimpleType::Bool.is_primitive_type());
    assert!(SimpleType::None.is_primitive_type());
    assert!(SimpleType::LinExpr.is_primitive_type());
    assert!(SimpleType::Constraint.is_primitive_type());

    assert!(!SimpleType::EmptyList.is_primitive_type());
    assert!(!SimpleType::List(ExprType::simple(SimpleType::Int)).is_primitive_type());
    assert!(!SimpleType::Object("Student".to_string()).is_primitive_type());
}

#[test]
fn simple_type_is_none() {
    assert!(SimpleType::None.is_none());

    assert!(!SimpleType::Int.is_none());
    assert!(!SimpleType::Bool.is_none());
}

#[test]
fn simple_type_is_list() {
    assert!(SimpleType::EmptyList.is_list());
    assert!(SimpleType::List(ExprType::simple(SimpleType::Int)).is_list());
    assert!(SimpleType::List(ExprType::simple(SimpleType::Bool)).is_list());

    assert!(!SimpleType::Int.is_list());
    assert!(!SimpleType::Bool.is_list());
    assert!(!SimpleType::Object("Student".to_string()).is_list());
}

#[test]
fn simple_type_is_empty_list() {
    assert!(SimpleType::EmptyList.is_empty_list());

    assert!(!SimpleType::List(ExprType::simple(SimpleType::Int)).is_empty_list());
    assert!(!SimpleType::Int.is_empty_list());
}

#[test]
fn simple_type_is_specific_primitives() {
    assert!(SimpleType::Int.is_int());
    assert!(!SimpleType::Bool.is_int());

    assert!(SimpleType::Bool.is_bool());
    assert!(!SimpleType::Int.is_bool());

    assert!(SimpleType::LinExpr.is_lin_expr());
    assert!(!SimpleType::Int.is_lin_expr());

    assert!(SimpleType::Constraint.is_constraint());
    assert!(!SimpleType::Int.is_constraint());
}

#[test]
fn simple_type_is_list_of_constraints() {
    assert!(SimpleType::List(ExprType::simple(SimpleType::Constraint)).is_list_of_constraints());

    assert!(!SimpleType::List(ExprType::simple(SimpleType::Int)).is_list_of_constraints());
    assert!(!SimpleType::Constraint.is_list_of_constraints());
    assert!(!SimpleType::EmptyList.is_list_of_constraints());
}

#[test]
fn simple_type_get_inner_list_type() {
    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    assert_eq!(
        list_int.get_inner_list_type(),
        Some(&ExprType::simple(SimpleType::Int))
    );

    assert_eq!(SimpleType::Int.get_inner_list_type(), None);
    assert_eq!(SimpleType::EmptyList.get_inner_list_type(), None);
}

#[test]
fn simple_type_to_inner_list_type() {
    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    assert_eq!(
        list_int.to_inner_list_type(),
        Some(ExprType::simple(SimpleType::Int))
    );

    assert_eq!(SimpleType::Int.to_inner_list_type(), None);
    assert_eq!(SimpleType::EmptyList.to_inner_list_type(), None);
}

#[test]
fn simple_type_is_object() {
    assert!(SimpleType::Object("Student".to_string()).is_object());

    assert!(!SimpleType::Int.is_object());
    assert!(!SimpleType::List(ExprType::simple(SimpleType::Int)).is_object());
}

#[test]
fn simple_type_get_inner_object_type() {
    let obj = SimpleType::Object("Student".to_string());
    assert_eq!(obj.get_inner_object_type(), Some(&"Student".to_string()));

    assert_eq!(SimpleType::Int.get_inner_object_type(), None);
}

#[test]
fn simple_type_to_inner_object_type() {
    let obj = SimpleType::Object("Student".to_string());
    assert_eq!(obj.to_inner_object_type(), Some("Student".to_string()));

    assert_eq!(SimpleType::Int.to_inner_object_type(), None);
}

#[test]
fn simple_type_is_arithmetic() {
    assert!(SimpleType::Int.is_arithmetic());
    assert!(SimpleType::LinExpr.is_arithmetic());

    assert!(!SimpleType::Bool.is_arithmetic());
    assert!(!SimpleType::Constraint.is_arithmetic());
    assert!(!SimpleType::None.is_arithmetic());
}

#[test]
fn simple_type_is_concrete() {
    // Primitives are concrete
    assert!(SimpleType::Int.is_concrete());
    assert!(SimpleType::Bool.is_concrete());
    assert!(SimpleType::None.is_concrete());

    // EmptyList is concrete
    assert!(SimpleType::EmptyList.is_concrete());

    // List of concrete type is concrete
    assert!(SimpleType::List(ExprType::simple(SimpleType::Int)).is_concrete());

    // List of union type is not concrete
    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert!(!SimpleType::List(union).is_concrete());
}

#[test]
fn simple_type_into_concrete() {
    let int = SimpleType::Int;
    assert!(int.clone().into_concrete().is_some());

    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    assert!(list_int.clone().into_concrete().is_some());

    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    let list_union = SimpleType::List(union);
    assert!(list_union.into_concrete().is_none());
}

// ========== SimpleType::is_subtype_of ==========

#[test]
fn simple_type_is_subtype_of_self() {
    assert!(SimpleType::Int.is_subtype_of(&SimpleType::Int));
    assert!(SimpleType::Bool.is_subtype_of(&SimpleType::Bool));
    assert!(SimpleType::None.is_subtype_of(&SimpleType::None));
}

#[test]
fn emptylist_is_subtype_of_any_list() {
    assert!(
        SimpleType::EmptyList.is_subtype_of(&SimpleType::List(ExprType::simple(SimpleType::Int)))
    );
    assert!(
        SimpleType::EmptyList.is_subtype_of(&SimpleType::List(ExprType::simple(SimpleType::Bool)))
    );
    assert!(SimpleType::EmptyList.is_subtype_of(&SimpleType::EmptyList));
}

#[test]
fn list_is_not_subtype_of_emptylist() {
    assert!(
        !SimpleType::List(ExprType::simple(SimpleType::Int)).is_subtype_of(&SimpleType::EmptyList)
    );
}

#[test]
fn list_subtyping_is_recursive() {
    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    let list_int_linexpr =
        SimpleType::List(ExprType::sum([SimpleType::Int, SimpleType::LinExpr]).unwrap());

    // [Int] is subtype of [Int | LinExpr]
    assert!(list_int.is_subtype_of(&list_int_linexpr));

    // [Int | LinExpr] is not subtype of [Int]
    assert!(!list_int_linexpr.is_subtype_of(&list_int));
}

#[test]
fn different_primitives_not_subtypes() {
    assert!(!SimpleType::Int.is_subtype_of(&SimpleType::Bool));
    assert!(!SimpleType::Bool.is_subtype_of(&SimpleType::Int));
    assert!(!SimpleType::Int.is_subtype_of(&SimpleType::LinExpr));
}

// ========== SimpleType::can_convert_to ==========

#[test]
fn can_convert_to_same_type() {
    let int_concrete = SimpleType::Int.into_concrete().unwrap();
    assert!(SimpleType::Int.can_convert_to(&int_concrete));
}

#[test]
fn int_can_convert_to_linexpr() {
    let linexpr_concrete = SimpleType::LinExpr.into_concrete().unwrap();
    assert!(SimpleType::Int.can_convert_to(&linexpr_concrete));
}

#[test]
fn linexpr_cannot_convert_to_int() {
    let int_concrete = SimpleType::Int.into_concrete().unwrap();
    assert!(!SimpleType::LinExpr.can_convert_to(&int_concrete));
}

#[test]
fn emptylist_can_convert_to_any_list() {
    let list_int_concrete = SimpleType::List(ExprType::simple(SimpleType::Int))
        .into_concrete()
        .unwrap();

    assert!(SimpleType::EmptyList.can_convert_to(&list_int_concrete));
}

#[test]
fn list_conversion_is_recursive() {
    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    let list_linexpr_concrete = SimpleType::List(ExprType::simple(SimpleType::LinExpr))
        .into_concrete()
        .unwrap();

    // [Int] can convert to [LinExpr]
    assert!(list_int.can_convert_to(&list_linexpr_concrete));
}

#[test]
fn cannot_convert_unrelated_types() {
    let bool_concrete = SimpleType::Bool.into_concrete().unwrap();
    assert!(!SimpleType::Int.can_convert_to(&bool_concrete));
    assert!(!SimpleType::LinExpr.can_convert_to(&bool_concrete));
}

// ========== SimpleType::overlaps_with ==========

#[test]
fn same_primitives_overlap() {
    assert!(SimpleType::Int.overlaps_with(&SimpleType::Int));
    assert!(SimpleType::Bool.overlaps_with(&SimpleType::Bool));
    assert!(SimpleType::None.overlaps_with(&SimpleType::None));
}

#[test]
fn different_primitives_dont_overlap() {
    assert!(!SimpleType::Int.overlaps_with(&SimpleType::Bool));
    assert!(!SimpleType::Int.overlaps_with(&SimpleType::None));
    assert!(!SimpleType::Bool.overlaps_with(&SimpleType::LinExpr));
}

#[test]
fn same_objects_overlap() {
    let student1 = SimpleType::Object("Student".to_string());
    let student2 = SimpleType::Object("Student".to_string());
    assert!(student1.overlaps_with(&student2));
}

#[test]
fn different_objects_dont_overlap() {
    let student = SimpleType::Object("Student".to_string());
    let teacher = SimpleType::Object("Teacher".to_string());
    assert!(!student.overlaps_with(&teacher));
}

#[test]
fn all_lists_overlap() {
    assert!(SimpleType::EmptyList.overlaps_with(&SimpleType::EmptyList));

    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    let list_bool = SimpleType::List(ExprType::simple(SimpleType::Bool));

    assert!(SimpleType::EmptyList.overlaps_with(&list_int));
    assert!(list_int.overlaps_with(&SimpleType::EmptyList));
    assert!(list_int.overlaps_with(&list_bool));
}

#[test]
fn lists_dont_overlap_with_primitives() {
    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    assert!(!list_int.overlaps_with(&SimpleType::Int));
    assert!(!SimpleType::Int.overlaps_with(&list_int));
}

// ========== ExprType Construction ==========

#[test]
fn expr_type_simple() {
    let typ = ExprType::simple(SimpleType::Int);
    assert!(typ.is_simple());
    assert_eq!(typ.as_simple(), Some(&SimpleType::Int));
}

#[test]
fn expr_type_maybe() {
    let typ = ExprType::maybe(SimpleType::Int).unwrap();
    assert!(!typ.is_simple());
    assert!(typ.contains(&SimpleType::None));
    assert!(typ.contains(&SimpleType::Int));

    // Cannot create maybe None
    assert!(ExprType::maybe(SimpleType::None).is_none());
}

#[test]
fn expr_type_sum() {
    let typ = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert!(!typ.is_simple());
    assert!(typ.contains(&SimpleType::Int));
    assert!(typ.contains(&SimpleType::Bool));

    // Empty sum returns None
    let empty: Vec<SimpleType> = vec![];
    assert!(ExprType::sum(empty).is_none());
}

#[test]
fn expr_type_sum_removes_subtypes() {
    // Creating [Int] | [] should just be Int (EmptyList is removed)
    let typ = ExprType::sum([
        SimpleType::EmptyList,
        SimpleType::List(ExprType::simple(SimpleType::Int)),
    ])
    .unwrap();

    assert!(typ.is_simple());
    assert_eq!(
        typ.as_simple(),
        Some(&SimpleType::List(ExprType::simple(SimpleType::Int)))
    );
}

#[test]
fn expr_type_sum_removes_duplicate_subtypes() {
    // [Int] | [[Int]] | [] should become [Int] | [[Int]] (others are subtypes)
    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    let list_list_int = SimpleType::List(ExprType::simple(list_int.clone()));

    let typ = ExprType::sum([
        SimpleType::EmptyList,
        list_int.clone(),
        list_list_int.clone(),
    ])
    .unwrap();

    assert!(!typ.is_simple());
    assert_eq!(typ, ExprType::sum([list_int, list_list_int,]).unwrap());
}

// ========== ExprType Query Methods ==========

#[test]
fn expr_type_is_simple() {
    assert!(ExprType::simple(SimpleType::Int).is_simple());

    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert!(!union.is_simple());
}

#[test]
fn expr_type_as_simple() {
    let simple = ExprType::simple(SimpleType::Int);
    assert_eq!(simple.as_simple(), Some(&SimpleType::Int));

    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert_eq!(union.as_simple(), None);
}

#[test]
fn expr_type_to_simple() {
    let simple = ExprType::simple(SimpleType::Int);
    assert_eq!(simple.to_simple(), Some(SimpleType::Int));

    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert_eq!(union.to_simple(), None);
}

#[test]
fn expr_type_is_primitive_type() {
    assert!(ExprType::simple(SimpleType::Int).is_primitive_type());
    assert!(ExprType::simple(SimpleType::Bool).is_primitive_type());

    assert!(!ExprType::simple(SimpleType::EmptyList).is_primitive_type());

    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert!(!union.is_primitive_type());
}

#[test]
fn expr_type_is_concrete() {
    assert!(ExprType::simple(SimpleType::Int).is_concrete());
    assert!(ExprType::simple(SimpleType::EmptyList).is_concrete());

    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert!(!union.is_concrete());
}

#[test]
fn expr_type_is_list() {
    let list_int = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Int)));
    assert!(list_int.is_list());

    let empty = ExprType::simple(SimpleType::EmptyList);
    assert!(empty.is_list());

    assert!(!ExprType::simple(SimpleType::Int).is_list());

    // Union of lists
    let union = ExprType::sum([
        SimpleType::List(ExprType::simple(SimpleType::Int)),
        SimpleType::EmptyList,
    ])
    .unwrap();
    assert!(union.is_list());

    // Mixed union (not all lists)
    let mixed = ExprType::sum([
        SimpleType::List(ExprType::simple(SimpleType::Int)),
        SimpleType::Int,
    ])
    .unwrap();
    assert!(!mixed.is_list());
}

#[test]
fn expr_type_get_inner_list_type() {
    let list_int = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Int)));
    assert_eq!(
        list_int.get_inner_list_type(),
        Some(ExprType::simple(SimpleType::Int))
    );

    // Union of lists
    let union = ExprType::sum([
        SimpleType::List(ExprType::simple(SimpleType::Int)),
        SimpleType::List(ExprType::simple(SimpleType::Bool)),
    ])
    .unwrap();
    assert_eq!(
        union.get_inner_list_type(),
        ExprType::sum([SimpleType::Int, SimpleType::Bool])
    );

    // Non-list returns None
    assert_eq!(
        ExprType::simple(SimpleType::Int).get_inner_list_type(),
        None
    );
}

#[test]
fn expr_type_is_specific_types() {
    assert!(ExprType::simple(SimpleType::None).is_none());
    assert!(ExprType::simple(SimpleType::Int).is_int());
    assert!(ExprType::simple(SimpleType::Bool).is_bool());
    assert!(ExprType::simple(SimpleType::LinExpr).is_lin_expr());
    assert!(ExprType::simple(SimpleType::Constraint).is_constraint());

    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert!(!union.is_int());
}

#[test]
fn expr_type_is_sum_of_objects() {
    let student = SimpleType::Object("Student".to_string());
    let teacher = SimpleType::Object("Teacher".to_string());

    let single = ExprType::simple(student.clone());
    assert!(single.is_sum_of_objects());

    let union = ExprType::sum([student, teacher]).unwrap();
    assert!(union.is_sum_of_objects());

    let mixed =
        ExprType::sum([SimpleType::Object("Student".to_string()), SimpleType::Int]).unwrap();
    assert!(!mixed.is_sum_of_objects());
}

#[test]
fn expr_type_is_arithmetic() {
    assert!(ExprType::simple(SimpleType::Int).is_arithmetic());
    assert!(ExprType::simple(SimpleType::LinExpr).is_arithmetic());

    let union = ExprType::sum([SimpleType::Int, SimpleType::LinExpr]).unwrap();
    assert!(union.is_arithmetic());

    let mixed = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert!(!mixed.is_arithmetic());
}

#[test]
fn expr_type_contains() {
    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    assert!(union.contains(&SimpleType::Int));
    assert!(union.contains(&SimpleType::Bool));
    assert!(!union.contains(&SimpleType::None));
}

// ========== ExprType::is_subtype_of ==========

#[test]
fn expr_type_is_subtype_of_self() {
    let typ = ExprType::simple(SimpleType::Int);
    assert!(typ.is_subtype_of(&typ));
}

#[test]
fn simple_is_subtype_of_union() {
    let int = ExprType::simple(SimpleType::Int);
    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();

    assert!(int.is_subtype_of(&union));
    assert!(!union.is_subtype_of(&int));
}

#[test]
fn smaller_union_is_subtype_of_larger() {
    let small = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    let large = ExprType::sum([SimpleType::Int, SimpleType::Bool, SimpleType::None]).unwrap();

    assert!(small.is_subtype_of(&large));
    assert!(!large.is_subtype_of(&small));
}

#[test]
fn expr_type_subtyping_respects_simple_subtyping() {
    let empty = ExprType::simple(SimpleType::EmptyList);
    let list_int = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Int)));

    assert!(empty.is_subtype_of(&list_int));
    assert!(!list_int.is_subtype_of(&empty));
}

// ========== ExprType::can_convert_to ==========

#[test]
fn expr_type_can_convert_to_same() {
    let int = ExprType::simple(SimpleType::Int);
    let int_concrete = SimpleType::Int.into_concrete().unwrap();
    assert!(int.can_convert_to(&int_concrete));
}

#[test]
fn expr_type_can_convert_int_to_linexpr() {
    let int = ExprType::simple(SimpleType::Int);
    let linexpr_concrete = SimpleType::LinExpr.into_concrete().unwrap();
    assert!(int.can_convert_to(&linexpr_concrete));
}

#[test]
fn expr_type_cannot_convert_if_one_variant_fails() {
    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    let linexpr_concrete = SimpleType::LinExpr.into_concrete().unwrap();

    // Int can convert but Bool cannot
    assert!(!union.can_convert_to(&linexpr_concrete));
}

#[test]
fn expr_type_all_variants_must_convert() {
    let union = ExprType::sum([SimpleType::Int, SimpleType::EmptyList]).unwrap();

    let list_linexpr_concrete = SimpleType::List(ExprType::simple(SimpleType::LinExpr))
        .into_concrete()
        .unwrap();

    // Neither Int nor EmptyList can convert to [LinExpr] individually
    // (EmptyList can, but Int cannot)
    assert!(!union.can_convert_to(&list_linexpr_concrete));
}

// ========== ExprType::unify_with ==========

#[test]
fn expr_type_unify_with_combines_types() {
    let int = ExprType::simple(SimpleType::Int);
    let bool = ExprType::simple(SimpleType::Bool);

    let unified = int.unify_with(&bool);
    assert!(unified.contains(&SimpleType::Int));
    assert!(unified.contains(&SimpleType::Bool));
}

#[test]
fn expr_type_unify_with_removes_duplicates() {
    let int = ExprType::simple(SimpleType::Int);
    let int2 = ExprType::simple(SimpleType::Int);

    let unified = int.unify_with(&int2);
    assert!(unified.is_simple());
    assert_eq!(unified.as_simple(), Some(&SimpleType::Int));
}

#[test]
fn expr_type_unify_with_removes_subtypes() {
    let empty = ExprType::simple(SimpleType::EmptyList);
    let list_int = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Int)));

    let unified = empty.unify_with(&list_int);
    assert!(unified.is_simple());
    assert_eq!(
        unified.as_simple(),
        Some(&SimpleType::List(ExprType::simple(SimpleType::Int)))
    );
}

// ========== ExprType::overlaps_with ==========

#[test]
fn expr_type_overlaps_with_itself() {
    let typ = ExprType::simple(SimpleType::Int);
    assert!(typ.overlaps_with(&typ));
}

#[test]
fn expr_type_overlaps_when_any_variant_overlaps() {
    let union1 = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    let union2 = ExprType::sum([SimpleType::Bool, SimpleType::None]).unwrap();

    // They overlap because both contain Bool
    assert!(union1.overlaps_with(&union2));
}

#[test]
fn expr_type_doesnt_overlap_if_no_variants_overlap() {
    let int = ExprType::simple(SimpleType::Int);
    let bool = ExprType::simple(SimpleType::Bool);

    assert!(!int.overlaps_with(&bool));
}

#[test]
fn expr_type_lists_always_overlap() {
    let list_int = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Int)));
    let list_bool = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Bool)));

    assert!(list_int.overlaps_with(&list_bool));
}

// ========== ExprType::subtract ==========

#[test]
fn expr_type_subtract_simple_from_union() {
    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool, SimpleType::None]).unwrap();

    let int = ExprType::simple(SimpleType::Int);
    let remaining = union.substract(&int).unwrap();

    assert!(remaining.contains(&SimpleType::Bool));
    assert!(remaining.contains(&SimpleType::None));
    assert!(!remaining.contains(&SimpleType::Int));
}

#[test]
fn expr_type_subtract_removes_subtypes() {
    let union = ExprType::sum([
        SimpleType::List(ExprType::simple(SimpleType::Int)),
        SimpleType::Int,
    ])
    .unwrap();

    let list_int = ExprType::simple(SimpleType::List(ExprType::simple(SimpleType::Int)));

    let remaining = union.substract(&list_int).unwrap();

    // Both [Int] and [] should be removed
    assert!(!remaining.contains(&SimpleType::List(ExprType::simple(SimpleType::Int))));
    assert!(remaining.contains(&SimpleType::Int));
}

#[test]
fn expr_type_subtract_all_returns_none() {
    let int = ExprType::simple(SimpleType::Int);
    let remaining = int.substract(&int);

    assert!(remaining.is_none());
}

#[test]
fn expr_type_subtract_disjoint_types() {
    let int = ExprType::simple(SimpleType::Int);
    let bool = ExprType::simple(SimpleType::Bool);

    let remaining = int.substract(&bool).unwrap();
    assert_eq!(remaining, int);
}

#[test]
fn expr_type_subtract_from_larger_union() {
    let large = ExprType::sum([
        SimpleType::Int,
        SimpleType::Bool,
        SimpleType::None,
        SimpleType::LinExpr,
    ])
    .unwrap();

    let to_remove = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();

    let remaining = large.substract(&to_remove).unwrap();

    assert!(!remaining.contains(&SimpleType::Int));
    assert!(!remaining.contains(&SimpleType::Bool));
    assert!(remaining.contains(&SimpleType::None));
    assert!(remaining.contains(&SimpleType::LinExpr));
}

// ========== ConcreteType ==========

#[test]
fn concrete_type_creation() {
    let concrete = SimpleType::Int.into_concrete().unwrap();
    assert_eq!(concrete.inner(), &SimpleType::Int);
}

#[test]
fn concrete_type_deref() {
    let concrete = SimpleType::Int.into_concrete().unwrap();
    assert!(concrete.is_int()); // Uses Deref to access SimpleType methods
}

#[test]
fn concrete_type_into_inner() {
    let concrete = SimpleType::Int.into_concrete().unwrap();
    let inner = concrete.into_inner();
    assert_eq!(inner, SimpleType::Int);
}

#[test]
fn concrete_type_display() {
    let concrete = SimpleType::Int.into_concrete().unwrap();
    assert_eq!(format!("{}", concrete), "Int");

    let list_concrete = SimpleType::List(ExprType::simple(SimpleType::Int))
        .into_concrete()
        .unwrap();
    assert_eq!(format!("{}", list_concrete), "[Int]");
}

// ========== Display Formatting ==========

#[test]
fn simple_type_display() {
    assert_eq!(format!("{}", SimpleType::Int), "Int");
    assert_eq!(format!("{}", SimpleType::Bool), "Bool");
    assert_eq!(format!("{}", SimpleType::None), "None");
    assert_eq!(format!("{}", SimpleType::LinExpr), "LinExpr");
    assert_eq!(format!("{}", SimpleType::Constraint), "Constraint");
    assert_eq!(format!("{}", SimpleType::EmptyList), "[]");

    let list_int = SimpleType::List(ExprType::simple(SimpleType::Int));
    assert_eq!(format!("{}", list_int), "[Int]");

    let obj = SimpleType::Object("Student".to_string());
    assert_eq!(format!("{}", obj), "Student");
}

#[test]
fn expr_type_display() {
    let int = ExprType::simple(SimpleType::Int);
    assert_eq!(format!("{}", int), "Int");

    let union = ExprType::sum([SimpleType::Int, SimpleType::Bool]).unwrap();
    let display = format!("{}", union);
    // Order might vary due to BTreeSet, so check both contain the types
    assert!(display.contains("Int"));
    assert!(display.contains("Bool"));
    assert!(display.contains(" | "));
}
