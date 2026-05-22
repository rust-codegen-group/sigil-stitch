use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::prelude::*;
use sigil_stitch::type_name::TypeName;

use crate::shared::LanguageTestSuite;

pub struct TypeScriptSuite;

impl LanguageTestSuite for TypeScriptSuite {
    fn control_flow_block() -> CodeBlock {
        let error_type = TypeName::importable_type("./errors", "NotFoundError");
        sigil_quote!(TypeScript {
            if(!user) {
                throw new $T(error_type)($S("not found"));
            } else {
                return user;
            }
        })
        .unwrap()
    }

    fn control_flow_golden_path() -> &'static str {
        "typescript/macro_control_flow.ts"
    }

    fn basic_block() -> CodeBlock {
        sigil_quote!(TypeScript {
            const name = $S("Alice");
            const age = $L("30");
            console.log(name, age);
        })
        .unwrap()
    }

    fn basic_golden_path() -> &'static str {
        "typescript/macro_basic.ts"
    }

    fn file_spec_name() -> &'static str {
        "test.ts"
    }
}
