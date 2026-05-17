use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::scala::Scala;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.scala", Scala::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_pattern_match() {
    let block = sigil_quote!(Scala {
        val result = x match {
            case 1 => "one"
            case _ => "other"
        }
    })
    .unwrap();
    golden::assert_golden("scala/quote_pattern_match.scala", &render(&block));
}

#[test]
fn test_implicit_param() {
    let block = sigil_quote!(Scala {
        def execute(query: String)(implicit ctx: ExecutionContext): Future[Result] = {
            Future(runQuery(query));
        }
    })
    .unwrap();
    golden::assert_golden("scala/quote_implicit.scala", &render(&block));
}

#[test]
fn test_for_comprehension() {
    let block = sigil_quote!(Scala {
        val result = for {
            x <- fetchX();
            y <- fetchY(x);
        } yield (x, y);
    })
    .unwrap();
    golden::assert_golden("scala/quote_for_comprehension.scala", &render(&block));
}

#[test]
fn test_case_class() {
    let block = sigil_quote!(Scala {
        case class Person(name: String, age: Int);
        case class Address(street: String, city: String, zip: String);
    })
    .unwrap();
    golden::assert_golden("scala/quote_case_class.scala", &render(&block));
}

#[test]
fn test_trait_mixin() {
    let block = sigil_quote!(Scala {
        trait Serializable {
            def serialize(): String;
        }
        class User(val name: String) extends Entity with Serializable {
            def serialize(): String = name;
        }
    })
    .unwrap();
    golden::assert_golden("scala/quote_trait_mixin.scala", &render(&block));
}
