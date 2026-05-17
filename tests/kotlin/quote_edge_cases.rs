use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.kt", Kotlin::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_safe_call() {
    let block = sigil_quote!(Kotlin {
        val name = response.body?.string();
        val length = name?.length;
    })
    .unwrap();
    golden::assert_golden("kotlin/quote_safe_call.kt", &render(&block));
}

#[test]
fn test_elvis() {
    let block = sigil_quote!(Kotlin {
        val name: String = input ?: "default";
        val count = list?.size ?: 0;
    })
    .unwrap();
    golden::assert_golden("kotlin/quote_elvis.kt", &render(&block));
}

#[test]
fn test_when_expression() {
    let block = sigil_quote!(Kotlin {
        val result = when (x) {
            0 -> $S("zero")
            1 -> $S("one")
            else -> $S("many")
        };
    })
    .unwrap();
    golden::assert_golden("kotlin/quote_when.kt", &render(&block));
}

#[test]
fn test_lambda_with_receiver() {
    let block = sigil_quote!(Kotlin {
        val config = Config().apply {
            host = $S("localhost");
            port = 8080;
        };
    })
    .unwrap();
    golden::assert_golden("kotlin/quote_lambda_receiver.kt", &render(&block));
}

#[test]
fn test_sealed_class() {
    let block = sigil_quote!(Kotlin {
        sealed class Result<out T> {
            data class Success<T>(val value: T) : Result<T>();
            data class Error(val message: String) : Result<Nothing>();
        }
    })
    .unwrap();
    golden::assert_golden("kotlin/quote_sealed_class.kt", &render(&block));
}

#[test]
fn test_extension_function() {
    let block = sigil_quote!(Kotlin {
        fun String.addExclamation(): String {
            return this + $S("!");
        }
    })
    .unwrap();
    golden::assert_golden("kotlin/quote_extension_fun.kt", &render(&block));
}
