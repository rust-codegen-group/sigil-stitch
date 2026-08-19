use super::parse_one_statement;
use crate::ir::{QuoteArg, Statement};
use crate::parse::MacroLang;
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
        Statement::Terminated(formatted) => {
            assert_eq!(formatted.format(), "const x = 42");
            assert!(formatted.args().is_empty());
        }
        _ => panic!("expected Statement, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn line_without_semicolon_is_line() {
    let stmts = parse_all_stmts("return x");
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Statement::Line(formatted) => {
            assert_eq!(formatted.format(), "return x");
            assert!(formatted.args().is_empty());
        }
        _ => panic!("expected Line, got {:?}", stmt_kind(&stmts[0])),
    }
}

#[test]
fn multiple_statements() {
    let stmts = parse_all_stmts("let a = 1; let b = 2;");
    assert_eq!(stmts.len(), 2);
    match &stmts[0] {
        Statement::Terminated(formatted) => assert_eq!(formatted.format(), "let a = 1"),
        _ => panic!("expected Statement"),
    }
    match &stmts[1] {
        Statement::Terminated(formatted) => assert_eq!(formatted.format(), "let b = 2"),
        _ => panic!("expected Statement"),
    }
}

#[test]
fn control_flow_if_with_body() {
    let stmt = parse_stmt("if (x > 0) { return x; }");
    match stmt {
        Statement::ControlFlow { branches, .. } => {
            assert_eq!(branches.len(), 1);
            assert_eq!(branches[0].condition.format(), "if (x > 0)");
            assert_eq!(branches[0].body.len(), 1);
        }
        _ => panic!("expected ControlFlow, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn control_flow_if_else() {
    let stmt = parse_stmt("if (x) { a(); } else { b(); }");
    match stmt {
        Statement::ControlFlow { branches, .. } => {
            assert_eq!(branches.len(), 2);
            assert_eq!(branches[0].condition.format(), "if (x)");
            assert_eq!(branches[1].condition.format(), "else");
        }
        _ => panic!("expected ControlFlow, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn comment_directive() {
    let stmt = parse_stmt("$comment(\"hello world\")");
    match stmt {
        Statement::Comment(_) => {}
        _ => panic!("expected Comment, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn comment_directive_expression() {
    let stmt = parse_stmt("$comment(my_var)");
    match stmt {
        Statement::Comment(_) => {}
        _ => panic!("expected Comment, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn comment_directive_format() {
    let stmt = parse_stmt("$comment(format!(\"hello {}\", name))");
    match stmt {
        Statement::Comment(_) => {}
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
        Statement::Terminated(formatted) => {
            assert_eq!(formatted.format(), "const u: %T = getUser()");
            assert_eq!(formatted.args().len(), 1);
        }
        _ => panic!("expected Statement"),
    }
}

#[test]
fn expression_block_with_meta_statements_is_a_typed_argument() {
    let stmt = parse_stmt("const value = { $let(inner = 1); nested: $L(inner); };");
    let Statement::Terminated(formatted) = stmt else {
        panic!("expected terminated statement");
    };
    assert_eq!(formatted.args().len(), 1);
    assert!(matches!(formatted.args()[0], QuoteArg::ParsedBlock(_)));
}

#[test]
fn newline_before_metafor_without_continuation_still_splits() {
    let stmts = parse_all_stmts("const before = 1\n$for(item in items) { const x = $N(item); }");
    assert_eq!(stmts.len(), 2);
    assert!(matches!(stmts[0], Statement::Line(_)));
    assert!(matches!(stmts[1], Statement::MetaFor { .. }));
}

#[test]
fn go_for_with_embedded_semicolons() {
    let stmt = parse_stmt_lang("for i := 0; i < n; i++ { body(); }", MacroLang::Go);
    match stmt {
        Statement::ControlFlow { branches, .. } => {
            assert!(branches[0].condition.format().contains("for"));
            assert!(branches[0].condition.format().contains(";"));
        }
        _ => panic!("expected ControlFlow, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_const_paren_block() {
    let stmt = parse_stmt_lang("const ( x = 1 )", MacroLang::Go);
    match stmt {
        Statement::ParenBlock { header, body } => {
            assert_eq!(header.format(), "const (");
            assert!(header.args().is_empty());
            assert_eq!(body.len(), 1);
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_var_paren_block() {
    let stmt = parse_stmt_lang("var ( x int )", MacroLang::Go);
    match stmt {
        Statement::ParenBlock { header, .. } => {
            assert_eq!(header.format(), "var (");
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_import_paren_block() {
    let stmt = parse_stmt_lang("import ( \"fmt\" )", MacroLang::Go);
    match stmt {
        Statement::ParenBlock { header, .. } => {
            assert_eq!(header.format(), "import (");
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_type_paren_block() {
    let stmt = parse_stmt_lang("type ( A struct{} )", MacroLang::Go);
    match stmt {
        Statement::ParenBlock { header, .. } => {
            assert_eq!(header.format(), "type (");
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn go_paren_block_with_metafor() {
    // Verify that $for inside a Go const paren block is parsed recursively.
    let stmt = parse_stmt_lang("const ( $for(v in items) { $L(\"x\") } )", MacroLang::Go);
    match stmt {
        Statement::ParenBlock { body, .. } => {
            assert_eq!(body.len(), 1);
            assert!(matches!(body[0], Statement::MetaFor { .. }));
        }
        _ => panic!("expected ParenBlock, got {:?}", stmt_kind(&stmt)),
    }
}

#[test]
fn control_flow_branches_carry_block_intent() {
    use crate::ir::BranchIntent;

    let stmt = parse_stmt("if (x) { a(); } else if (y) { b(); } else { c(); }");
    let Statement::ControlFlow { branches, .. } = stmt else {
        panic!("expected ControlFlow");
    };
    assert_eq!(branches[0].intent, BranchIntent::If);
    assert_eq!(branches[1].intent, BranchIntent::ElseIf);
    assert_eq!(branches[2].intent, BranchIntent::Else);
}

#[test]
fn go_func_and_cpp_lambda_carry_function_intents() {
    use crate::ir::BranchIntent;

    let stmt = parse_stmt_lang("go func() { body(); }", MacroLang::Go);
    let Statement::ControlFlow { branches, .. } = stmt else {
        panic!("expected ControlFlow");
    };
    assert_eq!(branches[0].intent, BranchIntent::Function);

    let stmt = parse_stmt_lang("auto fn = [&](int x) { return x; };", MacroLang::Cpp);
    let Statement::ControlFlow { branches, .. } = stmt else {
        panic!("expected ControlFlow");
    };
    assert_eq!(branches[0].intent, BranchIntent::Lambda);

    let stmt = parse_stmt_lang("if (matrix[0] > 0) { body(); }", MacroLang::Cpp);
    let Statement::ControlFlow { branches, .. } = stmt else {
        panic!("expected ControlFlow");
    };
    assert_eq!(branches[0].intent, BranchIntent::If);

    let stmt = parse_stmt_lang("if fn.String() != \"func\" { body(); }", MacroLang::Go);
    let Statement::ControlFlow { branches, .. } = stmt else {
        panic!("expected ControlFlow");
    };
    assert_eq!(branches[0].intent, BranchIntent::If);
}

#[test]
fn haskell_and_ocaml_infix_headers_carry_block_intent() {
    use crate::ir::BranchIntent;

    let stmt = parse_stmt_lang("main = do { body(); }", MacroLang::Haskell);
    let Statement::ControlFlow { branches, .. } = stmt else {
        panic!("expected ControlFlow");
    };
    assert_eq!(branches[0].intent, BranchIntent::Do);

    let stmt = parse_stmt_lang("let x = match v with { body(); }", MacroLang::OCaml);
    let Statement::ControlFlow { branches, .. } = stmt else {
        panic!("expected ControlFlow");
    };
    assert_eq!(branches[0].intent, BranchIntent::Match);
}

#[test]
fn non_go_paren_block_is_literal() {
    // Without MacroLang::Go, `const ( ... )` stays as a literal line.
    let stmt = parse_stmt_lang("const ( x = 1 )", MacroLang::Unaware);
    match stmt {
        Statement::Line(formatted) => {
            assert!(formatted.format().contains("const"));
            assert!(formatted.format().contains("("));
        }
        _ => panic!("expected Line, got {:?}", stmt_kind(&stmt)),
    }
}

fn stmt_kind(s: &Statement) -> &'static str {
    match s {
        Statement::Terminated(_) => "Statement",
        Statement::Line(_) => "Line",
        Statement::BlankLine => "BlankLine",
        Statement::Comment(_) => "Comment",
        Statement::Attr(_) => "Attr",
        Statement::ControlFlow { .. } => "ControlFlow",
        Statement::Indent => "Indent",
        Statement::Dedent => "Dedent",
        Statement::SpliceEach { .. } => "SpliceEach",
        Statement::MetaIf { .. } => "MetaIf",
        Statement::MetaFor { .. } => "MetaFor",
        Statement::InlineFor { .. } => "InlineFor",
        Statement::MetaLet { .. } => "MetaLet",
        Statement::ParenBlock { .. } => "ParenBlock",
    }
}
