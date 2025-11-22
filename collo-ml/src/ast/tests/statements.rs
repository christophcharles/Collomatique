use super::*;
use crate::parser::ColloMLParser;
use pest::Parser;

// ============= Basic Let Statements =============

#[test]
fn parse_simple_let_statement() {
    let input = "let f(x: Int) -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    assert_eq!(file.statements.len(), 1);
    match &file.statements[0].node {
        Statement::Let {
            name,
            params,
            output_type,
            body,
            public,
            docstring,
        } => {
            assert_eq!(name.node, "f");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name.node, "x");
            assert!(matches!(params[0].typ.node, TypeName::Int));
            assert_eq!(output_type.node, TypeName::LinExpr);
            assert!(matches!(body.node, Expr::Number(5)));
            assert!(!public);
            assert!(docstring.is_empty());
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_multiple_params() {
    let input = "let f(x: Int, y: Bool, z: Student) -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => {
            assert_eq!(params.len(), 3);
            assert_eq!(params[0].name.node, "x");
            assert!(matches!(params[0].typ.node, TypeName::Int));
            assert_eq!(params[1].name.node, "y");
            assert!(matches!(params[1].typ.node, TypeName::Bool));
            assert_eq!(params[2].name.node, "z");
            assert!(matches!(params[2].typ.node, TypeName::Object(ref s) if s == "Student"));
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_no_params() {
    let input = "let f() -> LinExpr = 42;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { params, .. } => {
            assert_eq!(params.len(), 0);
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_pub_let_statement() {
    let input = "pub let f() -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { public, .. } => {
            assert!(*public, "Expected public modifier to be true");
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_docstring() {
    let input = "## This is a doc\nlet f() -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { docstring, .. } => {
            assert_eq!(docstring.len(), 1);
            assert_eq!(docstring[0], " This is a doc");
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_let_with_multiple_docstrings() {
    let input = "## This is a doc\n## Second line\nlet f() -> LinExpr = 5;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let { docstring, .. } => {
            assert_eq!(docstring.len(), 2);
            assert_eq!(docstring[0], " This is a doc");
            assert_eq!(docstring[1], " Second line");
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn parse_pub_let_with_docstring() {
    let input = "## Documentation\npub let f() -> Int = 10;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Let {
            public, docstring, ..
        } => {
            assert!(*public);
            assert_eq!(docstring.len(), 1);
            assert_eq!(docstring[0], " Documentation");
        }
        _ => panic!("Expected Let statement"),
    }
}

// ============= Reify Statements =============

#[test]
fn parse_reify_statement() {
    let input = "reify my_constraint as $MyVar;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    assert_eq!(file.statements.len(), 1);
    match &file.statements[0].node {
        Statement::Reify {
            constraint_name,
            name,
            docstring,
            var_list,
        } => {
            assert_eq!(constraint_name.node, "my_constraint");
            assert_eq!(name.node, "MyVar");
            assert!(docstring.is_empty());
            assert!(!*var_list);
        }
        _ => panic!("Expected Reify statement"),
    }
}

#[test]
fn parse_reify_statement_with_var_list() {
    let input = "reify my_constraint as $[MyVarList];";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    assert_eq!(file.statements.len(), 1);
    match &file.statements[0].node {
        Statement::Reify {
            constraint_name,
            name,
            docstring,
            var_list,
        } => {
            assert_eq!(constraint_name.node, "my_constraint");
            assert_eq!(name.node, "MyVarList");
            assert!(docstring.is_empty());
            assert!(*var_list);
        }
        _ => panic!("Expected Reify statement"),
    }
}

#[test]
fn parse_reify_with_docstring() {
    let input = "## Reify this constraint\nreify my_constraint as $MyVar;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Reify { docstring, .. } => {
            assert_eq!(docstring.len(), 1);
            assert_eq!(docstring[0], " Reify this constraint");
        }
        _ => panic!("Expected Reify statement"),
    }
}

#[test]
fn parse_reify_with_multiple_docstrings() {
    let input = "## First line\n## Second line\nreify constraint as $Var;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    match &file.statements[0].node {
        Statement::Reify { docstring, .. } => {
            assert_eq!(docstring.len(), 2);
            assert_eq!(docstring[0], " First line");
            assert_eq!(docstring[1], " Second line");
        }
        _ => panic!("Expected Reify statement"),
    }
}

// ============= Multiple Statements =============

#[test]
fn parse_multiple_statements() {
    let input = "let f() -> Int = 5;\nlet g() -> Bool = true;\nreify c as $V;";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    assert_eq!(file.statements.len(), 3);
    assert!(matches!(file.statements[0].node, Statement::Let { .. }));
    assert!(matches!(file.statements[1].node, Statement::Let { .. }));
    assert!(matches!(file.statements[2].node, Statement::Reify { .. }));
}

#[test]
fn parse_empty_file() {
    let input = "";
    let pairs = ColloMLParser::parse(Rule::file, input).unwrap();
    let file = File::from_pest(pairs.into_iter().next().unwrap()).unwrap();

    assert_eq!(file.statements.len(), 0);
}
