use super::parse_one_statement;
use crate::parse::types::{MacroLang, Statement};
use proc_macro2::{TokenStream, TokenTree};

fn parse_stmt(src: &str) -> Statement {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let (stmt, _pos) = parse_one_statement(&tokens, 0, MacroLang::Unaware).unwrap();
    stmt
}

fn parse_stmt_lang(src: &str, lang: MacroLang) -> Statement {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let (stmt, _pos) = parse_one_statement(&tokens, 0, lang).unwrap();
    stmt
}

fn parse_all_stmts(src: &str) -> Vec<Statement> {
    let ts: TokenStream = src.parse().unwrap();
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let mut stmts = Vec::new();
    let mut pos = 0;
    while pos < tokens.len() {
        let (stmt, next) = parse_one_statement(&tokens, pos, MacroLang::Unaware).unwrap();
        stmts.push(stmt);
        pos = next;
    }
    stmts
}

#[test]
fn semicolon_terminated_statement() {
    let stmt = parse_stmt("const x = 42;");
    match stmt {
        Statement::Statement { format, args } => {
            assert_eq!(format, "const x = 42");
            assert!(args.is_empty());
        }
        _ => panic!("expected Statement, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn line_without_semicolon_is_line() {
    let stmts = parse_all_stmts("return x");
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Statement::Line { format, args } => {
            assert_eq!(format, "return x");
            assert!(args.is_empty());
        }
        _ => panic!("expected Line, got {:?}", stmt_kind(&stmts[0])),
    }
}

#[test]
fn multiple_statements() {
    let stmts = parse_all_stmts("let a = 1; let b = 2;");
    assert_eq!(stmts.len(), 2);
    match &stmts[0] {
        Statement::Statement { format, .. } => assert_eq!(format, "let a = 1"),
        _ => panic!("expected Statement"),
    }
    match &stmts[1] {
        Statement::Statement { format, .. } => assert_eq!(format, "let b = 2"),
        _ => panic!("expected Statement"),
    }
}

#[test]
fn control_flow_if_with_body() {
    let stmt = parse_stmt("if (x > 0) { return x; }");
    match stmt {
        Statement::ControlFlow { branches } => {
            assert_eq!(branches.len(), 1);
            assert_eq!(branches[0].condition_format, "if (x > 0)");
            assert_eq!(branches[0].body.len(), 1);
        }
        _ => panic!("expected ControlFlow, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn control_flow_if_else() {
    let stmt = parse_stmt("if (x) { a(); } else { b(); }");
    match stmt {
        Statement::ControlFlow { branches } => {
            assert_eq!(branches.len(), 2);
            assert_eq!(branches[0].condition_format, "if (x)");
            assert_eq!(branches[1].condition_format, "else");
        }
        _ => panic!("expected ControlFlow, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn comment_directive() {
    let stmt = parse_stmt("$comment(\"hello world\")");
    match stmt {
        Statement::Comment(text) => assert_eq!(text, "hello world"),
        _ => panic!("expected Comment, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn indent_directive() {
    let stmts = parse_all_stmts("$>");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::Indent));
}

#[test]
fn dedent_directive() {
    let stmts = parse_all_stmts("$<");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::Dedent));
}

#[test]
fn statement_with_interpolation() {
    let stmt = parse_stmt("const u: $T(user_type) = getUser();");
    match stmt {
        Statement::Statement { format, args } => {
            assert_eq!(format, "const u: %T = getUser()");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("expected Statement"),
    }
}

#[test]
fn go_for_with_embedded_semicolons() {
    let stmt = parse_stmt_lang("for i := 0; i < n; i++ { body(); }", MacroLang::GoLang);
    match stmt {
        Statement::ControlFlow { branches } => {
            assert!(branches[0].condition_format.contains("for"));
            assert!(branches[0].condition_format.contains(";"));
        }
        _ => panic!("expected ControlFlow, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_const_paren_block() {
    let stmt = parse_stmt_lang("const ( x = 1 )", MacroLang::GoLang);
    match stmt {
        Statement::ParenBlock {
            header_format,
            header_args,
            body,
        } => {
            assert_eq!(header_format, "const (");
            assert!(header_args.is_empty());
            assert_eq!(body.len(), 1);
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_var_paren_block() {
    let stmt = parse_stmt_lang("var ( x int )", MacroLang::GoLang);
    match stmt {
        Statement::ParenBlock { header_format, .. } => {
            assert_eq!(header_format, "var (");
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_import_paren_block() {
    let stmt = parse_stmt_lang("import ( \"fmt\" )", MacroLang::GoLang);
    match stmt {
        Statement::ParenBlock { header_format, .. } => {
            assert_eq!(header_format, "import (");
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_type_paren_block() {
    let stmt = parse_stmt_lang("type ( A struct{} )", MacroLang::GoLang);
    match stmt {
        Statement::ParenBlock { header_format, .. } => {
            assert_eq!(header_format, "type (");
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_paren_block_with_metafor() {
    // Verify that $for inside a Go const paren block is parsed recursively.
    let stmt = parse_stmt_lang(
        "const ( $for(v in items) { $L(\"x\") } )",
        MacroLang::GoLang,
    );
    match stmt {
        Statement::ParenBlock { body, .. } => {
            assert_eq!(body.len(), 1);
            assert!(matches!(body[0], Statement::MetaFor { .. }));
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn non_go_paren_block_is_literal() {
    // Without MacroLang::GoLang, `const ( ... )` stays as a literal line.
    let stmt = parse_stmt_lang("const ( x = 1 )", MacroLang::Unaware);
    match stmt {
        Statement::Line { format, .. } => {
            assert!(format.contains("const"));
            assert!(format.contains("("));
        }
        _ => panic!("expected Line, got {:?}", stmt_kind(&stmt)),
    }
}

fn stmt_kind(s: &Statement) -> &'static str {
    match s {
        Statement::Statement { .. } => "Statement",
        Statement::Line { .. } => "Line",
        Statement::BlankLine => "BlankLine",
        Statement::Comment(_) => "Comment",
        Statement::Attr(_) => "Attr",
        Statement::ControlFlow { .. } => "ControlFlow",
        Statement::Indent => "Indent",
        Statement::Dedent => "Dedent",
        Statement::SpliceEach { .. } => "SpliceEach",
        Statement::MetaIf { .. } => "MetaIf",
        Statement::MetaFor { .. } => "MetaFor",
        Statement::MetaLet { .. } => "MetaLet",
        Statement::ParenBlock { .. } => "ParenBlock",
    }
}
