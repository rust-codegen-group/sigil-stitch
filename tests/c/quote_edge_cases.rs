use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::c::C;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.c", C::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_pointer_declaration() {
    let block = sigil_quote!(C {
        int* ptr = malloc(sizeof(int));
        const char* msg = "hello";
        void* data = NULL;
    })
    .unwrap();
    golden::assert_golden("c/quote_pointer_decl.c", &render(&block));
}

#[test]
fn test_struct_access() {
    let block = sigil_quote!(C {
        node->next = NULL;
        node->data = value;
        result.x = node->point.x;
    })
    .unwrap();
    golden::assert_golden("c/quote_struct_access.c", &render(&block));
}

#[test]
fn test_preprocessor_define() {
    let block = sigil_quote!(C {
        #define MAX_SIZE 1024
        #define MIN(a, b) ((a) < (b) ? (a) : (b))
    })
    .unwrap();
    golden::assert_golden("c/quote_preprocessor.c", &render(&block));
}

#[test]
fn test_cast_and_sizeof() {
    let block = sigil_quote!(C {
        size_t size = sizeof(struct Node);
        int* arr = (int*)malloc(size * sizeof(int));
    })
    .unwrap();
    golden::assert_golden("c/quote_cast_sizeof.c", &render(&block));
}

#[test]
fn test_for_loop_with_pointer() {
    let block = sigil_quote!(C {
        for (int i = 0; i < n; i++) {
            arr[i] = i * 2;
        }
    })
    .unwrap();
    golden::assert_golden("c/quote_for_loop.c", &render(&block));
}

#[test]
fn test_name_keyword_escape_in_macro() {
    let name = "auto";
    let block = sigil_quote!(C {
        $N(name) = 1;
    })
    .unwrap();

    let output = render(&block);
    assert!(output.contains("auto_ = 1;"), "got: {output}");
}
