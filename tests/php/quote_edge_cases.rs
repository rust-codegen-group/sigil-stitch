use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::php::Php;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    FileSpec::builder_with("test.php", Php::new())
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap()
}

#[test]
fn test_indent() {
    let block = sigil_quote!(Php {
        function printDirections() {
        $>
        echo $S("North");
        echo $S("East");
        echo $S("South");
        echo $S("West");
        $<
        }
    })
    .unwrap();
    golden::assert_golden("php/macro_indent.php", &render(&block));
}

#[test]
fn test_name_escape_in_macro() {
    let name = "class";
    let block = sigil_quote!(Php {
        $$obj = new $N(name)();
    })
    .unwrap();

    let output = render(&block);
    assert!(
        output.contains("new class_()"),
        "Expected 'new class_()', got: {output}"
    );
    golden::assert_golden("php/quote_keyword_escape.php", &output);
}

#[test]
fn test_name_no_escape_in_macro() {
    let name = "myHandler";
    let block = sigil_quote!(Php {
        $$obj = new $N(name)();
    })
    .unwrap();

    let output = render(&block);
    assert!(
        output.contains("new myHandler()"),
        "Expected 'new myHandler()', got: {output}"
    );
    golden::assert_golden("php/quote_no_escape.php", &output);
}

#[test]
fn test_class_declaration() {
    let block = sigil_quote!(Php {
        class User {
            private string $$name;
            private int $$age;
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_class.php", &render(&block));
}

#[test]
fn test_interface_declaration() {
    let block = sigil_quote!(Php {
        interface Repository {
            public function findById(string $$id): ?User;
            public function save(User $$entity): void;
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_interface.php", &render(&block));
}

#[test]
fn test_trait_declaration() {
    let block = sigil_quote!(Php {
        trait LoggerTrait {
            public function log(string $$message): void {
                echo $$message;
            }
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_trait.php", &render(&block));
}

#[test]
fn test_enum_declaration() {
    let block = sigil_quote!(Php {
        enum Status: string {
            case Draft = $S("draft");
            case Published = $S("published");
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_enum.php", &render(&block));
}

#[test]
fn test_constructor() {
    let block = sigil_quote!(Php {
        class User {
            public function __construct(
                private string $$name,
                private int $$age,
            ) {}
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_constructor.php", &render(&block));
}

#[test]
fn test_method_call() {
    let block = sigil_quote!(Php {
        $$user = new User();
        $$user->setName($S("Alice"));
    })
    .unwrap();
    golden::assert_golden("php/quote_method_call.php", &render(&block));
}

#[test]
fn test_nullable_type() {
    let block = sigil_quote!(Php {
        function findUser(int $$id): ?User {
            return null;
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_nullable.php", &render(&block));
}

#[test]
fn test_attribute() {
    let block = sigil_quote!(Php {
        $attr("Override")
        public function toString(): string {
            return $$this->name;
        }
    })
    .unwrap();
    golden::assert_golden("php/quote_attribute.php", &render(&block));
}

#[test]
fn test_match_expression() {
    let block = sigil_quote!(Php {
        $$result = match ($$code) {
            200 => $S("OK"),
            404 => $S("Not Found"),
            default => $S("Unknown"),
        };
    })
    .unwrap();
    golden::assert_golden("php/quote_match.php", &render(&block));
}
