use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// =============================================================================
// ENUM AST BUILDING TESTS
// =============================================================================
// These tests verify that enum declarations and qualified expressions are
// correctly parsed into AST nodes.

// =============================================================================
// ENUM DECLARATION AST
// =============================================================================

#[test]
fn enum_decl_basic() {
    let input = "enum Result = Ok(Int) | Error(String);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    assert_eq!(file.statements.len(), 1);
    match &file.statements[0].node {
        Statement::EnumDecl { name, variants, .. } => {
            assert_eq!(name.node, "Result");
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].node.name.node, "Ok");
            assert_eq!(variants[1].node.name.node, "Error");
        }
        _ => panic!("Expected EnumDecl statement"),
    }
}

#[test]
fn enum_decl_unit_variant() {
    let input = "enum Option = Some(Int) | None;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::EnumDecl { name, variants, .. } => {
            assert_eq!(name.node, "Option");
            assert_eq!(variants.len(), 2);

            // Some(Int) has underlying type
            assert!(variants[0].node.underlying.is_some());

            // None has no underlying type (unit variant)
            assert!(variants[1].node.underlying.is_none());
        }
        _ => panic!("Expected EnumDecl statement"),
    }
}

#[test]
fn enum_decl_tuple_variant() {
    let input = "enum Pair = P(Int, Bool);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::EnumDecl { variants, .. } => {
            assert_eq!(variants.len(), 1);
            let underlying = variants[0].node.underlying.as_ref().unwrap();
            match &underlying.node {
                EnumVariantType::Tuple(types) => {
                    assert_eq!(types.len(), 2);
                }
                _ => panic!("Expected Tuple variant type"),
            }
        }
        _ => panic!("Expected EnumDecl statement"),
    }
}

#[test]
fn enum_decl_struct_variant() {
    let input = "enum Point = P { x: Int, y: Int };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::EnumDecl { variants, .. } => {
            assert_eq!(variants.len(), 1);
            let underlying = variants[0].node.underlying.as_ref().unwrap();
            match &underlying.node {
                EnumVariantType::Struct(fields) => {
                    assert_eq!(fields.len(), 2);
                }
                _ => panic!("Expected Struct variant type"),
            }
        }
        _ => panic!("Expected EnumDecl statement"),
    }
}

#[test]
fn enum_decl_primitive_variant_names() {
    let input = "enum MyType = Int(Int) | Bool(Bool) | None;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::EnumDecl { variants, .. } => {
            assert_eq!(variants.len(), 3);
            assert_eq!(variants[0].node.name.node, "Int");
            assert_eq!(variants[1].node.name.node, "Bool");
            assert_eq!(variants[2].node.name.node, "None");
        }
        _ => panic!("Expected EnumDecl statement"),
    }
}

// =============================================================================
// QUALIFIED TYPE CAST AST
// =============================================================================

#[test]
fn qualified_type_cast_ast() {
    let input = "let f() -> Int = Result::Ok(42);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::GenericCall { path, args } => {
                assert_eq!(path.node.segments.len(), 2);
                assert_eq!(path.node.segments[0].node, "Result");
                assert_eq!(path.node.segments[1].node, "Ok");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected GenericCall, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn qualified_type_cast_unit_variant_no_parens() {
    let input = "let f() -> Int = Option::None;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::IdentPath(path) => {
                assert_eq!(path.node.segments.len(), 2);
                assert_eq!(path.node.segments[0].node, "Option");
                assert_eq!(path.node.segments[1].node, "None");
            }
            _ => panic!("Expected IdentPath, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn qualified_type_cast_unit_variant_empty_parens() {
    let input = "let f() -> Int = Option::None();";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::GenericCall { path, args } => {
                assert_eq!(path.node.segments.len(), 2);
                assert_eq!(path.node.segments[0].node, "Option");
                assert_eq!(path.node.segments[1].node, "None");
                assert!(args.is_empty());
            }
            _ => panic!("Expected GenericCall, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn qualified_type_cast_tuple_variant() {
    let input = "let f() -> Int = Pair::P(1, 2);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::GenericCall { path, args } => {
                assert_eq!(path.node.segments.len(), 2);
                assert_eq!(path.node.segments[0].node, "Pair");
                assert_eq!(path.node.segments[1].node, "P");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected GenericCall, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// QUALIFIED STRUCT CAST AST
// =============================================================================

#[test]
fn qualified_struct_cast_ast() {
    let input = "let f() -> Int = Point::P { x: 1, y: 2 };";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { body, .. } => match &body.node {
            Expr::StructCall { path, fields } => {
                assert_eq!(path.node.segments.len(), 2);
                assert_eq!(path.node.segments[0].node, "Point");
                assert_eq!(path.node.segments[1].node, "P");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0.node, "x");
                assert_eq!(fields[1].0.node, "y");
            }
            _ => panic!("Expected StructCall, got {:?}", body.node),
        },
        _ => panic!("Expected Let statement"),
    }
}

// =============================================================================
// QUALIFIED TYPE IN TYPE ANNOTATIONS
// =============================================================================

#[test]
fn qualified_type_in_return_type() {
    let input = "let f() -> Result::Ok = Result::Ok(42);";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { output_type, .. } => {
            assert_eq!(output_type.node.types.len(), 1);
            match &output_type.node.types[0].node.inner {
                SimpleTypeName::Path(path) => {
                    assert_eq!(path.node.segments.len(), 2);
                    assert_eq!(path.node.segments[0].node, "Result");
                    assert_eq!(path.node.segments[1].node, "Ok");
                }
                _ => panic!("Expected Path type"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn qualified_type_in_param_type() {
    let input = "let f(x: Result::Ok) -> Int = 42;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => {
            assert_eq!(params.len(), 1);
            match &params[0].typ.node.types[0].node.inner {
                SimpleTypeName::Path(path) => {
                    assert_eq!(path.node.segments.len(), 2);
                    assert_eq!(path.node.segments[0].node, "Result");
                    assert_eq!(path.node.segments[1].node, "Ok");
                }
                _ => panic!("Expected Path type in param"),
            }
        }
        _ => panic!("Expected Let statement"),
    }
}
