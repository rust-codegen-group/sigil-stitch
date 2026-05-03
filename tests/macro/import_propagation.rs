use super::helpers::*;

#[test]
fn test_nested_code_block_import_propagation() {
    let user_type = TypeName::importable_type("./models", "User");
    let inner = CodeBlock::of("getUser(): %T", (user_type,)).unwrap();

    let block = sigil_quote!(TypeScript {
        $C(inner);
    })
    .unwrap();

    let refs = import_collector::collect_imports(&block);
    assert_eq!(refs.len(), 1, "expected 1 import, got: {refs:?}");
    assert_eq!(refs[0].name, "User");
}
